use super::{protocol::Request, safe_path};
use crate::interventions::registry::{Context, View};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub(super) const FRAME_LIMIT: usize = 65_536;
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub hook_pid: u32,
    pub hook_birth: u64,
    pub claude_pid: u32,
    pub claude_birth: u64,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Endpoint {
    pub addr: SocketAddr,
    pub token: String,
    pub pid: u32,
    pub birth: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Frame {
    token: String,
    identity: Identity,
    request: Value,
}
type Validator = dyn Fn(&Identity, &Request) -> bool + Send + Sync;
struct Pending {
    identity: Identity,
    request: Request,
    view: View,
    stream: TcpStream,
    expires: Instant,
}
struct State {
    generation: String,
    pending: HashMap<String, Pending>,
    connections: HashMap<String, TcpStream>,
}
struct Shared {
    state: Mutex<State>,
    validate: Box<Validator>,
    stopped: AtomicBool,
    active: AtomicUsize,
    timeout: Duration,
}
pub struct Service {
    shared: Arc<Shared>,
    endpoint_path: PathBuf,
    token: String,
    listener: Mutex<Option<JoinHandle<()>>>,
    endpoint_lock: Mutex<Option<File>>,
}

pub(super) fn read_endpoint(path: &Path) -> Option<Endpoint> {
    if !safe_path(path) || !fs::symlink_metadata(path).ok()?.is_file() {
        return None;
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .ok()?
        .take(4097)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() > 4096 {
        return None;
    }
    let endpoint: Endpoint = serde_json::from_slice(&bytes).ok()?;
    (endpoint.addr.ip() == IpAddr::V4(Ipv4Addr::LOCALHOST)
        && endpoint.addr.port() != 0
        && uuid::Uuid::parse_str(&endpoint.token).is_ok())
    .then_some(endpoint)
}
pub(super) fn read_frame(stream: &mut TcpStream, timeout: Duration) -> std::io::Result<Vec<u8>> {
    let deadline = Instant::now() + timeout;
    let mut bytes = Vec::new();
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or(std::io::ErrorKind::TimedOut)?;
        stream.set_read_timeout(Some(remaining))?;
        let mut byte = [0];
        stream.read_exact(&mut byte)?;
        if byte[0] == b'\n' {
            return Ok(bytes);
        }
        bytes.push(byte[0]);
        if bytes.len() >= FRAME_LIMIT {
            return Err(std::io::ErrorKind::InvalidData.into());
        }
    }
}
fn clear(state: &mut State) {
    for (_, stream) in state.connections.drain() {
        let _ = stream.shutdown(Shutdown::Both);
    }
    for (_, entry) in state.pending.drain() {
        let _ = entry.stream.shutdown(Shutdown::Both);
    }
    state.generation = format!("claude:{}", uuid::Uuid::new_v4());
}
fn alive(shared: &Shared, entry: &Pending) -> bool {
    if Instant::now() >= entry.expires || !(shared.validate)(&entry.identity, &entry.request) {
        return false;
    }
    let mut byte = [0];
    matches!(entry.stream.peek(&mut byte), Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut))
}
fn prune(shared: &Shared, state: &mut State) {
    state.pending.retain(|_, entry| {
        let keep = alive(shared, entry);
        if !keep {
            let _ = entry.stream.shutdown(Shutdown::Both);
        }
        keep
    });
}
impl Service {
    pub fn start(
        endpoint_path: PathBuf,
        validate: impl Fn(&Identity, &Request) -> bool + Send + Sync + 'static,
    ) -> Result<Self, String> {
        Self::start_with_timeout(endpoint_path, validate, Duration::from_secs(110))
    }
    fn start_with_timeout(
        endpoint_path: PathBuf,
        validate: impl Fn(&Identity, &Request) -> bool + Send + Sync + 'static,
        timeout: Duration,
    ) -> Result<Self, String> {
        let parent = endpoint_path.parent().ok_or("invalid endpoint path")?;
        if !safe_path(&endpoint_path) {
            return Err("unsafe endpoint path".into());
        }
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        if !safe_path(&endpoint_path) {
            return Err("unsafe endpoint path".into());
        }
        // A separate empty lock file permits hook readers while reserving endpoint ownership.
        let lock_path = endpoint_path.with_extension("lock");
        if !safe_path(&lock_path) {
            return Err("unsafe endpoint lock".into());
        }
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)
            .map_err(|e| e.to_string())?;
        lock.try_lock().map_err(|e| e.to_string())?;
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).map_err(|e| e.to_string())?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let token = uuid::Uuid::new_v4().to_string();
        let endpoint = Endpoint {
            addr: listener.local_addr().map_err(|e| e.to_string())?,
            token: token.clone(),
            pid: std::process::id(),
            birth: crate::discovery::process_birth(std::process::id())
                .ok_or("process identity unavailable")?,
        };
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&endpoint_path)
            .map_err(|e| e.to_string())?;
        file.set_len(0).map_err(|e| e.to_string())?;
        serde_json::to_writer(&mut file, &endpoint).map_err(|e| e.to_string())?;
        file.flush().map_err(|e| e.to_string())?;
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                generation: format!("claude:{}", uuid::Uuid::new_v4()),
                pending: HashMap::new(),
                connections: HashMap::new(),
            }),
            validate: Box::new(validate),
            stopped: AtomicBool::new(false),
            active: AtomicUsize::new(0),
            timeout,
        });
        let worker = shared.clone();
        let secret = token.clone();
        let handle = thread::spawn(move || {
            let mut connections = Vec::new();
            while !worker.stopped.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, peer)) if peer.ip() == IpAddr::V4(Ipv4Addr::LOCALHOST) => {
                        if worker.active.fetch_add(1, Ordering::AcqRel) >= 64 {
                            worker.active.fetch_sub(1, Ordering::AcqRel);
                            continue;
                        }
                        let nonce = uuid::Uuid::new_v4().to_string();
                        let Ok(socket) = stream.try_clone() else {
                            worker.active.fetch_sub(1, Ordering::AcqRel);
                            continue;
                        };
                        worker
                            .state
                            .lock()
                            .unwrap()
                            .connections
                            .insert(nonce.clone(), socket);
                        let state = worker.clone();
                        let token = secret.clone();
                        connections.push(thread::spawn(move || {
                            connection(stream, &state, &token, &nonce);
                            state.state.lock().unwrap().connections.remove(&nonce);
                            state.active.fetch_sub(1, Ordering::AcqRel);
                        }));
                    }
                    _ => thread::sleep(Duration::from_millis(20)),
                }
                connections.retain(|h| !h.is_finished());
            }
            for connection in connections {
                let _ = connection.join();
            }
        });
        Ok(Self {
            shared,
            endpoint_path,
            token,
            listener: Mutex::new(Some(handle)),
            endpoint_lock: Mutex::new(Some(lock)),
        })
    }
    pub fn snapshot(&self) -> Vec<View> {
        let mut state = self.shared.state.lock().unwrap();
        prune(&self.shared, &mut state);
        let mut views: Vec<_> = state
            .pending
            .values()
            .map(|entry| entry.view.clone())
            .collect();
        views.sort_by(|a, b| a.context.nonce.cmp(&b.context.nonce));
        views
    }
    pub fn respond(&self, context: &Context, response: Value) -> Result<(), String> {
        let mut state = self.shared.state.lock().unwrap();
        prune(&self.shared, &mut state);
        let entry = state
            .pending
            .get(&context.nonce)
            .ok_or("Claude request expired")?;
        if entry.view.context != *context || context.generation != state.generation {
            return Err("Claude context mismatch".into());
        }
        let reply = entry
            .request
            .response(&response)
            .map_err(|_| "invalid Claude response")?;
        let mut bytes = serde_json::to_vec(&reply).map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        if bytes.len() > FRAME_LIMIT {
            return Err("Claude response too large".into());
        }
        let mut entry = state.pending.remove(&context.nonce).unwrap();
        entry
            .stream
            .set_nonblocking(false)
            .map_err(|e| e.to_string())?;
        entry
            .stream
            .set_write_timeout(Some(Duration::from_millis(300)))
            .map_err(|e| e.to_string())?;
        let result = entry.stream.write_all(&bytes).map_err(|e| e.to_string());
        let _ = entry.stream.shutdown(Shutdown::Both);
        result
    }
    pub fn reset(&self) {
        clear(&mut self.shared.state.lock().unwrap());
    }
    pub fn stop(&self) {
        let mut lock = self.endpoint_lock.lock().unwrap();
        if lock.is_none() {
            return;
        }
        self.shared.stopped.store(true, Ordering::Release);
        self.reset();
        if let Some(handle) = self.listener.lock().unwrap().take() {
            let _ = handle.join();
        }
        if read_endpoint(&self.endpoint_path).is_some_and(|endpoint| endpoint.token == self.token) {
            let _ = fs::remove_file(&self.endpoint_path);
        }
        lock.take();
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use serde_json::json;
    fn setup(timeout: Duration) -> (Service, PathBuf) {
        let path = std::env::temp_dir()
            .join(format!("capy-interventions-{}", uuid::Uuid::new_v4()))
            .join("endpoint.json");
        let service = Service::start_with_timeout(
            path.clone(),
            |identity, _| identity.hook_pid == 42,
            timeout,
        )
        .unwrap();
        (service, path)
    }
    fn client(path: &Path, session: &str) -> TcpStream {
        let endpoint = read_endpoint(path).unwrap();
        let mut stream =
            TcpStream::connect_timeout(&endpoint.addr, Duration::from_millis(300)).unwrap();
        let frame = json!({"token":endpoint.token,"identity":{"hook_pid":42,"hook_birth":1,"claude_pid":2,"claude_birth":3},
            "request":{"hook_event_name":"PermissionRequest","session_id":session,"cwd":"C:\\project","tool_name":"Bash","tool_input":{"command":"echo test"}}});
        let mut bytes = serde_json::to_vec(&frame).unwrap();
        bytes.push(b'\n');
        stream.write_all(&bytes).unwrap();
        stream
    }
    fn wait(service: &Service, count: usize) -> Vec<View> {
        let end = Instant::now() + Duration::from_secs(2);
        loop {
            let views = service.snapshot();
            if views.len() == count {
                return views;
            }
            assert!(
                Instant::now() < end,
                "pending request count did not converge"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }
    const FIRST: &str = "12345678-1234-4123-8123-123456789abc";
    const SECOND: &str = "12345678-1234-4123-8123-123456789abd";
    #[test]
    fn claude_intervention_service_two_concurrent_wrong_context_replay() {
        let (service, path) = setup(Duration::from_secs(3));
        let mut one = client(&path, FIRST);
        let mut two = client(&path, SECOND);
        let views = wait(&service, 2);
        let first = &views
            .iter()
            .find(|v| v.context.thread_id == FIRST)
            .unwrap()
            .context;
        let second = &views
            .iter()
            .find(|v| v.context.thread_id == SECOND)
            .unwrap()
            .context;
        let mut wrong = first.clone();
        wrong.session_id = second.session_id.clone();
        assert!(service
            .respond(&wrong, json!({"decision":"allow"}))
            .is_err());
        let mut stale_generation = first.clone();
        stale_generation.generation = format!("claude:{}", uuid::Uuid::new_v4());
        assert!(service
            .respond(&stale_generation, json!({"decision":"allow"}))
            .is_err());
        assert_eq!(service.snapshot().len(), 2);
        assert!(service.snapshot().iter().any(|view| view.context == *first));
        service.respond(first, json!({"decision":"allow"})).unwrap();
        assert!(service.respond(first, json!({"decision":"allow"})).is_err());
        let response: Value =
            serde_json::from_slice(&read_frame(&mut one, Duration::from_secs(1)).unwrap()).unwrap();
        assert_eq!(
            response["hookSpecificOutput"]["decision"]["behavior"],
            "allow"
        );
        service.respond(second, json!({"decision":"deny"})).unwrap();
        let response: Value =
            serde_json::from_slice(&read_frame(&mut two, Duration::from_secs(1)).unwrap()).unwrap();
        assert_eq!(
            response["hookSpecificOutput"]["decision"]["behavior"],
            "deny"
        );
        service.stop();
        assert!(!path.exists());
        let _ = fs::remove_dir(path.parent().unwrap());
    }
    #[test]
    fn claude_intervention_service_disconnect_timeout_reset() {
        let (service, path) = setup(Duration::from_millis(250));
        let one = client(&path, FIRST);
        wait(&service, 1);
        drop(one);
        wait(&service, 0);
        let mut two = client(&path, SECOND);
        let expired = wait(&service, 1)[0].context.clone();
        thread::sleep(Duration::from_millis(280));
        wait(&service, 0);
        assert!(service
            .respond(&expired, json!({"decision":"allow"}))
            .is_err());
        assert!(read_frame(&mut two, Duration::from_millis(100)).is_err());
        let mut three = client(&path, FIRST);
        let old = wait(&service, 1)[0].context.clone();
        service.reset();
        assert!(service.snapshot().is_empty());
        assert!(service.respond(&old, json!({"decision":"allow"})).is_err());
        assert!(read_frame(&mut three, Duration::from_millis(100)).is_err());
        let mut header = TcpStream::connect(read_endpoint(&path).unwrap().addr).unwrap();
        header.write_all(b"{").unwrap();
        thread::sleep(Duration::from_millis(40));
        service.reset();
        assert!(read_frame(&mut header, Duration::from_millis(100)).is_err());
        service.stop();
        let _ = fs::remove_dir(path.parent().unwrap());
    }
    #[test]
    fn claude_intervention_service_endpoint_lock_auth_and_frame_limit() {
        let (service, path) = setup(Duration::from_secs(3));
        let token = read_endpoint(&path).unwrap().token;
        assert!(Service::start(path.clone(), |_, _| true).is_err());
        assert_eq!(read_endpoint(&path).unwrap().token, token);
        let endpoint = read_endpoint(&path).unwrap();
        let mut wrong = TcpStream::connect(endpoint.addr).unwrap();
        wrong.write_all(b"{\"token\":\"wrong\",\"identity\":{\"hook_pid\":42,\"hook_birth\":1,\"claude_pid\":2,\"claude_birth\":3},\"request\":{}}\n").unwrap();
        assert!(read_frame(&mut wrong, Duration::from_secs(1)).is_err());
        assert!(service.snapshot().is_empty());
        let mut huge = TcpStream::connect(endpoint.addr).unwrap();
        huge.write_all(&vec![b'x'; FRAME_LIMIT]).unwrap();
        assert!(read_frame(&mut huge, Duration::from_secs(2)).is_err());
        assert!(service.snapshot().is_empty());
        drop(service);
        assert!(!path.exists());
        let _ = fs::remove_file(path.with_extension("lock"));
        let _ = fs::remove_dir(path.parent().unwrap());
    }
    #[test]
    fn claude_intervention_service_validator_revocation_closes_active_request() {
        let valid = Arc::new(AtomicBool::new(true));
        let validator = valid.clone();
        let path = std::env::temp_dir()
            .join(format!("capy-interventions-{}", uuid::Uuid::new_v4()))
            .join("endpoint.json");
        let service = Service::start_with_timeout(
            path.clone(),
            move |_, _| validator.load(Ordering::Acquire),
            Duration::from_secs(3),
        )
        .unwrap();
        let mut socket = client(&path, FIRST);
        let context = wait(&service, 1)[0].context.clone();
        valid.store(false, Ordering::Release);
        wait(&service, 0);
        assert!(service
            .respond(&context, json!({"decision":"allow"}))
            .is_err());
        assert!(read_frame(&mut socket, Duration::from_secs(1)).is_err());
        service.stop();
        let _ = fs::remove_file(path.with_extension("lock"));
        let _ = fs::remove_dir(path.parent().unwrap());
    }
    #[test]
    fn claude_intervention_service_stop_closes_pending_socket_and_endpoint() {
        let (service, path) = setup(Duration::from_secs(3));
        let mut socket = client(&path, FIRST);
        let context = wait(&service, 1)[0].context.clone();
        assert!(path.exists());
        service.stop();
        assert!(!path.exists());
        assert!(service.snapshot().is_empty());
        assert!(service
            .respond(&context, json!({"decision":"allow"}))
            .is_err());
        assert!(read_frame(&mut socket, Duration::from_secs(1)).is_err());
        let _ = fs::remove_file(path.with_extension("lock"));
        let _ = fs::remove_dir(path.parent().unwrap());
    }
}
impl Drop for Service {
    fn drop(&mut self) {
        self.stop();
    }
}
fn connection(mut stream: TcpStream, shared: &Shared, token: &str, nonce: &str) {
    let Ok(bytes) = read_frame(&mut stream, Duration::from_secs(2)) else {
        return;
    };
    let Ok(frame) = serde_json::from_slice::<Frame>(&bytes) else {
        return;
    };
    if frame.token != token || shared.stopped.load(Ordering::Acquire) {
        return;
    }
    let Ok(request) = Request::parse(&frame.request) else {
        return;
    };
    if !(shared.validate)(&frame.identity, &request) {
        return;
    }
    if stream.set_nonblocking(true).is_err() {
        return;
    }
    let Ok(response_stream) = stream.try_clone() else {
        return;
    };
    {
        let mut state = shared.state.lock().unwrap();
        if shared.stopped.load(Ordering::Acquire)
            || state.pending.len() >= 64
            || !state.connections.contains_key(nonce)
        {
            return;
        }
        let context = Context {
            nonce: nonce.to_owned(),
            generation: state.generation.clone(),
            session_id: format!("claude:{}", request.session_id),
            thread_id: request.session_id.clone(),
            turn_id: nonce.to_owned(),
            item_id: request
                .tool_use_id
                .clone()
                .unwrap_or_else(|| nonce.to_owned()),
        };
        let view = View {
            context,
            status: "pending".into(),
            body: request.body.clone(),
            decisions: request.decisions.clone(),
        };
        state.pending.insert(
            nonce.to_owned(),
            Pending {
                identity: frame.identity,
                request,
                view,
                stream: response_stream,
                expires: Instant::now() + shared.timeout,
            },
        );
    }
    loop {
        thread::sleep(Duration::from_millis(20));
        let mut state = shared.state.lock().unwrap();
        let Some(entry) = state.pending.get(nonce) else {
            break;
        };
        if shared.stopped.load(Ordering::Acquire) || !alive(shared, entry) {
            if let Some(entry) = state.pending.remove(nonce) {
                let _ = entry.stream.shutdown(Shutdown::Both);
            }
            break;
        }
    }
}
