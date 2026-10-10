use serde_json::{json, Value};
use std::collections::HashSet;

const PAYLOAD_LIMIT: usize = 65_536;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Invalid {
    Payload,
    Identity,
    Unsupported,
    Questions,
    Decision,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Event {
    Permission,
    Question,
}

#[derive(Clone, Debug)]
pub struct Request {
    pub session_id: String,
    pub cwd: String,
    pub tool_use_id: Option<String>,
    pub tool_name: String,
    pub body: Value,
    pub decisions: Vec<String>,
    input: Value,
    event: Event,
}

fn text(value: &Value, limit: usize) -> Result<String, Invalid> {
    value
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= limit && !s.contains('\0'))
        .map(str::to_owned)
        .ok_or(Invalid::Payload)
}

fn identity(value: &Value) -> Result<String, Invalid> {
    let value = text(value, 256).map_err(|_| Invalid::Identity)?;
    if value.chars().any(char::is_control) {
        return Err(Invalid::Identity);
    }
    Ok(value)
}

fn bounded(value: &Value) -> Result<(), Invalid> {
    if serde_json::to_vec(value)
        .map_err(|_| Invalid::Payload)?
        .len()
        > PAYLOAD_LIMIT
    {
        return Err(Invalid::Payload);
    }
    Ok(())
}

impl Request {
    pub fn parse(message: &Value) -> Result<Self, Invalid> {
        bounded(message)?;
        if !message.is_object() {
            return Err(Invalid::Payload);
        }
        if message
            .get("agent_id")
            .is_some_and(|v| !v.is_null() && v != "")
        {
            return Err(Invalid::Unsupported);
        }
        let session_id = identity(&message["session_id"])?;
        let cwd = text(&message["cwd"], 4096)?;
        let tool_name = identity(&message["tool_name"])?;
        let input = message
            .get("tool_input")
            .filter(|v| v.is_object())
            .ok_or(Invalid::Payload)?
            .clone();
        let tool_use_id = match message.get("tool_use_id") {
            None | Some(Value::Null) => None,
            Some(value) => Some(identity(value)?),
        };
        let (event, body) = match message["hook_event_name"].as_str() {
            Some("PermissionRequest") if tool_name != "AskUserQuestion" => (
                Event::Permission,
                json!({"kind":"claudePermission","toolName":tool_name,"toolInput":input,"cwd":cwd}),
            ),
            Some("PreToolUse") if tool_name == "AskUserQuestion" => {
                if tool_use_id.is_none() {
                    return Err(Invalid::Identity);
                }
                let questions = input["questions"]
                    .as_array()
                    .filter(|qs| (1..=4).contains(&qs.len()))
                    .ok_or(Invalid::Questions)?;
                let mut seen = HashSet::new();
                let mut views = Vec::new();
                for q in questions {
                    let question = text(&q["question"], 4096).map_err(|_| Invalid::Questions)?;
                    if !seen.insert(question.clone()) {
                        return Err(Invalid::Questions);
                    }
                    let header = text(&q["header"], 128).map_err(|_| Invalid::Questions)?;
                    if header.chars().count() > 12 {
                        return Err(Invalid::Questions);
                    }
                    let multi = q["multiSelect"].as_bool().ok_or(Invalid::Questions)?;
                    let options = q["options"]
                        .as_array()
                        .filter(|os| (2..=4).contains(&os.len()))
                        .ok_or(Invalid::Questions)?;
                    let mut labels = HashSet::new();
                    let mut option_views = Vec::new();
                    for option in options {
                        let label = text(&option["label"], 4096).map_err(|_| Invalid::Questions)?;
                        if !labels.insert(label.clone()) {
                            return Err(Invalid::Questions);
                        }
                        let description = option["description"]
                            .as_str()
                            .filter(|s| s.len() <= 4096 && !s.contains('\0'))
                            .ok_or(Invalid::Questions)?;
                        option_views.push(json!({"label":label,"description":description}));
                    }
                    views.push(json!({"id":question,"question":question,"header":header,
                        "options":option_views,"multiSelect":multi,"isOther":false,"allowFreeText":true,"isSecret":false}));
                }
                (
                    Event::Question,
                    json!({"kind":"question","questions":views}),
                )
            }
            _ => return Err(Invalid::Unsupported),
        };
        Ok(Self {
            session_id,
            cwd,
            tool_use_id,
            tool_name,
            body,
            decisions: ["allow", "deny", "cancel"].map(str::to_owned).to_vec(),
            input,
            event,
        })
    }

