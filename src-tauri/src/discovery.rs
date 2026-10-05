use crate::demo::Session;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashMap},
    fs::{self, File},
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
};

const METADATA_LIMIT: u64 = 65_536;

#[derive(Clone, Serialize, Default)]
pub struct Report {
    pub sessions: Vec<Session>,
    pub integrations: Vec<Integration>,
}

#[derive(Clone, Serialize, PartialEq)]
pub struct Integration {
    pub agent: String,
    pub message: String,
}

pub struct Sources {
    pub claude: PathBuf,
    pub codex: PathBuf,
}

impl Sources {
    pub fn local() -> Self {
        let home = std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .unwrap_or_default();
        Self {
            claude: std::env::var_os("CLAUDE_CONFIG_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".claude")),
            codex: std::env::var_os("CODEX_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".codex")),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClaudeRecord {
    pid: u32,
    session_id: String,
    cwd: String,
    proc_start: String,
}

#[derive(Deserialize)]
struct CodexRecord {
    r#type: String,
    payload: CodexMetadata,
}
#[derive(Deserialize)]
struct CodexMetadata {
    id: String,
    cwd: String,
    source: serde_json::Value,
}

fn uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}

fn session(agent: &str, kind: &str, id: &str, cwd: &str) -> Session {
    let project = cwd
        .trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .filter(|p| !p.is_empty())
        .unwrap_or(cwd);
    Session {
        id: format!("{kind}:{id}"),
        project: project.into(),
        agent: agent.into(),
        symbol: if kind == "claude" { "C" } else { "O" }.into(),
        kind: kind.into(),
        origin: cwd.into(),
        state: "unknown".into(),
        request: None,
        message: "Sessão aberta. Atividade e pedidos ainda não são observados.".into(),
        command: None,
        hidden: false,
    }
}

fn small_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, ()> {
    let file = File::open(path).map_err(|_| ())?;
    let mut bytes = Vec::new();
    file.take(METADATA_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ())?;
    if bytes.len() as u64 > METADATA_LIMIT {
        return Err(());
    }
    serde_json::from_slice(&bytes).map_err(|_| ())
}

#[cfg(windows)]
pub fn process_birth(pid: u32) -> Option<u64> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, FILETIME, STILL_ACTIVE},
        System::Threading::{
            GetExitCodeProcess, GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        },
    };
    // Only query the existing process; no handle with mutation rights is requested.
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return None;
        }
        let mut birth: FILETIME = std::mem::zeroed();
        let mut exit: FILETIME = std::mem::zeroed();
        let mut kernel: FILETIME = std::mem::zeroed();
        let mut user: FILETIME = std::mem::zeroed();
        let mut code = 0;
        let ok = GetProcessTimes(handle, &mut birth, &mut exit, &mut kernel, &mut user) != 0
            && GetExitCodeProcess(handle, &mut code) != 0
            && code == STILL_ACTIVE as u32;
        CloseHandle(handle);
        ok.then_some((u64::from(birth.dwHighDateTime) << 32) | u64::from(birth.dwLowDateTime))
    }
}
#[cfg(not(windows))]
pub fn process_birth(_: u32) -> Option<u64> {
    None
}

fn held_lock(path: &Path) -> Result<bool, ()> {
    let file = File::open(path).map_err(|_| ())?;
    match file.try_lock_shared() {
        Ok(()) => Ok(false),
        Err(std::fs::TryLockError::WouldBlock) => Ok(true),
        Err(_) => Err(()),
    }
}

fn diagnostic(agent: &str, count: usize, problems: usize, present: bool) -> Integration {
    let sessions = if count == 1 {
        "1 sessão aberta".into()
    } else {
        format!("{count} sessões abertas")
    };
    Integration {
        agent: agent.into(),
        message: if !present {
            "Fonte local não encontrada.".into()
        } else if problems > 0 {
            format!(
                "{sessions} · {problems} registros ilegíveis ou incompatíveis. Fonte experimental."
            )
        } else {
            format!("{sessions} · descoberta local experimental.")
        },
    }
}

pub fn scan(sources: &Sources) -> Report {
    scan_with(sources, process_birth)
}

