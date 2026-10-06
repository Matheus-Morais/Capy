use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashSet;

const PAYLOAD_LIMIT: usize = 65_536;

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Question {
    pub id: String,
    pub header: String,
    pub question: String,
    pub options: Vec<OptionLabel>,
    pub is_other: bool,
    pub is_secret: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct OptionLabel {
    pub label: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Body {
    Question(Vec<Question>),
    Command {
        command: String,
        cwd: String,
        reason: Option<String>,
    },
    Files {
        changes: Value,
        reason: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Callback {
    pub rpc_id: Value,
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub body: Body,
    pub decisions: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Invalid {
    Payload,
    Identity,
    Unsupported,
    Context,
    Questions,
    Decision,
}

fn text(value: &Value, max: usize) -> Result<String, Invalid> {
    value
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= max && !s.contains('\0'))
        .map(str::to_owned)
        .ok_or(Invalid::Payload)
}
fn id(value: &Value) -> Result<String, Invalid> {
    let s = text(value, 128).map_err(|_| Invalid::Identity)?;
    if s.chars().any(char::is_control) {
        return Err(Invalid::Identity);
    }
    Ok(s)
}
fn optional_text(value: &Value, max: usize) -> Result<Option<String>, Invalid> {
    if value.is_null() {
        Ok(None)
    } else {
        text(value, max).map(Some)
    }
}
fn flag(value: &Value) -> Result<bool, Invalid> {
    if value.is_null() {
        Ok(false)
    } else {
        value.as_bool().ok_or(Invalid::Questions)
    }
}

impl Callback {
    pub fn parse(message: &Value, item: Option<&Value>) -> Result<Self, Invalid> {
        if serde_json::to_vec(message)
            .map_err(|_| Invalid::Payload)?
            .len()
            > PAYLOAD_LIMIT
        {
            return Err(Invalid::Payload);
        }
        let rpc_id = message.get("id").ok_or(Invalid::Identity)?.clone();
        if !(rpc_id.as_i64().is_some() || rpc_id.as_u64().is_some() || id(&rpc_id).is_ok()) {
            return Err(Invalid::Identity);
        }
        let p = message
            .get("params")
            .filter(|v| v.is_object())
            .ok_or(Invalid::Payload)?;
        let thread_id = id(&p["threadId"])?;
        let turn_id = id(&p["turnId"])?;
        let item_id = id(&p["itemId"])?;
        let mut decisions = vec![];
        let body = match message["method"].as_str() {
            Some("item/tool/requestUserInput") => {
                let qs = p["questions"]
                    .as_array()
                    .filter(|v| (1..=4).contains(&v.len()))
                    .ok_or(Invalid::Questions)?;
                let mut question_ids = HashSet::new();
                let mut questions = Vec::new();
                for q in qs {
                    let question_id = id(&q["id"])?;
                    if !question_ids.insert(question_id.clone()) {
                        return Err(Invalid::Questions);
                    }
                    let options = q["options"]
                        .as_array()
                        .filter(|v| (2..=4).contains(&v.len()))
                        .ok_or(Invalid::Questions)?;
                    let mut labels = HashSet::new();
                    let mut parsed = Vec::new();
                    for option in options {
                        let label = text(&option["label"], 4096)?;
                        if !labels.insert(label.clone()) {
                            return Err(Invalid::Questions);
                        }
                        let description = option["description"]
                            .as_str()
                            .filter(|s| s.len() <= 4096 && !s.contains('\0'))
                            .ok_or(Invalid::Questions)?
                            .to_owned();
                        parsed.push(OptionLabel { label, description });
                    }
                    questions.push(Question {
                        id: question_id,
                        header: text(&q["header"], 128)?,
                        question: text(&q["question"], 4096)?,
                        options: parsed,
                        is_other: flag(&q["isOther"])?,
                        is_secret: flag(&q["isSecret"])?,
                    });
                }
                Body::Question(questions)
            }
            Some("item/commandExecution/requestApproval") => {
                if !p["networkApprovalContext"].is_null()
                    || !p["additionalPermissions"].is_null()
                    || !p["environmentId"].is_null()
                    || p.get("kind").is_some_and(|k| k != "command")
                {
                    return Err(Invalid::Unsupported);
                }
                decisions = match p.get("availableDecisions").filter(|v| !v.is_null()) {
                    Some(v) => v
                        .as_array()
                        .ok_or(Invalid::Decision)?
                        .iter()
                        .filter_map(|v| v.as_str())
                        .filter(|d| ["accept", "decline", "cancel"].contains(d))
                        .map(str::to_owned)
                        .collect(),
                    None => ["accept", "decline", "cancel"].map(str::to_owned).to_vec(),
                };
                decisions.sort();
                decisions.dedup();
                if decisions.is_empty() {
                    return Err(Invalid::Decision);
                }
                Body::Command {
                    command: text(&p["command"], PAYLOAD_LIMIT)?,
                    cwd: text(&p["cwd"], 4096)?,
                    reason: optional_text(&p["reason"], 4096)?,
                }
            }
            Some("item/fileChange/requestApproval") => {
                if !p["grantRoot"].is_null() {
                    return Err(Invalid::Unsupported);
                }
                let item = item
                    .filter(|i| i["id"] == item_id && i["type"] == "fileChange")
                    .ok_or(Invalid::Context)?;
                if serde_json::to_vec(item)
                    .map_err(|_| Invalid::Context)?
                    .len()
                    > PAYLOAD_LIMIT
                {
                    return Err(Invalid::Context);
                }
                let changes = item["changes"]
                    .as_array()
                    .filter(|v| !v.is_empty() && v.len() <= 64)
                    .ok_or(Invalid::Context)?;
                for change in changes {
                    text(&change["path"], 4096).map_err(|_| Invalid::Context)?;
                    if !matches!(
                        change["kind"]["type"].as_str(),
                        Some("add" | "delete" | "update")
                    ) || !change["diff"].is_string()
                    {
                        return Err(Invalid::Context);
                    }
                }
                decisions = ["accept", "decline", "cancel"].map(str::to_owned).to_vec();
                Body::Files {
                    changes: item["changes"].clone(),
                    reason: optional_text(&p["reason"], 4096)?,
                }
            }
            _ => return Err(Invalid::Unsupported),
        };
        Ok(Self {
            rpc_id,
            thread_id,
            turn_id,
            item_id,
            body,
            decisions,
        })
    }

    pub fn approval(&self, decision: &str) -> Result<Value, Invalid> {
        if !self.decisions.iter().any(|d| d == decision) {
            return Err(Invalid::Decision);
        }
        Ok(json!({"id":self.rpc_id,"result":{"decision":decision}}))
    }

    pub fn answer(&self, answers: &Value) -> Result<Value, Invalid> {
        let Body::Question(questions) = &self.body else {
            return Err(Invalid::Decision);
        };
        let answers = answers
            .as_object()
            .filter(|a| a.len() == questions.len())
            .ok_or(Invalid::Decision)?;
        let mut result = serde_json::Map::new();
        for question in questions {
            let answer = text(answers.get(&question.id).ok_or(Invalid::Decision)?, 4096)
                .map_err(|_| Invalid::Decision)?;
            if !question.is_other && !question.options.iter().any(|o| o.label == answer) {
                return Err(Invalid::Decision);
            }
            result.insert(question.id.clone(), json!({"answers":[answer]}));
        }
        Ok(json!({"id":self.rpc_id,"result":{"answers":result}}))
    }
}

#[cfg(test)]
mod tests;
