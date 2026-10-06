use crate::discovery::{self, Integration, Report, Sources};
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, path::Path, time::Duration};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    time::{SystemTime, UNIX_EPOCH},
};

const SESSION_LIMIT: usize = 64;
const METADATA_LIMIT: usize = 65_536;
const VARIANTS: [&str; 3] = ["antigravity-cli", "antigravity", "antigravity-ide"];
const TTL_MS: u64 = 30_000;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Hook {
    conversation_id: String,
    workspace_paths: Vec<String>,
    transcript_path: String,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    version: u8,
    id: String,
    cwd: String,
    variant: String,
    pid: u32,
    birth: u64,
    at_ms: u64,
    state: String,
}
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn activity_dir(sources: &Sources) -> std::path::PathBuf {
    sources.antigravity.join("capy-activity")
}
fn event_state(event: &str) -> &'static str {
    match event {
        "PreInvocation" | "PostToolUse" => "working",
        _ => "unknown",
    }
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
    let mut bytes = Vec::new();
    file.take(METADATA_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ())?;
    if bytes.len() > METADATA_LIMIT {
        return Err(());
    }
    serde_json::from_slice(&bytes).map_err(|_| ())
}
fn valid_observation(o: &Observation, now: u64, birth: impl Fn(u32) -> Option<u64>) -> bool {
    o.version == 1
        && discovery::uuid(&o.id)
        && VARIANTS.contains(&o.variant.as_str())
        && Path::new(&o.cwd).is_absolute()
        && birth(o.pid) == Some(o.birth)
        && now.checked_sub(o.at_ms).is_some_and(|age| age <= TTL_MS)
        && matches!(o.state.as_str(), "working" | "unknown")
}
pub fn collect(event: &str) {
    let _ = (|| -> Result<(), ()> {
        if !matches!(event, "PreInvocation" | "PostToolUse" | "Stop") {
            return Err(());
        }
        let mut input = Vec::new();
        std::io::stdin()
            .lock()
            .take(1_048_577)
            .read_to_end(&mut input)
            .map_err(|_| ())?;
        if input.len() > 1_048_576 {
            return Err(());
        }
        let h: Hook = serde_json::from_slice(&input).map_err(|_| ())?;
        if !discovery::uuid(&h.conversation_id)
            || h.workspace_paths.is_empty()
            || h.workspace_paths.len() > 16
        {
            return Err(());
        }
        let cwd = Path::new(&h.workspace_paths[0]);
        if !cwd.is_absolute() || h.workspace_paths[0].len() > 4096 {
            return Err(());
        }
        let sources = Sources::local();
        let dir = activity_dir(&sources);
        if dir.join("disabled").exists() {
            return Ok(());
        }
        let variant = VARIANTS
            .iter()
            .find(|v| {
                Path::new(&h.transcript_path)
                    == sources
                        .antigravity
                        .join(v)
                        .join("brain")
                        .join(&h.conversation_id)
                        .join(".system_generated/logs/transcript.jsonl")
            })
            .ok_or(())?;
        #[cfg(windows)]
        let (pid, birth) = crate::claude_activity::process::ancestor_named(&[
            "agy.exe",
            "Antigravity IDE.exe",
            "language_server_windows_x64.exe",
        ])
        .ok_or(())?;
        #[cfg(not(windows))]
        let (pid, birth) = return Err(());
        fs::create_dir_all(&dir).map_err(|_| ())?;
        if fs::symlink_metadata(&dir)
            .map_err(|_| ())?
            .file_type()
            .is_symlink()
        {
            return Err(());
        }
        let path = dir.join(format!("{}.json", h.conversation_id));
        if path.exists()
            && !fs::symlink_metadata(&path)
                .map_err(|_| ())?
                .file_type()
                .is_file()
        {
            return Err(());
        }
        let mut file = OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(|_| ())?;
        file.try_lock().map_err(|_| ())?;
        let o = Observation {
            version: 1,
            id: h.conversation_id,
            cwd: cwd.to_string_lossy().into(),
            variant: (*variant).into(),
            pid,
            birth,
            at_ms: now_ms(),
            state: event_state(event).into(),
        };
        let bytes = serde_json::to_vec(&o).map_err(|_| ())?;
        if bytes.len() > METADATA_LIMIT {
            return Err(());
        }
        file.set_len(0).map_err(|_| ())?;
        file.write_all(&bytes).map_err(|_| ())
    })();
    println!("{{}}");
}

