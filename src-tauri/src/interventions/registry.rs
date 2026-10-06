use super::{Body, Callback, Invalid};
use crate::demo::Session;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct Source {
    pub session_id: String,
    pub thread_id: String,
    pub origin: String,
}
impl Source {
    pub fn from_session(session: &Session, real: bool) -> Result<Self, Invalid> {
        let thread = session.id.strip_prefix("codex:").ok_or(Invalid::Identity)?;
        let uuid = thread.len() == 36
            && thread.bytes().enumerate().all(|(i, b)| {
                if [8, 13, 18, 23].contains(&i) {
                    b == b'-'
                } else {
                    b.is_ascii_hexdigit()
                }
            });
        if !real || session.hidden || session.kind != "codex" || session.origin.is_empty() || !uuid
        {
            return Err(Invalid::Identity);
        }
        Ok(Self {
            session_id: session.id.clone(),
            thread_id: thread.to_owned(),
            origin: session.origin.clone(),
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Context {
    pub nonce: String,
    pub generation: String,
    pub session_id: String,
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
}
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct View {
    #[serde(flatten)]
    pub context: Context,
    pub status: String,
    pub body: Value,
    pub decisions: Vec<String>,
}
#[derive(Clone, Debug)]
struct Entry {
    source: Source,
    callback: Callback,
    view: View,
}

pub struct Registry {
    instance: String,
    generation: u64,
    sequence: u64,
    sources: HashMap<String, Source>,
    requests: HashMap<String, Entry>,
}
impl Default for Registry {
    fn default() -> Self {
        static INSTANCES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let serial = INSTANCES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self {
            instance: format!("{}:{at}:{serial}", std::process::id()),
            generation: 0,
            sequence: 0,
            sources: HashMap::new(),
            requests: HashMap::new(),
        }
    }
}
impl Registry {
    pub fn subscribed(&self, thread: &str) -> bool {
        self.sources.contains_key(thread)
    }
    fn generation_key(&self) -> String {
        format!("{}:{}", self.instance, self.generation)
    }
    pub fn begin_connection(&mut self) -> String {
        self.clear();
        self.generation = self
            .generation
            .checked_add(1)
            .expect("connection generation exhausted");
        self.generation_key()
    }
    pub fn subscribe(&mut self, source: Source) -> Result<(), Invalid> {
        if self.generation == 0
            || (self.sources.len() >= 64 && !self.sources.contains_key(&source.thread_id))
        {
            return Err(Invalid::Identity);
        }
        if self
            .sources
            .get(&source.thread_id)
            .is_some_and(|old| old != &source)
        {
            self.unsubscribe(&source.thread_id);
        }
        self.sources.insert(source.thread_id.clone(), source);
        Ok(())
    }
    pub fn unsubscribe(&mut self, thread: &str) {
        self.sources.remove(thread);
        self.requests
            .retain(|_, entry| entry.callback.thread_id != thread);
    }
    pub fn clear(&mut self) {
        self.sources.clear();
        self.requests.clear();
    }
    pub fn invalidate_requests(&mut self) {
        self.requests.clear();
        self.generation = self
            .generation
            .checked_add(1)
            .expect("connection generation exhausted");
    }
    pub fn receive(&mut self, callback: Callback) -> Result<Context, Invalid> {
        let source = self
            .sources
            .get(&callback.thread_id)
            .ok_or(Invalid::Identity)?
            .clone();
        if let Some(entry) = self
            .requests
            .values()
            .find(|entry| entry.callback.rpc_id == callback.rpc_id)
        {
            return if entry.callback == callback {
                Ok(entry.view.context.clone())
            } else {
                Err(Invalid::Identity)
            };
        }
        if self.requests.len() >= 64 {
            return Err(Invalid::Payload);
        }
        self.sequence = self.sequence.checked_add(1).ok_or(Invalid::Identity)?;
        let context = Context {
            nonce: format!("{}:{}", self.generation_key(), self.sequence),
            generation: self.generation_key(),
            session_id: source.session_id.clone(),
            thread_id: callback.thread_id.clone(),
            turn_id: callback.turn_id.clone(),
            item_id: callback.item_id.clone(),
        };
        let body = match &callback.body {
            Body::Question(questions) => json!({"kind":"question","questions":questions}),
            Body::Command {
                command,
                cwd,
                reason,
            } => json!({"kind":"command","command":command,"cwd":cwd,"reason":reason}),
            Body::Files { changes, reason } => {
                json!({"kind":"files","changes":changes,"reason":reason})
            }
        };
        let view = View {
            context: context.clone(),
            status: "pending".into(),
            body,
            decisions: callback.decisions.clone(),
        };
        self.requests.insert(
            context.nonce.clone(),
            Entry {
                source,
                callback,
                view,
            },
        );
        Ok(context)
    }
    pub fn views(&self) -> Vec<View> {
        let mut views: Vec<_> = self
            .requests
            .values()
            .map(|entry| entry.view.clone())
            .collect();
        views.sort_by(|a, b| a.context.nonce.cmp(&b.context.nonce));
        views
    }
    pub fn prepare(
        &mut self,
        context: &Context,
        current: &Source,
        response: &Value,
    ) -> Result<Value, Invalid> {
        let generation = self.generation_key();
        let entry = self
            .requests
            .get_mut(&context.nonce)
            .ok_or(Invalid::Identity)?;
        if entry.view.context != *context
            || entry.source != *current
            || entry.view.status != "pending"
            || context.generation != generation
            || self.sources.get(&current.thread_id) != Some(current)
        {
            return Err(Invalid::Identity);
        }
        let reply = match &entry.callback.body {
            Body::Question(_) => {
                if response.as_object().is_none_or(|v| v.len() != 1) {
                    return Err(Invalid::Decision);
                }
                entry.callback.answer(&response["answers"])?
            }
            _ => {
                if response.as_object().is_none_or(|v| v.len() != 1) {
                    return Err(Invalid::Decision);
                }
                entry
                    .callback
                    .approval(response["decision"].as_str().ok_or(Invalid::Decision)?)?
            }
        };
        entry.view.status = "submitting".into();
        Ok(reply)
    }
    pub fn event(&mut self, event: &Value) {
        let p = &event["params"];
        match event["method"].as_str() {
            Some("serverRequest/resolved") => self.requests.retain(|_, entry| {
                !(p["threadId"] == entry.callback.thread_id
                    && p["requestId"] == entry.callback.rpc_id)
            }),
            Some("turn/completed") => self.requests.retain(|_, entry| {
                !(p["threadId"] == entry.callback.thread_id
                    && p["turn"]["id"] == entry.callback.turn_id)
            }),
            Some("thread/closed" | "thread/archived") => {
                if let Some(thread) = p["threadId"].as_str() {
                    self.unsubscribe(thread);
                }
            }
            Some("account/updated") => self.clear(),
            _ => {}
        }
    }
}
