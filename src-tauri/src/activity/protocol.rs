use super::Observation;
use serde_json::{json, Value};
use std::io::{Read, Write};
use tungstenite::{client::client_with_config, protocol::WebSocketConfig, Message, WebSocket};

struct Client<S> {
    socket: WebSocket<S>,
    next_id: u64,
    account_changed: bool,
}
enum Request<'a> {
    Initialize,
    Loaded,
    Read(&'a str),
    Account,
    Limits,
}
impl<S: Read + Write> Client<S> {
    fn request(&mut self, request: Request<'_>) -> Result<Value, ()> {
        let (method, params) = match request {
            Request::Initialize => (
                "initialize",
                json!({"clientInfo":{"name":"capy_activity","version":env!("CARGO_PKG_VERSION")}}),
            ),
            Request::Loaded => ("thread/loaded/list", json!({})),
            Request::Read(id) => ("thread/read", json!({"threadId":id,"includeTurns":false})),
            Request::Account => ("account/read", json!({"refreshToken":false})),
            Request::Limits => ("account/rateLimits/read", json!({})),
        };
        self.next_id += 1;
        let id = self.next_id;
        self.socket
            .send(Message::Text(
                json!({"id":id,"method":method,"params":params})
                    .to_string()
                    .into(),
            ))
            .map_err(|_| ())?;
        for _ in 0..128 {
            let message = self.socket.read().map_err(|_| ())?;
            let Message::Text(text) = message else {
                continue;
            };
            let response: Value = serde_json::from_str(&text).map_err(|_| ())?;
            if response["method"].as_str() == Some("account/updated") {
                self.account_changed = true;
            }
            if response["id"].as_u64() != Some(id) {
                continue;
            }
            if response.get("error").is_some() {
                return Err(());
            }
            return response.get("result").cloned().ok_or(());
        }
        Err(())
    }
}
fn connect<S: Read + Write>(stream: S) -> Result<Client<S>, ()> {
    let config = WebSocketConfig::default()
        .max_message_size(Some(1_048_576))
        .max_frame_size(Some(1_048_576));
    let (socket, _) =
        client_with_config("ws://localhost/", stream, Some(config)).map_err(|_| ())?;
    let mut client = Client {
        socket,
        next_id: 0,
        account_changed: false,
    };
    client.request(Request::Initialize)?;
    client
        .socket
        .send(Message::Text(
            json!({"method":"initialized","params":{}})
                .to_string()
                .into(),
        ))
        .map_err(|_| ())?;
    Ok(client)
}
pub(crate) fn quota<S: Read + Write>(stream: S) -> Result<crate::quotas::RawSample, ()> {
    let mut client = connect(stream)?;
    let before = client.request(Request::Account)?;
    client.account_changed = false;
    let limits = client.request(Request::Limits)?;
    let after = client.request(Request::Account)?;
    Ok(crate::quotas::RawSample {
        before,
        limits,
        after,
        identity_changed: client.account_changed,
    })
}
pub(super) fn observe<S: Read + Write>(stream: S, ids: &[String]) -> Result<Vec<Observation>, ()> {
    let mut client = connect(stream)?;
    let loaded = client.request(Request::Loaded)?;
    let loaded = loaded["data"].as_array().ok_or(())?;
    let mut observations = Vec::new();
    for id in ids.iter().take(super::SESSION_LIMIT).filter(|id| {
        loaded
            .iter()
            .any(|value| value.as_str() == Some(id.as_str()))
    }) {
        let result = client.request(Request::Read(id))?;
        if let Some(thread) = result.get("thread") {
            if let Ok(observation) = serde_json::from_value::<Observation>(thread.clone()) {
                if observation.id == *id {
                    observations.push(observation);
                }
            }
        }
    }
    Ok(observations)
}
