use crate::discovery::{self, Report, Sources};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const INPUT_LIMIT: u64 = 1_048_576;
const RECORD_LIMIT: u64 = 65_536;
const TTL_MS: u64 = 30_000;
const SESSION_LIMIT: usize = 64;

#[derive(Deserialize)]
struct Hook {
    session_id: String,
    cwd: String,
    hook_event_name: String,
    agent_id: Option<String>,
    notification_type: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Presence {
    pid: u32,
    session_id: String,
    cwd: String,
    proc_start: String,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    version: u8,
    session_id: String,
    cwd: String,
    pid: u32,
    proc_start: String,
    at_ms: u64,
    state: String,
}
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn uuid(s: &str) -> bool {
    s.len() == 36
        && s.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}
fn directory(sources: &Sources) -> PathBuf {
    sources.claude.join("capy-activity")
}
fn read_json<T: serde::de::DeserializeOwned>(reader: impl Read, limit: u64) -> Result<T, ()> {
    let mut bytes = Vec::new();
    reader
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ())?;
    if bytes.len() as u64 > limit {
        return Err(());
    }
    serde_json::from_slice(&bytes).map_err(|_| ())
}
fn event_state(h: &Hook) -> Option<&'static str> {
    if h.agent_id.is_some() {
        return None;
    }
    Some(match h.hook_event_name.as_str() {
        "UserPromptSubmit" | "PreToolUse" | "PostToolUse" | "PostToolUseFailure"
        | "PostToolBatch" | "PermissionDenied" => "working",
        "PermissionRequest" => "waiting",
        "Notification" => match h.notification_type.as_deref() {
            Some("permission_prompt") => "waiting",
            Some("idle_prompt") => "idle",
            _ => "unknown",
        },
        _ => "unknown",
    })
}
fn matches(o: &Observation, p: &Presence) -> bool {
    o.version == 1
        && o.session_id == p.session_id
        && o.cwd == p.cwd
        && o.pid == p.pid
        && o.proc_start == p.proc_start
}
fn read_observation(path: &Path) -> Result<Observation, ()> {
    if !fs::symlink_metadata(path)
        .map_err(|_| ())?
        .file_type()
        .is_file()
    {
        return Err(());
    }
    let file = File::open(path).map_err(|_| ())?;
    file.try_lock_shared().map_err(|_| ())?;
    read_json(file, RECORD_LIMIT)
}
fn write_observation(path: &Path, o: &Observation) -> Result<(), ()> {
    if path.exists()
        && !fs::symlink_metadata(path)
            .map_err(|_| ())?
            .file_type()
            .is_file()
    {
        return Err(());
    }
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)
        .map_err(|_| ())?;
    file.try_lock().map_err(|_| ())?;
    if let Ok(previous) = read_json::<Observation>(&mut file, RECORD_LIMIT) {
        if previous.pid == o.pid && previous.proc_start == o.proc_start && previous.at_ms >= o.at_ms
        {
            return Ok(());
        }
    }
    let bytes = serde_json::to_vec(o).map_err(|_| ())?;
    if bytes.len() as u64 > RECORD_LIMIT {
        return Err(());
    }
    file.seek(SeekFrom::Start(0)).map_err(|_| ())?;
    file.set_len(0).map_err(|_| ())?;
    file.write_all(&bytes).map_err(|_| ())
}
fn cleanup(dir: &Path, birth: impl Fn(u32) -> Option<u64>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.take(128).flatten() {
        if !entry.file_type().is_ok_and(|t| t.is_file()) {
            continue;
        }
        let Ok(o) = read_observation(&entry.path()) else {
            continue;
        };
        if o.proc_start
            .parse::<u64>()
            .ok()
            .is_none_or(|start| birth(o.pid) != Some(start))
        {
            let _ = fs::remove_file(entry.path());
        }
    }
}