    pub fn approval(&self, decision: &str) -> Result<Value, Invalid> {
        if !self.decisions.iter().any(|d| d == decision)
            || (self.event == Event::Question && decision == "allow")
        {
            return Err(Invalid::Decision);
        }
        let message = if decision == "cancel" {
            "User cancelled the request"
        } else {
            "User denied the request"
        };
        Ok(match self.event {
            Event::Permission if decision == "allow" => json!({"hookSpecificOutput":{
                "hookEventName":"PermissionRequest","decision":{"behavior":"allow"}}}),
            Event::Permission => json!({"hookSpecificOutput":{"hookEventName":"PermissionRequest",
                "decision":{"behavior":"deny","message":message,"interrupt":decision == "cancel"}}}),
            Event::Question => json!({"hookSpecificOutput":{"hookEventName":"PreToolUse",
                "permissionDecision":"deny","permissionDecisionReason":message}}),
        })
    }

    pub fn answer(&self, answers: &Value) -> Result<Value, Invalid> {
        if self.event != Event::Question {
            return Err(Invalid::Decision);
        }
        bounded(answers).map_err(|_| Invalid::Decision)?;
        let questions = self.input["questions"]
            .as_array()
            .ok_or(Invalid::Questions)?;
        let answers = answers
            .as_object()
            .filter(|a| a.len() == questions.len())
            .ok_or(Invalid::Decision)?;
        let mut wire = serde_json::Map::new();
        for q in questions {
            let key = q["question"].as_str().ok_or(Invalid::Questions)?;
            let value = answers.get(key).ok_or(Invalid::Decision)?;
            let answer = if let Some(selected) = value.as_array() {
                if q["multiSelect"] != true || selected.is_empty() || selected.len() > 4 {
                    return Err(Invalid::Decision);
                }
                let mut seen = HashSet::new();
                let mut labels = Vec::new();
                for label in selected {
                    let label = text(label, 4096).map_err(|_| Invalid::Decision)?;
                    if !seen.insert(label.clone())
                        || !q["options"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|o| o["label"] == label)
                    {
                        return Err(Invalid::Decision);
                    }
                    labels.push(label);
                }
                labels.join(", ")
            } else {
                text(value, 4096).map_err(|_| Invalid::Decision)?
            };
            if answer.len() > 4096 {
                return Err(Invalid::Decision);
            }
            wire.insert(key.to_owned(), Value::String(answer));
        }
        let mut updated = self.input.clone();
        updated["answers"] = Value::Object(wire);
        // A general response overrides per-question answers in Claude Code.
        updated.as_object_mut().unwrap().remove("response");
        let output = json!({"hookSpecificOutput":{"hookEventName":"PreToolUse",
            "permissionDecision":"allow","updatedInput":updated}});
        bounded(&output).map_err(|_| Invalid::Decision)?;
        Ok(output)
    }

