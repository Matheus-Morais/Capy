use super::{
    protocol::{Connection, RespondError},
    registry::{Context, Source, View},
};
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::HashMap,
    io::{self, Read, Write},
    path::PathBuf,
    process::{Child, ChildStdin},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc, Arc, Mutex,
    },
    time::{Duration, Instant},
};

type Ack = mpsc::Sender<Result<(), String>>;
type Validate = Arc<dyn Fn(&Source) -> bool + Send + Sync>;
enum Command {
    Connect(Source, Ack),
    Disconnect(String, Ack),
    Respond(Context, Source, Value, Ack),
    Stop,
    Reset,
}
#[derive(Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Subscription {
    pub session_id: String,
    pub status: String,
    pub message: String,
}
#[derive(Default)]
struct Output {
    requests: Vec<View>,
    subscriptions: Vec<Subscription>,
}
pub struct Service {
    sender: mpsc::SyncSender<Command>,
    output: Arc<Mutex<Output>>,
}
impl Service {
    pub fn new(home: PathBuf, validate: impl Fn(&Source) -> bool + Send + Sync + 'static) -> Self {
        let (sender, receiver) = mpsc::sync_channel(64);
        let output = Arc::new(Mutex::new(Output::default()));
        let result = output.clone();
        std::thread::spawn(move || {
            worker(
                home,
                Arc::new(Mutex::new(receiver)),
                result,
                Arc::new(validate),
            )
        });
        Self { sender, output }
    }
    fn call(&self, command: impl FnOnce(Ack) -> Command) -> Result<(), String> {
        let (ack, result) = mpsc::channel();
        self.sender
            .try_send(command(ack))
            .map_err(|_| "Serviço de respostas ocupado ou indisponível.".to_owned())?;
        result.recv_timeout(Duration::from_secs(10)).map_err(|_|"A entrega não foi confirmada. Não reenvie automaticamente; confira o pedido na origem.".to_owned())?
    }
    pub fn connect(&self, source: Source) -> Result<(), String> {
        self.call(|ack| Command::Connect(source, ack))
    }
    pub fn disconnect(&self, thread: String) -> Result<(), String> {
        self.call(|ack| Command::Disconnect(thread, ack))
    }
    pub fn respond(&self, context: Context, source: Source, response: Value) -> Result<(), String> {
        self.call(|ack| Command::Respond(context, source, response, ack))
    }
    pub fn snapshot(&self) -> (Vec<View>, Vec<Subscription>) {
        self.output
            .lock()
            .map(|o| (o.requests.clone(), o.subscriptions.clone()))
            .unwrap_or_default()
    }
    pub fn stop(&self) {
        let _ = self.sender.try_send(Command::Stop);
    }
    pub fn reset(&self) {
        unavailable(&self.output);
        let _ = self.sender.try_send(Command::Reset);
    }
}
impl Drop for Service {
    fn drop(&mut self) {
        self.stop();
    }
}

fn subscription(output: &Arc<Mutex<Output>>, source: &Source, status: &str, message: &str) {
    if let Ok(mut output) = output.lock() {
        let row = Subscription {
            session_id: source.session_id.clone(),
            status: status.into(),
            message: message.into(),
        };
        if let Some(old) = output
            .subscriptions
            .iter_mut()
            .find(|s| s.session_id == source.session_id)
        {
            *old = row;
        } else {
            if output.subscriptions.len() >= 64 {
                if let Some(at) = output
                    .subscriptions
                    .iter()
                    .position(|s| s.status != "connected" && s.status != "connecting")
                {
                    output.subscriptions.remove(at);
                } else {
                    return;
                }
            }
            output.subscriptions.push(row);
        }
    }
}
fn unavailable(output: &Arc<Mutex<Output>>) {
    if let Ok(mut output) = output.lock() {
        output.requests.clear();
        for s in &mut output.subscriptions {
            if matches!(s.status.as_str(), "connected" | "connecting") {
                s.status = "disconnected".into();
                s.message = "Conexão encerrada. Reconecte para receber os pedidos atuais.".into();
            }
        }
    }
}
fn reject(command: Command) {
    match command {
        Command::Connect(_, ack) | Command::Disconnect(_, ack) | Command::Respond(_, _, _, ack) => {
            let _ = ack.send(Err("Conexão indisponível ou pedido expirado.".into()));
        }
        Command::Stop | Command::Reset => {}
    }
}