fn scan_with(sources: &Sources, birth: impl Fn(u32) -> Option<u64>) -> Report {
    let mut report = Report::default();
    let claude_dir = sources.claude.join("sessions");
    let mut problems = 0;
    let mut found = HashMap::new();
    match fs::read_dir(&claude_dir) {
        Ok(entries) => {
            for entry in entries {
                let Ok(entry) = entry else {
                    problems += 1;
                    continue;
                };
                if entry.path().extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                if !entry.file_type().is_ok_and(|t| t.is_file()) {
                    problems += 1;
                    continue;
                }
                let Ok(record) = small_json::<ClaudeRecord>(&entry.path()) else {
                    problems += 1;
                    continue;
                };
                if !uuid(&record.session_id) || record.cwd.is_empty() {
                    problems += 1;
                    continue;
                }
                let Ok(expected) = record.proc_start.parse::<u64>() else {
                    problems += 1;
                    continue;
                };
                if birth(record.pid) == Some(expected) {
                    found.insert(
                        record.session_id.clone(),
                        session("Claude Code", "claude", &record.session_id, &record.cwd),
                    );
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => problems += 1,
    }
    report.integrations.push(diagnostic(
        "Claude Code",
        found.len(),
        problems,
        claude_dir.exists(),
    ));
    report.sessions.extend(found.into_values());

    let lock_dir = sources.codex.join("thread-writer-locks");
    let mut active = BTreeSet::new();
    problems = 0;
    match fs::read_dir(&lock_dir) {
        Ok(entries) => {
            for entry in entries {
                let Ok(entry) = entry else {
                    problems += 1;
                    continue;
                };
                let path = entry.path();
                let Some(id) = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .filter(|s| uuid(s))
                else {
                    continue;
                };
                if path.extension().and_then(|s| s.to_str()) != Some("lock") {
                    continue;
                }
                if !entry.file_type().is_ok_and(|t| t.is_file()) {
                    problems += 1;
                    continue;
                }
                match held_lock(&path) {
                    Ok(true) => {
                        active.insert(id.to_owned());
                    }
                    Ok(false) => {}
                    Err(()) => problems += 1,
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => problems += 1,
    }
    let mut found = HashMap::new();
    if !active.is_empty() {
        let mut budget = 20_000;
        let mut resolved = BTreeSet::new();
        walk_metadata(
            &sources.codex.join("sessions"),
            5,
            &mut budget,
            &active,
            &mut found,
            &mut problems,
            &mut resolved,
        );
        problems += active
            .iter()
            .filter(|id| {
                !resolved.contains(*id)
                    && held_lock(&lock_dir.join(format!("{id}.lock"))) == Ok(true)
            })
            .count();
    }
    // A held writer can belong to a subagent, which is deliberately excluded here.
    report.integrations.push(diagnostic(
        "Codex",
        found.len(),
        problems,
        lock_dir.exists(),
    ));
    report.sessions.extend(found.into_values());
    report.integrations.push(Integration {
        agent: "Antigravity".into(),
        message:
            "Integração pendente: presença e projeto ainda precisam de validação na CLI e IDE."
                .into(),
    });
    report.sessions.sort_by(|a, b| a.id.cmp(&b.id));
    report
}

fn walk_metadata(
    dir: &Path,
    depth: usize,
    budget: &mut usize,
    active: &BTreeSet<String>,
    found: &mut HashMap<String, Session>,
    problems: &mut usize,
    resolved: &mut BTreeSet<String>,
) {
    let Ok(entries) = fs::read_dir(dir) else {
        *problems += 1;
        return;
    };
    for entry in entries {
        if *budget == 0 {
            *problems += 1;
            return;
        }
        *budget -= 1;
        let Ok(entry) = entry else {
            *problems += 1;
            continue;
        };
        let Ok(kind) = entry.file_type() else {
            *problems += 1;
            continue;
        };
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            if depth > 0 {
                walk_metadata(
                    &entry.path(),
                    depth - 1,
                    budget,
                    active,
                    found,
                    problems,
                    resolved,
                );
            }
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let Some(id) = active
            .iter()
            .find(|id| name.ends_with(&format!("-{id}.jsonl")))
        else {
            continue;
        };
        let Ok(file) = File::open(entry.path()) else {
            resolved.insert(id.clone());
            *problems += 1;
            continue;
        };
        let mut line = String::new();
        resolved.insert(id.clone());
        if BufReader::new(file.take(METADATA_LIMIT))
            .read_line(&mut line)
            .is_err()
            || !line.ends_with('\n')
        {
            *problems += 1;
            continue;
        }
        let Ok(record) = serde_json::from_str::<CodexRecord>(&line) else {
            *problems += 1;
            continue;
        };
        if record.r#type != "session_meta"
            || record.payload.id != *id
            || record.payload.cwd.is_empty()
        {
            *problems += 1;
            continue;
        }
        if !record
            .payload
            .source
            .as_str()
            .is_some_and(|s| ["cli", "vscode", "appServer", "app-server"].contains(&s))
        {
            if !record
                .payload
                .source
                .as_object()
                .is_some_and(|s| s.contains_key("subagent"))
            {
                *problems += 1;
            }
            continue;
        }
        // Recheck presence after reading metadata in case the owner exited during the scan.
        let lock = dir
            .ancestors()
            .find(|p| p.file_name().is_some_and(|n| n == "sessions"))
            .and_then(Path::parent)
            .map(|p| p.join("thread-writer-locks").join(format!("{id}.lock")));
        if lock.as_ref().is_some_and(|p| held_lock(p) == Ok(true)) {
            found.insert(
                id.clone(),
                session("Codex", "codex", id, &record.payload.cwd),
            );
        }
    }
}

#[derive(Default, Clone)]
pub struct Preferences {
    pub hidden: BTreeSet<String>,
    pub path: Option<PathBuf>,
}
impl Preferences {
    pub fn load(path: PathBuf) -> Self {
        let hidden = small_json(&path).unwrap_or_default();
        Self {
            hidden,
            path: Some(path),
        }
    }
    pub fn save(&self) -> Result<(), String> {
        let path = self
            .path
            .as_ref()
            .ok_or("Preferências ainda não estão prontas")?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let bytes = serde_json::to_vec(&self.hidden).map_err(|e| e.to_string())?;
        fs::write(path, bytes).map_err(|e| e.to_string())
    }
    pub fn apply(&self, sessions: &mut [Session]) {
        for s in sessions {
            s.hidden = self.hidden.contains(&s.id);
        }
    }
}

#[cfg(test)]
mod tests;