    pub fn response(&self, response: &Value) -> Result<Value, Invalid> {
        let object = response
            .as_object()
            .filter(|o| o.len() == 1)
            .ok_or(Invalid::Decision)?;
        if let Some(decision) = object.get("decision") {
            self.approval(decision.as_str().ok_or(Invalid::Decision)?)
        } else if let Some(answers) = object.get("answers") {
            self.answer(answers)
        } else {
            Err(Invalid::Decision)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn question(multi: bool) -> Value {
        json!({"session_id":"session-a","cwd":"C:\\project","tool_name":"AskUserQuestion",
            "hook_event_name":"PreToolUse","tool_use_id":"tool-a","tool_input":{
                "questions":[{"question":"Which?","header":"Choice","multiSelect":multi,
                    "options":[{"label":"A","description":"First"},{"label":"B","description":"Second"}]}]}})
    }
    #[test]
    fn claude_intervention_protocol_answers_preserve_input() {
        let mut input = question(true);
        input["tool_input"]["metadata"] = json!({"source":"original"});
        let request = Request::parse(&input).unwrap();
        let output = request
            .response(&json!({"answers":{"Which?":["A","B"]}}))
            .unwrap();
        let updated = &output["hookSpecificOutput"]["updatedInput"];
        assert_eq!(updated["answers"]["Which?"], "A, B");
        assert_eq!(updated["questions"], input["tool_input"]["questions"]);
        assert_eq!(updated["metadata"], input["tool_input"]["metadata"]);
        assert_eq!(
            request.answer(&json!({"Which?":"custom text"})).unwrap()["hookSpecificOutput"]
                ["updatedInput"]["answers"]["Which?"],
            "custom text"
        );
    }
    #[test]
    fn claude_intervention_protocol_rejects_invalid_answers() {
        let request = Request::parse(&question(false)).unwrap();
        for value in [
            json!({}),
            json!({"Which?":["A"]}),
            json!({"Which?":""}),
            json!({"Which?":["A","B"]}),
            json!({"Which?":"A","extra":"B"}),
        ] {
            assert_eq!(request.answer(&value), Err(Invalid::Decision));
        }
        assert_eq!(request.approval("allow"), Err(Invalid::Decision));
        assert_eq!(
            request.response(&json!({"decision":"deny","answers":{}})),
            Err(Invalid::Decision)
        );
        let request = Request::parse(&question(true)).unwrap();
        for value in [
            json!({"Which?":["A","A"]}),
            json!({"Which?":["unknown","B"]}),
            json!({"Which?":[]}),
        ] {
            assert_eq!(request.answer(&value), Err(Invalid::Decision));
        }
    }
    #[test]
    fn claude_intervention_protocol_permission_decisions() {
        let input = json!({"session_id":"session-b","cwd":"C:\\project","hook_event_name":"PermissionRequest",
            "tool_name":"GenericTool","tool_input":{"full":"context"}});
        let request = Request::parse(&input).unwrap();
        assert_eq!(request.tool_use_id, None);
        assert_eq!(request.body["toolInput"], input["tool_input"]);
        assert_eq!(
            request.approval("allow").unwrap(),
            json!({"hookSpecificOutput":{
            "hookEventName":"PermissionRequest","decision":{"behavior":"allow"}}})
        );
        assert_eq!(
            request.approval("cancel").unwrap()["hookSpecificOutput"]["decision"]["interrupt"],
            true
        );
        assert_eq!(
            request.approval("deny").unwrap()["hookSpecificOutput"]["decision"]["behavior"],
            "deny"
        );
        assert_eq!(request.approval("accept"), Err(Invalid::Decision));
        assert_eq!(request.answer(&json!({})), Err(Invalid::Decision));
    }
    #[test]
    fn claude_intervention_protocol_rejects_malformed_requests() {
        let mut input = question(false);
        input.as_object_mut().unwrap().remove("tool_use_id");
        assert!(matches!(Request::parse(&input), Err(Invalid::Identity)));
        let mut input = question(false);
        input["agent_id"] = json!("subagent");
        assert!(matches!(Request::parse(&input), Err(Invalid::Unsupported)));
        let mut input = question(false);
        input["tool_input"]["questions"][0]["header"] = json!("More than twelve");
        assert!(matches!(Request::parse(&input), Err(Invalid::Questions)));
        let mut input = question(false);
        let duplicate = input["tool_input"]["questions"][0].clone();
        input["tool_input"]["questions"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(matches!(Request::parse(&input), Err(Invalid::Questions)));
        let mut input = question(false);
        input["tool_input"] = json!([]);
        assert!(matches!(Request::parse(&input), Err(Invalid::Payload)));
        let mut input = question(false);
        input["oversized"] = json!("x".repeat(PAYLOAD_LIMIT));
        assert!(matches!(Request::parse(&input), Err(Invalid::Payload)));
    }
}