fn worker(
    home: PathBuf,
    receiver: Arc<Mutex<mpsc::Receiver<Command>>>,
    output: Arc<Mutex<Output>>,
    validate: Validate,
) {
    loop {
        let command = match receiver.lock().ok().and_then(|r| r.recv().ok()) {
            Some(command) => command,
            None => return,
        };
        let (source, ack) = match command {
            Command::Connect(source, ack) => (source, ack),
            Command::Stop => return,
            Command::Reset => {
                unavailable(&output);
                continue;
            }
            other => {
                reject(other);
                continue;
            }
        };
        if !validate(&source) {
            let _ = ack.send(Err(
                "A sessão mudou ou não está disponível para respostas.".into()
            ));
            continue;
        }
        subscription(
            &output,
            &source,
            "connecting",
            "Conectando aos pedidos desta conversa…",
        );
        #[cfg(windows)]
        {
            let Ok(mut child) = crate::activity::proxy::spawn_proxy(&home) else {
                subscription(
                    &output,
                    &source,
                    "disconnected",
                    "O daemon Codex está indisponível.",
                );
                let _ = ack.send(Err("O daemon Codex está indisponível.".into()));
                continue;
            };
            let Ok(pipe) = BufferedPipe::new(&mut child) else {
                let _ = child.kill();
                let _ = child.wait();
                unavailable(&output);
                let _ = ack.send(Err("Transporte indisponível.".into()));
                continue;
            };
            let progress = Arc::new(AtomicU64::new(0));
            let cancelled = Arc::new(AtomicBool::new(false));
            let cancellation = cancelled.clone();
            let ticks = progress.clone();
            let commands = receiver.clone();
            let shared = output.clone();
            let guard = validate.clone();
            let (finished, done) = mpsc::channel();
            std::thread::spawn(move || {
                let _ = finished.send(run(
                    pipe,
                    commands,
                    shared,
                    guard,
                    (source, ack),
                    ticks,
                    cancellation,
                ));
            });
            let mut last = 0;
            let mut advanced = Instant::now();
            loop {
                match done.recv_timeout(Duration::from_millis(250)) {
                    Ok(_) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
                let now = progress.load(Ordering::Relaxed);
                if now != last {
                    last = now;
                    advanced = Instant::now();
                }
                // initialize and resume each have an eight-second RPC deadline.
                // Give the complete two-step startup a bounded window to finish.
                if advanced.elapsed() >= Duration::from_secs(20) {
                    cancelled.store(true, Ordering::Release);
                    let _ = child.kill();
                    let _ = done.recv_timeout(Duration::from_secs(2));
                    break;
                }
            }
            let _ = child.kill();
            let _ = child.wait();
            unavailable(&output);
        }
        #[cfg(not(windows))]
        {
            let _ = &home;
            subscription(
                &output,
                &source,
                "disconnected",
                "Respostas disponíveis somente no Windows.",
            );
            let _ = ack.send(Err("Plataforma não suportada.".into()));
        }
    }
}

fn run(
    mut pipe: BufferedPipe,
    receiver: Arc<Mutex<mpsc::Receiver<Command>>>,
    output: Arc<Mutex<Output>>,
    validate: Validate,
    first: (Source, Ack),
    progress: Arc<AtomicU64>,
    cancelled: Arc<AtomicBool>,
) -> Result<(), ()> {
    pipe.timeout = Duration::from_secs(8);
    let (source, ack) = first;
    let mut connection = match Connection::connect(pipe) {
        Ok(c) => c,
        Err(_) => {
            let _ = ack.send(Err("Conexão com Codex falhou.".into()));
            return Err(());
        }
    };
    connection.stream_mut().timeout = Duration::from_millis(200);
    if connection.resume(source.clone()).is_err() {
        let _ = ack.send(Err("Não foi possível inscrever a conversa original.".into()));
        return Err(());
    }
    subscription(
        &output,
        &source,
        "connected",
        "Respostas conectadas a esta conversa.",
    );
    let _ = ack.send(Ok(()));
    let mut sources = HashMap::from([(source.thread_id.clone(), source)]);
    let mut deliveries: HashMap<String, (Ack, Instant)> = HashMap::new();
    let mut checked = Instant::now();
    let outcome = (|| loop {
        if cancelled.load(Ordering::Acquire) {
            return Err(());
        }
        progress.fetch_add(1, Ordering::Relaxed);
        let command = receiver.lock().map_err(|_| ())?.try_recv();
        match command {
            Ok(Command::Stop) => return Ok(()),
            Ok(Command::Reset) => return Ok(()),
            Err(mpsc::TryRecvError::Disconnected) => return Ok(()),
            Err(mpsc::TryRecvError::Empty) => {}
            Ok(Command::Connect(source, ack)) => {
                if !validate(&source) || cancelled.load(Ordering::Acquire) {
                    let _ = ack.send(Err("Sessão indisponível.".into()));
                    continue;
                }
                if sources.get(&source.thread_id) == Some(&source) {
                    let _ = ack.send(Ok(()));
                    continue;
                }
                subscription(
                    &output,
                    &source,
                    "connecting",
                    "Conectando aos pedidos desta conversa…",
                );
                if connection.resume(source.clone()).is_err() {
                    let _ = ack.send(Err("Inscrição indisponível.".into()));
                    if connection.is_closed() {
                        return Err(());
                    }
                    subscription(&output, &source, "disconnected", "Inscrição indisponível.");
                } else {
                    subscription(
                        &output,
                        &source,
                        "connected",
                        "Respostas conectadas a esta conversa.",
                    );
                    sources.insert(source.thread_id.clone(), source);
                    let _ = ack.send(Ok(()));
                }
            }
            Ok(Command::Disconnect(thread, ack)) => {
                if let Some(source) = sources.remove(&thread) {
                    subscription(&output, &source, "disconnected", "Respostas desconectadas.");
                    if connection.unsubscribe(&thread).is_err() {
                        let _ = ack.send(Err("Desconexão não confirmada.".into()));
                        return Err(());
                    }
                }
                let _ = ack.send(Ok(()));
                if sources.is_empty() {
                    return Ok(());
                }
            }
            Ok(Command::Respond(context, source, response, ack)) => {
                if !validate(&source)
                    || cancelled.load(Ordering::Acquire)
                    || sources.get(&source.thread_id) != Some(&source)
                {
                    let _ = ack.send(Err("Pedido expirado ou sessão mudou.".into()));
                    continue;
                }
                match connection.respond(&context, &source, &response) {
                    Ok(()) => {
                        deliveries.insert(context.nonce, (ack, Instant::now()));
                    }
                    Err(RespondError::Expired) => {
                        let _ = ack.send(Err("Pedido expirado ou sessão mudou.".into()));
                    }
                    Err(RespondError::Transport) => {
                        let _ = ack.send(Err(
                            "Resposta não confirmada: conexão encerrada. Confira na origem.".into(),
                        ));
                        return Err(());
                    }
                }
            }
        }
        connection.pump()?;
        if cancelled.load(Ordering::Acquire) {
            return Err(());
        }
        for context in connection.take_resolved() {
            if let Some((ack, _)) = deliveries.remove(&context.nonce) {
                let _ = ack.send(Ok(()));
            }
        }
        deliveries.retain(|_, (ack, at)| {
            if at.elapsed() >= Duration::from_secs(8) {
                let _ = ack.send(Err(
                    "Entrega não confirmada; confira o pedido na origem.".into()
                ));
                false
            } else {
                true
            }
        });
        if checked.elapsed() >= Duration::from_secs(5) {
            checked = Instant::now();
            for source in sources.values().cloned().collect::<Vec<_>>() {
                if !validate(&source) || cancelled.load(Ordering::Acquire) {
                    sources.remove(&source.thread_id);
                    subscription(
                        &output,
                        &source,
                        "disconnected",
                        "Sessão indisponível para respostas.",
                    );
                    connection.unsubscribe(&source.thread_id)?;
                }
            }
            if sources.is_empty() {
                return Ok(());
            }
        }
        if let Ok(mut result) = output.lock() {
            if !cancelled.load(Ordering::Acquire) {
                result.requests = connection.views();
            }
        }
    })();
    for (_, (ack, _)) in deliveries {
        let _ = ack.send(Err("Conexão encerrada; entrega não confirmada.".into()));
    }
    outcome
}

struct BufferedPipe {
    input: ChildStdin,
    receiver: mpsc::Receiver<io::Result<Vec<u8>>>,
    buffer: Vec<u8>,
    offset: usize,
    timeout: Duration,
}
impl BufferedPipe {
    fn new(child: &mut Child) -> Result<Self, ()> {
        let input = child.stdin.take().ok_or(())?;
        let mut output = child.stdout.take().ok_or(())?;
        let (sender, receiver) = mpsc::sync_channel(4);
        std::thread::spawn(move || loop {
            let mut bytes = vec![0; 65_536];
            let read = output.read(&mut bytes);
            let last = match read {
                Ok(0) => Err(io::Error::from(io::ErrorKind::UnexpectedEof)),
                Ok(count) => {
                    bytes.truncate(count);
                    Ok(bytes)
                }
                Err(error) => Err(error),
            };
            let terminal = last.is_err();
            if sender.send(last).is_err() || terminal {
                return;
            }
        });
        Ok(Self {
            input,
            receiver,
            buffer: Vec::new(),
            offset: 0,
            timeout: Duration::from_secs(8),
        })
    }
}
impl Read for BufferedPipe {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        if self.offset == self.buffer.len() {
            self.buffer = self.receiver.recv_timeout(self.timeout).map_err(|error| {
                io::Error::from(match error {
                    mpsc::RecvTimeoutError::Timeout => io::ErrorKind::WouldBlock,
                    mpsc::RecvTimeoutError::Disconnected => io::ErrorKind::UnexpectedEof,
                })
            })??;
            self.offset = 0;
        }
        let count = out.len().min(self.buffer.len() - self.offset);
        out[..count].copy_from_slice(&self.buffer[self.offset..self.offset + count]);
        self.offset += count;
        Ok(count)
    }
}
impl Write for BufferedPipe {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.input.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.input.flush()
    }
}
