use crate::discovery::{self, Integration, Report, Sources};
use rusqlite::{Connection, OpenFlags};
use std::{collections::BTreeSet, fs, path::Path, time::Duration};

const SESSION_LIMIT: usize = 64;
const METADATA_LIMIT: usize = 65_536;
const VARIANTS: [&str; 3] = ["antigravity-cli", "antigravity", "antigravity-ide"];

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
    report.integrations.push(Integration {
        agent: "Antigravity".into(),
        message: if present {
            format!("{} sessões abertas · {problems} registros incompatíveis. Descoberta por bloqueio e projeto; atividade depende dos hooks. Fonte experimental.", seen.len())
        } else {
            "Fonte de presença local não encontrada. Habilitação e validação dos hooks CLI/IDE pendentes.".into()
        },
    });
}

#[cfg(test)]
mod tests;
