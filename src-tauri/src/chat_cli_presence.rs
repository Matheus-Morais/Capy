use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const LEASE_MS: u64 = 240_000;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Lease {
    version: u8,
    session_id: String,
    cwd: String,
    pid: u32,
    proc_start: String,
    expires_at: u64,
}

pub fn lease_path(config: &Path, session: &str) -> PathBuf {
    config
        .join("capy-activity")
        .join("chat-leases")
        .join(format!("{session}.json"))
}

fn owned_directories(config: &Path) -> (PathBuf, PathBuf) {
    let activity = config.join("capy-activity");
    let leases = activity.join("chat-leases");
    (activity, leases)
}

fn ensure_directory(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => match fs::create_dir(path) {
            Ok(()) => Ok(()),
            Err(error)
                if error.kind() == std::io::ErrorKind::AlreadyExists
                    && fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_dir()) =>
            {
                Ok(())
            }
            Err(_) => Err("Diretório de registro do chat incompatível.".into()),
        },
        _ => Err("Diretório de registro do chat incompatível.".into()),
    }
}

pub fn write(
    config: &Path,
    session: &str,
    cwd: &Path,
    pid: u32,
    proc_start: u64,
) -> Result<PathBuf, String> {
    if !crate::discovery::uuid(session) || pid == 0 || !cwd.is_absolute() {
        return Err("Identidade do processo de chat inválida.".into());
    }
    let path = lease_path(config, session);
    let (activity, dir) = owned_directories(config);
    ensure_directory(&activity)?;
    ensure_directory(&dir)?;
    if !fs::symlink_metadata(dir).is_ok_and(|m| m.file_type().is_dir())
        || fs::symlink_metadata(&path).is_ok()
    {
        return Err("Registro do processo do chat incompatível; nenhum envio foi feito.".into());
    }
    let expires_at = now_ms().saturating_add(LEASE_MS);
    let lease = Lease {
        version: 1,
        session_id: session.into(),
        cwd: cwd.to_string_lossy().into_owned(),
        pid,
        proc_start: proc_start.to_string(),
        expires_at,
    };
    let bytes = serde_json::to_vec(&lease)
        .map_err(|_| "Não foi possível serializar o registro do chat.")?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|_| "Não foi possível reservar o registro do chat; nenhum envio foi feito.")?;
    use std::io::Write;
    if file.write_all(&bytes).is_err() {
        let _ = fs::remove_file(&path);
        return Err("Não foi possível gravar o registro do chat.".into());
    }
    Ok(path)
}

pub fn remove(path: &Path) {
    let _ = fs::remove_file(path);
}

pub fn valid_for_config(
    path: &Path,
    config: &Path,
    session: &str,
    cwd: &str,
    pid: u32,
    proc_start: u64,
    birth: impl Fn(u32) -> Option<u64>,
) -> bool {
    if path != lease_path(config, session) {
        return false;
    }
    let (activity, leases) = owned_directories(config);
    if !fs::symlink_metadata(&activity).is_ok_and(|m| m.file_type().is_dir())
        || !fs::symlink_metadata(&leases).is_ok_and(|m| m.file_type().is_dir())
    {
        return false;
    }
    for _ in 0..100 {
        if valid(path, session, cwd, pid, proc_start, now_ms(), &birth) {
            return true;
        }
        if !path.exists() {
            std::thread::sleep(std::time::Duration::from_millis(25));
        } else {
            return false;
        }
    }
    false
}

fn valid(
    path: &Path,
    session: &str,
    cwd: &str,
    pid: u32,
    proc_start: u64,
    now: u64,
    birth: impl Fn(u32) -> Option<u64>,
) -> bool {
    let Some(parent) = path.parent() else {
        return false;
    };
    if !fs::symlink_metadata(parent).is_ok_and(|m| m.file_type().is_dir())
        || !fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_file() && m.len() <= 8192)
    {
        return false;
    }
    let Ok(bytes) = fs::read(path) else {
        return false;
    };
    let Ok(lease) = serde_json::from_slice::<Lease>(&bytes) else {
        return false;
    };
    lease.version == 1
        && lease.session_id == session
        && lease.cwd == cwd
        && lease.pid == pid
        && lease.proc_start == proc_start.to_string()
        && lease.expires_at >= now
        && lease.expires_at.saturating_sub(now) <= LEASE_MS
        && birth(pid) == Some(proc_start)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lease_requires_exact_session_cwd_pid_birth_and_fresh_expiry() {
        let root = std::env::temp_dir().join(format!("capy-chat-lease-{}", uuid::Uuid::new_v4()));
        let session = uuid::Uuid::new_v4().to_string();
        let cwd = root.join("workspace");
        fs::create_dir_all(&cwd).unwrap();
        let pid = 231;
        let birth = 9182;
        let now = now_ms();
        let path = lease_path(&root, &session);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let lease = Lease {
            version: 1,
            session_id: session.clone(),
            cwd: cwd.to_string_lossy().into_owned(),
            pid,
            proc_start: birth.to_string(),
            expires_at: now.saturating_add(100_000),
        };
        fs::write(&path, serde_json::to_vec(&lease).unwrap()).unwrap();
        let actual = cwd.to_string_lossy();
        assert!(valid_for_config(
            &path,
            &root,
            &session,
            &actual,
            pid,
            birth,
            |id| { (id == pid).then_some(birth) }
        ));
        assert!(!valid_for_config(
            &path.with_file_name("other.json"),
            &root,
            &session,
            &actual,
            pid,
            birth,
            |_| Some(birth)
        ));
        let matches = |s: &str, c: &str, p, b, now| {
            valid(&path, s, c, p, b, now, |id| (id == pid).then_some(birth))
        };
        assert!(matches(&session, &actual, pid, birth, now));
        assert!(!matches(
            &uuid::Uuid::new_v4().to_string(),
            &actual,
            pid,
            birth,
            now
        ));
        assert!(!matches(&session, "C:/elsewhere", pid, birth, now));
        assert!(!matches(&session, &actual, pid + 1, birth, now));
        assert!(!valid(
            &path,
            &session,
            &actual,
            pid,
            birth + 1,
            now,
            |_| Some(birth)
        ));
        assert!(!matches(
            &session,
            &actual,
            pid,
            birth,
            now.saturating_add(100_001)
        ));
        fs::remove_dir_all(root).unwrap();
    }
}
