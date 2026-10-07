use serde_json::Value;
use std::{collections::{BTreeMap, BTreeSet}, io::{BufRead, BufReader, Read, Seek, SeekFrom}, path::Path};

const HISTORY_LIMIT: u64 = 32 * 1024 * 1024;
const TEXT_LIMIT: usize = 8_000;

#[derive(Default)]
pub struct Context {
    pub answers: Vec<String>,
    pub instructions: Vec<String>,
    pub guides: BTreeMap<String, String>,
    pub commands: Vec<CommandEvidence>,
    pub edits: BTreeSet<String>,
    pub partial: bool,
}

pub struct CommandEvidence {
    pub command: String,
    pub result: Option<String>,
    pub failed: Option<bool>,
}

enum Pending {
    Guide(String),
    Command(usize),
}

pub fn excerpt(text: &str, limit: usize) -> String {
    if text.len() <= limit { return text.into(); }
    let mut end = limit;
    while !text.is_char_boundary(end) { end -= 1; }
    format!("{}\n[Trecho limitado; consulte o histórico original para o conteúdo integral.]", &text[..end])
}

fn content_text(value: &Value) -> String {
    if let Some(text) = value.as_str() { return text.into(); }
    value.as_array().map(|blocks| blocks.iter().filter_map(|block| block["text"].as_str()).collect::<Vec<_>>().join("\n")).unwrap_or_default()
}

fn remember(list: &mut Vec<String>, text: &str, context_partial: &mut bool) {
    if text.trim().is_empty() { return; }
    if list.len() == 24 { list.remove(0); *context_partial = true; }
    if text.len() > TEXT_LIMIT { *context_partial = true; }
    list.push(excerpt(text, TEXT_LIMIT));
}

pub fn parse(reader: impl Read) -> Context {
    let mut context = Context::default();
    let mut pending = BTreeMap::<String, Pending>::new();
    let mut total = 0usize;
    for line in BufReader::new(reader.take(HISTORY_LIMIT + 1)).lines() {
        let Ok(line) = line else { context.partial = true; break; };
        total += line.len() + 1;
        if total as u64 > HISTORY_LIMIT { context.partial = true; break; }
        if line.len() > 1_048_576 { context.partial = true; continue; }
        let Ok(value) = serde_json::from_str::<Value>(&line) else { context.partial = true; continue; };
        let role = value["type"].as_str().unwrap_or("");
        // Sidechain/subagent transcripts are not the main session's decisions.
        if value["isSidechain"] == true { continue; }
        let content = &value["message"]["content"];
        if let Some(text) = content.as_str() {
            if role == "user" { remember(&mut context.instructions, text, &mut context.partial); }
            else if role == "assistant" { remember(&mut context.answers, text, &mut context.partial); }
        }
        for block in content.as_array().into_iter().flatten() {
            match (role, block["type"].as_str().unwrap_or("")) {
                ("assistant", "text") => {
                    if let Some(text) = block["text"].as_str() { remember(&mut context.answers, text, &mut context.partial); }
                }
                ("user", "text") => {
                    if let Some(text) = block["text"].as_str() { remember(&mut context.instructions, text, &mut context.partial); }
                }
                ("assistant", "tool_use") => {
                    let name = block["name"].as_str().unwrap_or("");
                    let input = &block["input"];
                    let id = block["id"].as_str().unwrap_or("");
                    if name == "Read" {
                        if let Some(path) = input["file_path"].as_str().filter(|p|p.to_ascii_lowercase().ends_with(".md")) {
                            if context.guides.len() < 64 || context.guides.contains_key(path) {
                                context.guides.entry(path.into()).or_default();
                                pending.insert(id.into(), Pending::Guide(path.into()));
                            } else { context.partial = true; }
                        }
                    }
                    if matches!(name, "Bash" | "PowerShell") {
                        if let Some(command) = input["command"].as_str() {
                            if context.commands.len() == 128 {
                                context.commands.remove(0);
                                pending.retain(|_, item| match item {
                                    Pending::Command(index) if *index == 0 => false,
                                    Pending::Command(index) => { *index -= 1; true },
                                    _ => true,
                                });
                                context.partial = true;
                            }
                            let index = context.commands.len();
                            context.commands.push(CommandEvidence {command: excerpt(command, 2_000), result: None, failed: None});
                            pending.insert(id.into(), Pending::Command(index));
                        }
                    }
                    if matches!(name, "Edit" | "Write" | "NotebookEdit") {
                        if let Some(path) = input["file_path"].as_str().or_else(||input["notebook_path"].as_str()) {
                            if context.edits.len() < 256 { context.edits.insert(path.into()); } else { context.partial = true; }
                        }
                    }
                    if pending.len() > 256 { pending.clear(); context.partial = true; }
                }
                ("user", "tool_result") => {
                    let id = block["tool_use_id"].as_str().unwrap_or("");
                    let text = content_text(&block["content"]);
                    let failed = block["is_error"].as_bool().unwrap_or(false);
                    match pending.remove(id) {
                        Some(Pending::Guide(path)) => {
                            // Use what the agent actually read, including an error; do not substitute today's file.
                            let entry = context.guides.get_mut(&path).unwrap();
                            if !entry.is_empty() { entry.push_str("\n\n"); }
                            let prefix = if failed { "Leitura falhou:\n" } else { "Trecho efetivamente recebido pelo agente:\n" };
                            *entry = excerpt(&format!("{entry}{prefix}{text}"), 8_000);
                            if text.len() > 8_000 { context.partial = true; }
                        }
                        Some(Pending::Command(index)) => {
                            if let Some(command) = context.commands.get_mut(index) {
                                command.result = Some(excerpt(&text, 4_000));
                                command.failed = Some(failed);
                                if text.len() > 4_000 { context.partial = true; }
                            }
                        }
                        _ => {},
                    }
                }
                _ => {},
            }
        }
    }
    context
}