pub fn collect() {
    // Hook failures never influence Claude's permission flow and never expose its input.
    let at_ms = now_ms();
    let _ = (|| -> Result<(), ()> {
        let h: Hook = read_json(std::io::stdin().lock(), INPUT_LIMIT)?;
        let Some(state) = event_state(&h) else {
            return Ok(());
        };
        if !uuid(&h.session_id) || h.cwd.is_empty() {
            return Err(());
        }
        let sources = Sources::local();
        if directory(&sources).join("disabled").exists() {
            return Ok(());
        }
        let (pid, start) = process::claude_ancestor().ok_or(())?;
        let p: Presence = read_json(
            File::open(sources.claude.join("sessions").join(format!("{pid}.json")))
                .map_err(|_| ())?,
            RECORD_LIMIT,
        )?;
        if p.pid != pid
            || p.session_id != h.session_id
            || p.cwd != h.cwd
            || p.proc_start != start.to_string()
        {
            return Err(());
        }
        let dir = directory(&sources);
        fs::create_dir_all(&dir).map_err(|_| ())?;
        if fs::symlink_metadata(&dir)
            .map_err(|_| ())?
            .file_type()
            .is_symlink()
        {
            return Err(());
        }
        cleanup(&dir, discovery::process_birth);
        write_observation(
            &dir.join(format!("{}.json", h.session_id)),
            &Observation {
                version: 1,
                session_id: h.session_id,
                cwd: h.cwd,
                pid,
                proc_start: p.proc_start,
                at_ms,
                state: state.into(),
            },
        )
    })();
}

pub fn enrich(sources: &Sources, report: &mut Report) {
    enrich_with(sources, report, now_ms(), discovery::process_birth);
}
fn enrich_with(
    sources: &Sources,
    report: &mut Report,
    now: u64,
    birth: impl Fn(u32) -> Option<u64>,
) {
    let mut confirmed = 0;
    for (index, s) in report
        .sessions
        .iter_mut()
        .filter(|s| s.kind == "claude")
        .enumerate()
    {
        s.state = "unknown".into();
        s.request = None;
        s.command = None;
        s.message = "Sessão aberta. Atividade desconhecida: hooks ausentes, evidência antiga ou incompatível.".into();
        let Some(id) = s.id.strip_prefix("claude:").filter(|id| uuid(id)) else {
            continue;
        };
        if index >= SESSION_LIMIT || directory(sources).join("disabled").exists() {
            continue;
        }
        let Ok(o) = read_observation(&directory(sources).join(format!("{id}.json"))) else {
            continue;
        };
        if o.session_id != id
            || o.cwd != s.origin
            || !matches!(o.state.as_str(), "working" | "waiting" | "idle" | "unknown")
        {
            continue;
        }
        let Ok(p) = File::open(
            sources
                .claude
                .join("sessions")
                .join(format!("{}.json", o.pid)),
        )
        .map_err(|_| ())
        .and_then(|f| read_json::<Presence>(f, RECORD_LIMIT)) else {
            continue;
        };
        let valid = matches(&o, &p)
            && o.proc_start
                .parse::<u64>()
                .ok()
                .is_some_and(|start| birth(o.pid) == Some(start))
            && now.checked_sub(o.at_ms).is_some_and(|age| age <= TTL_MS);
        if !valid {
            continue;
        }
        s.state = o.state;
        s.message = match s.state.as_str() {
            "working" => "O Claude Code está trabalhando nesta sessão.",
            "waiting" => "O Claude Code aguarda uma permissão. Responda no Claude Code.",
            "idle" => "Sessão ociosa. Isso não confirma conclusão da tarefa.",
            _ => "Sessão aberta. O último evento não confirma a atividade atual.",
        }
        .into();
        confirmed += usize::from(s.state != "unknown");
    }
    if let Some(i) = report
        .integrations
        .iter_mut()
        .find(|i| i.agent == "Claude Code")
    {
        i.message.push_str(&format!(" Atividade por hooks: {confirmed} com evidência recente (30 s). Respostas no Claude Code."));
    }
}
#[cfg(windows)]
mod process;
#[cfg(not(windows))]
mod process {
    pub fn claude_ancestor() -> Option<(u32, u64)> {
        None
    }
}
#[cfg(test)]
mod tests;
