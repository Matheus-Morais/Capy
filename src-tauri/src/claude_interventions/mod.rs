pub mod protocol;
mod service;
pub use service::{Identity, Service};

use crate::discovery::{process_birth, Sources};
use serde_json::Value;
use std::{fs, io::{Read, Write}, net::TcpStream, path::{Path, PathBuf}, time::Duration};

pub fn default_endpoint() -> PathBuf {
    let root = std::env::var_os("CAPY_CLAUDE_INTERVENTION_ROOT").filter(|v| !v.is_empty()).map(PathBuf::from)
        .or_else(|| std::env::var_os("APPDATA").filter(|v| !v.is_empty()).map(|v| PathBuf::from(v).join("dev.capy.desktop")));
    root.map(|p| p.join("claude-interventions.json")).unwrap_or_default()
}

pub(super) fn safe_path(path: &Path) -> bool {
    path.ancestors().all(|p| match fs::symlink_metadata(p) {
        Ok(m) => {
            #[cfg(windows)]
            { use std::os::windows::fs::MetadataExt; m.file_attributes() & 0x400 == 0 }
            #[cfg(not(windows))]
            { !m.file_type().is_symlink() }
        }
        Err(e) => e.kind() == std::io::ErrorKind::NotFound,
    })
}

#[cfg(windows)]
fn same_image(pid: u32) -> bool {
    use windows_sys::Win32::{Foundation::CloseHandle, System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION}};
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() { return false; }
        let mut path = vec![0u16; 32768]; let mut len = path.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, 0, path.as_mut_ptr(), &mut len) != 0;
        CloseHandle(handle);
        let actual = String::from_utf16(&path[..len as usize]).ok().and_then(|p| fs::canonicalize(p).ok());
        ok && actual.is_some() && actual == std::env::current_exe().ok().and_then(|p| fs::canonicalize(p).ok())
    }
}
#[cfg(not(windows))]
fn same_image(_: u32) -> bool { false }

pub fn collect() {
    let _ = (|| -> Result<(), ()> {
        let activity = Sources::local().claude.join("capy-activity");
        let enabled = activity.join("responses-enabled");
        if activity.join("disabled").exists() || !safe_path(&enabled) || !fs::symlink_metadata(enabled).is_ok_and(|m| m.is_file()) { return Ok(()); }
        let mut bytes = Vec::new();
        std::io::stdin().lock().take(service::FRAME_LIMIT as u64 + 1).read_to_end(&mut bytes).map_err(|_| ())?;
        if bytes.len() > service::FRAME_LIMIT { return Err(()); }
        let input: Value = serde_json::from_slice(&bytes).map_err(|_| ())?;
        protocol::Request::parse(&input).map_err(|_| ())?;
        let hook_pid = std::process::id();
        let hook_birth = process_birth(hook_pid).ok_or(())?;
        let (claude_pid, claude_birth) = crate::claude_activity::hook_ancestor(hook_pid).ok_or(())?;
        let path = default_endpoint();
        if path.as_os_str().is_empty() || !path.is_absolute() || !safe_path(&path) { return Err(()); }
        let endpoint = service::read_endpoint(&path).ok_or(())?;
        if process_birth(endpoint.pid) != Some(endpoint.birth) || !same_image(endpoint.pid) { return Err(()); }
        let mut stream = TcpStream::connect_timeout(&endpoint.addr, Duration::from_millis(300)).map_err(|_| ())?;
        stream.set_write_timeout(Some(Duration::from_millis(300))).map_err(|_| ())?;
        let frame = serde_json::json!({"token":endpoint.token,"identity":Identity{hook_pid,hook_birth,claude_pid,claude_birth},"request":input});
        let mut frame = serde_json::to_vec(&frame).map_err(|_| ())?;
        frame.push(b'\n');
        if frame.len() > service::FRAME_LIMIT { return Err(()); }
        stream.write_all(&frame).map_err(|_| ())?;
        let reply = service::read_frame(&mut stream, Duration::from_secs(110)).map_err(|_| ())?;
        let reply: Value = serde_json::from_slice(&reply).map_err(|_| ())?;
        let mut out = std::io::stdout().lock();
        serde_json::to_writer(&mut out, &reply).map_err(|_| ())?;
        out.write_all(b"\n").map_err(|_| ())
    })();
}