pub fn load(path: &Path) -> Result<Context, String> {
    let mut file = std::fs::File::open(path).map_err(|_|"O histórico desta sessão está indisponível. Não foi possível preparar um resumo comprovável.".to_owned())?;
    let size = file.metadata().map_err(|e|e.to_string())?.len();
    let cut = size > HISTORY_LIMIT;
    if cut { file.seek(SeekFrom::Start(size - HISTORY_LIMIT)).map_err(|e|e.to_string())?; }
    let mut reader = BufReader::new(file);
    if cut {
        let mut first = Vec::new();
        reader.read_until(b'\n', &mut first).map_err(|e|e.to_string())?;
    }
    let mut context = parse(reader);
    context.partial |= cut;
    Ok(context)
}

pub fn test_command(command: &str) -> bool {
    ["cargo test", "npm test", "npm run test", "pytest", "dotnet test", "vitest", "jest", "go test", "node --test", "pnpm test", "bun test", "mvn test", "gradle test"].iter().any(|needle|command.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn handoff_links_actual_test_results_and_read_rules_by_tool_id() {
        let rows = [
            serde_json::json!({"type":"user","message":{"content":"Continue: keep API compatibility and implement retries next."}}),
            serde_json::json!({"type":"assistant","message":{"content":[
                {"type":"tool_use","id":"guide","name":"Read","input":{"file_path":".design/plan.md"}},
                {"type":"tool_use","id":"test","name":"Bash","input":{"command":"npm test"}},
                {"type":"tool_use","id":"edit","name":"Edit","input":{"file_path":"src/parser.ts"}},
                {"type":"text","text":"Decision: retain the public API. Next: retry the failed test after fixing timeout."}
            ]}}),
            serde_json::json!({"type":"user","message":{"content":[
                {"type":"tool_result","tool_use_id":"test","is_error":true,"content":"FAIL retry timeout; 2 passed, 1 failed"},
                {"type":"tool_result","tool_use_id":"guide","content":[{"type":"text","text":"# Contract\nNever retry a payment without an idempotency key."}]}
            ]}}),
        ];
        let input=rows.iter().map(Value::to_string).collect::<Vec<_>>().join("\n");
        let context=parse(input.as_bytes());
        assert!(context.instructions[0].contains("retries next"));
        assert!(context.answers[0].contains("retain the public API"));
        assert!(context.guides[".design/plan.md"].contains("idempotency key"));
        assert_eq!(context.commands[0].failed,Some(true));
        assert!(context.commands[0].result.as_ref().unwrap().contains("1 failed"));
        assert!(context.edits.contains("src/parser.ts"));
        assert!(!context.partial);
    }
    #[test]
    fn handoff_missing_results_are_unknown_and_sidechains_do_not_set_next_steps() {
        let input=concat!(
            "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"tool_use\",\"id\":\"t\",\"name\":\"Bash\",\"input\":{\"command\":\"node --test tests/parser.test.mjs\"}}]}}\n",
            "{\"type\":\"assistant\",\"isSidechain\":true,\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"Everything is done\"}]}}\n");
        let context=parse(input.as_bytes());
        assert!(test_command(&context.commands[0].command));
        assert!(context.commands[0].result.is_none());
        assert!(context.answers.is_empty());
    }
    #[test]
    fn handoff_large_history_keeps_latest_turn_and_reports_partial() {
        let path=std::env::temp_dir().join(format!("capy-history-{}.jsonl",uuid::Uuid::new_v4()));
        let mut file=std::fs::File::create(&path).unwrap();
        use std::io::Write;
        for _ in 0..34 {file.write_all(&vec![b' ';1_048_576]).unwrap();file.write_all(b"\n").unwrap();}
        file.write_all(b"{\"type\":\"assistant\",\"message\":{\"content\":\"Latest next step: finish native QA\"}}\n").unwrap();drop(file);
        let context=load(&path).unwrap();
        assert!(context.partial);assert!(context.answers.last().unwrap().contains("finish native QA"));
        std::fs::remove_file(path).unwrap();
        assert!(excerpt(&"ç".repeat(8_000),7_999).ends_with("integral.]"));
    }
}
