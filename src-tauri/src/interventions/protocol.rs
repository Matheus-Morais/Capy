use super::{
    registry::{Context, Registry, Source, View},
    Callback,
};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    io::{ErrorKind, Read, Write},
    time::{Duration, Instant},
};
use tungstenite::{client::client_with_config, protocol::WebSocketConfig, Message, WebSocket};

pub struct Connection<S> {
    socket: WebSocket<S>,
    registry: Registry,
    next_id: u64,
    waiting: Option<String>,
    result: Option<Value>,
    items: HashMap<(String, String, String), Value>,
    resolved: Vec<Context>,
    closed: bool,
    last_received: Instant,
    last_ping: Instant,
    auth_mode: Option<Option<String>>,
    initial_auth_updates: Vec<Option<String>>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RespondError {
    Expired,
    Transport,
}
fn account_mode(account: &Value) -> Option<Option<String>> {
    if account["account"].is_null() {
        return Some(None);
    }
    let kind = account["account"]["type"].as_str()?;
    let mode = match kind {
        "apiKey" => "apikey",
        "chatgpt" | "chatgptAuthTokens" | "agentIdentity" | "personalAccessToken" => kind,
        "amazonBedrock" if account["account"]["credentialSource"] == "codexManaged" => {
            "bedrockApiKey"
        }
        "amazonBedrock" => "awsManaged",
        _ => return None,
    };
    Some(Some(mode.to_owned()))
}
impl<S: Read + Write> Connection<S> {
    pub fn connect(stream: S) -> Result<Self, ()> {
        let config = WebSocketConfig::default()
            .max_message_size(Some(1_048_576))
            .max_frame_size(Some(1_048_576));
        let (socket, _) =
            client_with_config("ws://localhost/", stream, Some(config)).map_err(|_| ())?;
        let mut registry = Registry::default();
        registry.begin_connection();
        let mut connection = Self {
            socket,
            registry,
            next_id: 0,
            waiting: None,
            result: None,
            items: HashMap::new(),
            resolved: Vec::new(),
            closed: false,
            last_received: Instant::now(),
            last_ping: Instant::now(),
            auth_mode: None,
            initial_auth_updates: Vec::new(),
        };
        connection.rpc("initialize",json!({"clientInfo":{"name":"capy_interventions","version":env!("CARGO_PKG_VERSION")},"capabilities":{"experimentalApi":true}}))?;
        connection.send(json!({"method":"initialized","params":{}}))?;
        let account = connection.rpc("account/read", json!({"refreshToken":false}))?;
        let mode = account_mode(&account).ok_or(())?;
        if connection
            .initial_auth_updates
            .iter()
            .any(|observed| *observed != mode)
        {
            connection.fail();
            return Err(());
        }
        connection.auth_mode = Some(mode);
        Ok(connection)
    }
    pub fn stream_mut(&mut self) -> &mut S {
        self.socket.get_mut()
    }
    fn fail(&mut self) {
        self.closed = true;
        self.registry.clear();
        self.items.clear();
        self.resolved.clear();
    }
    fn send(&mut self, value: Value) -> Result<(), ()> {
        if self.closed {
            return Err(());
        }
        if self
            .socket
            .send(Message::Text(value.to_string().into()))
            .is_err()
        {
            self.fail();
            return Err(());
        }
        Ok(())
    }
    fn rpc(&mut self, method: &str, params: Value) -> Result<Value, ()> {
        self.next_id = self.next_id.checked_add(1).ok_or(())?;
        let id = format!("capy-rpc-{}", self.next_id);
        self.waiting = Some(id.clone());
        self.result = None;
        if self
            .send(json!({"id":id,"method":method,"params":params}))
            .is_err()
        {
            #[cfg(feature = "intervention-proof")]
            eprintln!("rpc_failure method={method} kind=send");
            return Err(());
        }
        let deadline = Instant::now() + Duration::from_secs(8);
        let mut received = 0;
        while Instant::now() < deadline && received < 128 {
            let read = match self.read_one() {
                Ok(read) => read,
                Err(()) => {
                    #[cfg(feature = "intervention-proof")]
                    eprintln!("rpc_failure method={method} kind=read");
                    return Err(());
                }
            };
            if read {
                received += 1;
            }
            if let Some(response) = self.result.take() {
                self.waiting = None;
                if response.get("error").is_some() {
                    #[cfg(feature = "intervention-proof")]
                    eprintln!(
                        "rpc_error method={method} code={}",
                        response["error"]["code"]
                            .as_i64()
                            .map(|n| n.to_string())
                            .unwrap_or_else(|| "unknown".into())
                    );
                    return Err(());
                }
                return response.get("result").cloned().ok_or(());
            }
        }
        #[cfg(feature = "intervention-proof")]
        eprintln!("rpc_failure method={method} kind=timeout");
        self.fail();
        Err(())
    }
    pub fn resume(&mut self, source: Source) -> Result<(), ()> {
        self.registry.subscribe(source.clone()).map_err(|_| ())?;
        let result = self.rpc(
            "thread/resume",
            json!({"threadId":source.thread_id,"excludeTurns":true}),
        );
        #[cfg(feature = "intervention-proof")]
        eprintln!(
            "resume_result={}",
            result
                .as_ref()
                .map(|r| (r["thread"]["id"] == source.thread_id).to_string())
                .unwrap_or_else(|_| "rpc_error".into())
        );
        if !result.is_ok_and(|result| result["thread"]["id"] == source.thread_id) {
            self.registry.unsubscribe(&source.thread_id);
            return Err(());
        }
        Ok(())
    }
    pub fn unsubscribe(&mut self, thread: &str) -> Result<(), ()> {
        self.registry.unsubscribe(thread);
        self.items.retain(|(t, _, _), _| t != thread);
        self.rpc("thread/unsubscribe", json!({"threadId":thread}))
            .map(|_| ())
    }
    pub fn views(&self) -> Vec<View> {
        self.registry.views()
    }
    pub fn is_closed(&self) -> bool {
        self.closed
    }
    pub fn take_resolved(&mut self) -> Vec<Context> {
        std::mem::take(&mut self.resolved)
    }
    pub fn respond(
        &mut self,
        context: &Context,
        current: &Source,
        response: &Value,
    ) -> Result<(), RespondError> {
        if self.closed {
            return Err(RespondError::Transport);
        }
        let message = self
            .registry
            .prepare(context, current, response)
            .map_err(|_| RespondError::Expired)?;
        self.send(message).map_err(|_| RespondError::Transport)
    }
    pub fn pump(&mut self) -> Result<(), ()> {
        if self.closed {
            return Err(());
        }
        if self.last_received.elapsed() > Duration::from_secs(45) {
            self.fail();
            return Err(());
        }
        if self.last_ping.elapsed() >= Duration::from_secs(15) {
            self.last_ping = Instant::now();
            if self.socket.send(Message::Ping(Vec::new().into())).is_err() {
                self.fail();
                return Err(());
            }
        }
        self.read_one().map(|_| ())
    }
    fn read_one(&mut self) -> Result<bool, ()> {
        if self.closed {
            return Err(());
        }
        let message = match self.socket.read() {
            Ok(message) => message,
            Err(tungstenite::Error::Io(error))
                if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) =>
            {
                return Ok(false)
            }
            Err(error) => {
                #[cfg(feature = "intervention-proof")]
                eprintln!(
                    "websocket_read_error={}",
                    match &error {
                        tungstenite::Error::Io(io) => format!("io:{:?}", io.kind()),
                        tungstenite::Error::Protocol(_) => "protocol".into(),
                        tungstenite::Error::ConnectionClosed => "closed".into(),
                        tungstenite::Error::AlreadyClosed => "already_closed".into(),
                        tungstenite::Error::Capacity(_) => "capacity".into(),
                        tungstenite::Error::Utf8(_) => "utf8".into(),
                        _ => "other".into(),
                    }
                );
                self.fail();
                return Err(());
            }
        };
        self.last_received = Instant::now();
        if let Message::Close(_) = message {
            self.fail();
            return Err(());
        }
        let Message::Text(text) = message else {
            return Ok(true);
        };
        let value: Value = serde_json::from_str(&text).map_err(|_| {
            self.fail();
        })?;
        if value.get("method").is_none() {
            if self.waiting.as_ref().is_some_and(|id| value["id"] == *id) {
                self.result = Some(value);
            }
            return Ok(true);
        }
        let p = &value["params"];
        if value["method"] == "account/updated" {
            let observed = value["params"]["authMode"].as_str().map(str::to_owned);
            match &self.auth_mode {
                Some(expected) if *expected == observed => self.registry.invalidate_requests(),
                Some(_) => {
                    self.registry.event(&value);
                    self.fail();
                    return Err(());
                }
                None if self.initial_auth_updates.len() < 8 => {
                    self.initial_auth_updates.push(observed);
                }
                None => {
                    self.fail();
                    return Err(());
                }
            }
            return Ok(true);
        }
        if value["method"] == "serverRequest/resolved" {
            let before = self.registry.views();
            self.registry.event(&value);
            let after = self.registry.views();
            for view in before {
                if !after.iter().any(|v| v.context == view.context) {
                    if self.resolved.len() < 64 {
                        self.resolved.push(view.context);
                    }
                }
            }
        } else {
            self.registry.event(&value);
        }
        if value["method"] == "item/started" && p["item"]["type"] == "fileChange" {
            if let (Some(thread), Some(turn), Some(item)) = (
                p["threadId"].as_str(),
                p["turnId"].as_str(),
                p["item"]["id"].as_str(),
            ) {
                if self.registry.subscribed(thread)
                    && self.items.len() < 64
                    && serde_json::to_vec(&p["item"]).is_ok_and(|v| v.len() <= 65_536)
                {
                    self.items
                        .insert((thread.into(), turn.into(), item.into()), p["item"].clone());
                }
            }
        }
        if value["method"] == "item/completed" {
            if let (Some(thread), Some(turn), Some(item)) = (
                p["threadId"].as_str(),
                p["turnId"].as_str(),
                p["item"]["id"].as_str(),
            ) {
                self.items
                    .remove(&(thread.into(), turn.into(), item.into()));
            }
        }
        if value["method"] == "turn/completed" {
            self.items.retain(|(thread, turn, _), _| {
                p["threadId"] != *thread || p["turn"]["id"] != *turn
            });
        }
        if value.get("id").is_some() {
            let key = (
                p["threadId"].as_str().unwrap_or_default().to_owned(),
                p["turnId"].as_str().unwrap_or_default().to_owned(),
                p["itemId"].as_str().unwrap_or_default().to_owned(),
            );
            match Callback::parse(&value, self.items.get(&key)) {
                Ok(callback) => {
                    if let Err(error) = self.registry.receive(callback) {
                        #[cfg(feature = "intervention-proof")]
                        eprintln!("callback_rejected kind=registry:{error:?}");
                    }
                }
                Err(error) => {
                    #[cfg(feature = "intervention-proof")]
                    eprintln!("callback_rejected kind=parse:{error:?}");
                }
            }
        }
        Ok(true)
    }
}