fn workspace(value: &str) -> Result<String, ()> {
    if value.len() > METADATA_LIMIT {
        return Err(());
    }
    let uris: Vec<String> = serde_json::from_str(value).map_err(|_| ())?;
    if uris.is_empty() || uris.len() > 16 {
        return Err(());
    }
    let uri = url::Url::parse(&uris[0]).map_err(|_| ())?;
    if uri.scheme() != "file"
        || uri
            .host_str()
            .is_some_and(|host| !host.is_empty() && host != "localhost")
    {
        return Err(());
    }
    let path = uri.to_file_path().map_err(|_| ())?;
    if !path.is_absolute() {
        return Err(());
    }
    path.to_str().map(str::to_owned).ok_or(())
}

fn read_metadata(root: &Path, id: &str) -> Result<Option<String>, ()> {
    let connection = Connection::open_with_flags(
        root.join("conversation_summaries.db"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|_| ())?;
    connection.busy_timeout(Duration::ZERO).map_err(|_| ())?;
    let (uris, subagent): (Option<String>, bool) = connection.query_row(
        "SELECT CASE WHEN length(CAST(workspace_uris AS BLOB)) <= 65536 THEN workspace_uris ELSE NULL END, parent_conversation_id <> '' FROM conversation_summaries WHERE conversation_id = ?1",
        [id], |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|_| ())?;
    if subagent {
        return Ok(None);
    }
    workspace(&uris.ok_or(())?).map(Some)
}

pub fn scan(sources: &Sources, report: &mut Report) {
    let mut seen = BTreeSet::new();
    let mut problems = 0;
    let mut inspected = 0;
    let mut present = false;
    for variant in VARIANTS {
        let root = sources.antigravity.join(variant);
        let Ok(entries) = fs::read_dir(root.join("presence")) else {
            continue;
        };
        present = true;
        for entry in entries.take(4096).flatten() {
            let path = entry.path();
            let Some(id) = path
                .file_stem()
                .and_then(|v| v.to_str())
                .filter(|id| discovery::uuid(id))
            else {
                continue;
            };
            if path.extension().and_then(|v| v.to_str()) != Some("lock")
                || !entry.file_type().is_ok_and(|v| v.is_file())
            {
                continue;
            }
            match discovery::held_lock(&path) {
                Ok(false) => continue,
                Err(()) => {
                    problems += 1;
                    continue;
                }
                Ok(true) => {}
            }
            if inspected >= SESSION_LIMIT {
                problems += 1;
                break;
            }
            inspected += 1;
            match read_metadata(&root, id) {
                Ok(Some(cwd))
                    if discovery::held_lock(&path) == Ok(true) && seen.insert(id.to_owned()) =>
                {
                    report.sessions.push(discovery::session(
                        "Antigravity",
                        "antigravity",
                        id,
                        &cwd,
                    ));
                }
                Ok(_) => {}
                Err(()) => problems += 1,
            }
        }
    }
    let dir = activity_dir(sources);
    if !dir.join("disabled").exists() {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.take(128).flatten() {
                let Ok(o) = read_observation(&entry.path()) else {
                    continue;
                };
                if entry.path().file_stem().and_then(|v| v.to_str()) != Some(o.id.as_str())
                    || !valid_observation(&o, now_ms(), discovery::process_birth)
                {
                    continue;
                }
                if o.variant == "antigravity-cli" {
                    let root = sources.antigravity.join(&o.variant);
                    if discovery::held_lock(&root.join("presence").join(format!("{}.lock", o.id)))
                        != Ok(true)
                        || !read_metadata(&root, &o.id)
                            .is_ok_and(|cwd| cwd.as_deref() == Some(o.cwd.as_str()))
                    {
                        continue;
                    }
                }
                present = true;
                let id = format!("antigravity:{}", o.id);
                if let Some(session) = report
                    .sessions
                    .iter_mut()
                    .find(|s| s.id == id && s.origin == o.cwd)
                {
                    session.state = o.state.clone();
                    session.message = hook_message(&o.state).into();
                } else if !seen.contains(&o.id) && seen.len() < SESSION_LIMIT {
                    let mut session =
                        discovery::session("Antigravity", "antigravity", &o.id, &o.cwd);
                    session.state = o.state.clone();
                    session.message = hook_message(&o.state).into();
                    report.sessions.push(session);
                    seen.insert(o.id);
                }
            }
        }
    }
    report.integrations.push(Integration {
        agent: "Antigravity".into(),
        message: if present {
            format!("{} sessões abertas · {problems} registros incompatíveis. Descoberta por bloqueio e projeto; atividade depende dos hooks. Fonte experimental.", seen.len())
        } else {
            "Fonte de presença local não encontrada. Habilitação e validação dos hooks CLI/IDE pendentes.".into()
        },
    });
}
fn hook_message(state: &str) -> &'static str {
    if state == "working" {
        "O Antigravity está trabalhando. Evidência de hook recente (30 s)."
    } else {
        "Sessão observada por hook. O último evento não confirma atividade ou conclusão."
    }
}

#[cfg(test)]
mod tests;
