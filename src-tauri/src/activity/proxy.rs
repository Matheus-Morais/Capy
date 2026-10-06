use super::{protocol, Observation};
use serde::Deserialize;
use std::os::windows::process::CommandExt;
use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::mpsc,
    time::Duration,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Daemon {
    pid: u32,
    process_start_time: String,
}

fn executable(home: &Path) -> Result<PathBuf, ()> {
    let mut bytes = Vec::new();
    File::open(home.join("app-server-daemon/daemon.pid"))
        .map_err(|_| ())?
        .take(65_537)
        .read_to_end(&mut bytes)
        .map_err(|_| ())?;
    if bytes.len() > 65_536 {
        return Err(());
    }
    let daemon: Daemon = serde_json::from_slice(&bytes).map_err(|_| ())?;
    let expected = daemon.process_start_time.parse::<u64>().map_err(|_| ())?;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, FILETIME, STILL_ACTIVE},
        System::Threading::{
            GetExitCodeProcess, GetProcessTimes, OpenProcess, QueryFullProcessImageNameW,
            PROCESS_QUERY_LIMITED_INFORMATION,
        },
    };
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, daemon.pid);
        if handle.is_null() {
            return Err(());
        }
        let mut birth: FILETIME = std::mem::zeroed();
        let mut exit = std::mem::zeroed();
        let mut kernel = std::mem::zeroed();
        let mut user = std::mem::zeroed();
        let mut code = 0;
        let mut path = vec![0u16; 32_768];
        let mut len = path.len() as u32;
        let ok = GetProcessTimes(handle, &mut birth, &mut exit, &mut kernel, &mut user) != 0
            && ((u64::from(birth.dwHighDateTime) << 32) | u64::from(birth.dwLowDateTime))
                == expected
            && GetExitCodeProcess(handle, &mut code) != 0
            && code == STILL_ACTIVE as u32
            && QueryFullProcessImageNameW(handle, 0, path.as_mut_ptr(), &mut len) != 0;
        CloseHandle(handle);
        if !ok {
            return Err(());
        }
        let path = PathBuf::from(String::from_utf16(&path[..len as usize]).map_err(|_| ())?);
        if !path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("codex.exe"))
        {
            return Err(());
        }
        Ok(path)
    }
}
struct Pipes {
    input: ChildStdin,
    output: ChildStdout,
}
impl Read for Pipes {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        self.output.read(bytes)
    }
}
impl Write for Pipes {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.input.write(bytes)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.input.flush()
    }
}
pub(super) fn query_child(
    child: &mut Child,
    ids: Vec<String>,
    timeout: Duration,
) -> Result<Vec<Observation>, ()> {
    query_task(child, timeout, move |pipes| protocol::observe(pipes, &ids))
}
fn query_task<R: Send + 'static>(
    child: &mut Child,
    timeout: Duration,
    task: impl FnOnce(Pipes) -> Result<R, ()> + Send + 'static,
) -> Result<R, ()> {
    let result = match (child.stdin.take(), child.stdout.take()) {
        (Some(input), Some(output)) => {
            let (sender, receiver) = mpsc::sync_channel(1);
            std::thread::spawn(move || {
                let _ = sender.send(task(Pipes { input, output }));
            });
            receiver.recv_timeout(timeout).unwrap_or(Err(()))
        }
        _ => Err(()),
    };
    let _ = child.kill();
    child.wait().map_err(|_| ())?;
    result
}
pub(super) fn query(home: &Path, ids: Vec<String>) -> Result<Vec<Observation>, ()> {
    let mut child = spawn_proxy(home)?;
    query_child(&mut child, ids, Duration::from_secs(5))
}
pub(crate) fn quota(home: &Path) -> Result<crate::quotas::RawSample, ()> {
    let mut child = spawn_proxy(home)?;
    query_task(&mut child, Duration::from_secs(8), protocol::quota)
}
pub(crate) fn spawn_proxy(home: &Path) -> Result<Child, ()> {
    let exe = executable(home)?;
    Command::new(exe)
        .args(["app-server", "proxy", "--sock"])
        .arg(home.join("app-server-control/app-server-control.sock"))
        .creation_flags(0x08000000)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| ())
}
