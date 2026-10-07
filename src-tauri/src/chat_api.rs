use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{io::Read, time::Duration};

#[derive(Clone, Deserialize, Serialize)]
pub struct Message {
    pub role: String,
    pub text: String,
}

pub struct Reply {
    pub text: String,
    pub completed: bool,
    pub note: Option<String>,
}

struct Contract {
    endpoint: String,
    body: Value,
}

pub fn valid_model(model: &str) -> bool {
    !model.is_empty() && model.len() <= 128
        && model.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b))
        && !model.starts_with('-')
}

fn contract(provider: &str, model: &str, messages: &[Message]) -> Result<Contract, String> {
    if !valid_model(model) || messages.is_empty() || messages.len() > 200
        || messages.iter().any(|m| !["user", "assistant"].contains(&m.role.as_str()) || m.text.trim().is_empty())
        || messages.iter().map(|m| m.text.len()).sum::<usize>() > 196_608 {
        return Err("Modelo ou histórico inválido; revise a conversa antes de enviar.".into());
    }
    let ordinary: Vec<Value> = messages.iter().map(|m| json!({"role":m.role,"content":m.text})).collect();
    let (endpoint, body) = match provider {
        "OpenAI" => ("https://api.openai.com/v1/responses".into(),
            json!({"model":model,"input":ordinary,"store":false,"max_output_tokens":4096})),
        "Anthropic" => ("https://api.anthropic.com/v1/messages".into(),
            json!({"model":model,"messages":ordinary,"max_tokens":4096})),
        "Gemini" => {
            let contents: Vec<Value> = messages.iter().map(|m| json!({
                "role":if m.role=="assistant" {"model"} else {"user"},"parts":[{"text":m.text}]
            })).collect();
            (format!("https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent"),
                json!({"contents":contents,"generationConfig":{"maxOutputTokens":4096}}))
        }
        _ => return Err("Provedor de API não suportado.".into()),
    };
    Ok(Contract { endpoint, body })
}

pub fn send(provider: &str, model: &str, key: &str, messages: &[Message]) -> Result<Reply, String> {
    let contract = contract(provider, model, messages)?;
    send_contract(provider, key, &contract)
}

fn send_contract(provider: &str, key: &str, contract: &Contract) -> Result<Reply, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(180))
        .connect_timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .build().map_err(|_| "Não foi possível preparar a conexão HTTPS.")?;
    let request = client.post(&contract.endpoint).json(&contract.body);
    let request = match provider {
        "OpenAI" => request.bearer_auth(key),
        "Anthropic" => request.header("x-api-key", key).header("anthropic-version", "2023-06-01"),
        "Gemini" => request.header("x-goog-api-key", key),
        _ => return Err("Provedor de API não suportado.".into()),
    };
    let response = request.send().map_err(|_| "A conexão falhou ou expirou. O consumo pode ter ocorrido; nenhuma mensagem foi reenviada.")?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("{provider} retornou HTTP {}. Confira a conta e o modelo; nenhuma troca ou reenvio foi feito.", status.as_u16()));
    }
    let mut bytes = Vec::new();
    response.take(2_097_153).read_to_end(&mut bytes).map_err(|_| "A resposta foi interrompida; o consumo pode ter ocorrido.")?;
    if bytes.len() > 2_097_152 { return Err("Resposta excedeu o limite de 2 MiB; nenhum reenvio foi feito.".into()); }
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| "O provedor retornou uma resposta incompatível; nenhum reenvio foi feito.")?;
    parse(provider, &value)
}

fn parse(provider: &str, value: &Value) -> Result<Reply, String> {
    let mut text = Vec::new();
    let mut blocked = false;
    let completed = match provider {
        "OpenAI" => {
            if let Some(output) = value["output"].as_array() {
                for item in output {
                    if item["type"] != "message" || item["role"] != "assistant" { continue; }
                    if let Some(content) = item["content"].as_array() {
                        for part in content {
                            if part["type"] == "output_text" {
                                if let Some(s) = part["text"].as_str() { text.push(s); }
                            } else if part["type"] == "refusal" {
                                blocked = true;
                                if let Some(s) = part["refusal"].as_str() { text.push(s); }
                            }
                        }
                    }
                }
            }
            value["status"] == "completed" && value["error"].is_null()
        }
        "Anthropic" => {
            if value["type"] != "message" || value["role"] != "assistant" { return Err("Resposta Anthropic incompatível.".into()); }
            if let Some(content) = value["content"].as_array() {
                for part in content { if part["type"] == "text" { if let Some(s) = part["text"].as_str() { text.push(s); } } }
            }
            matches!(value["stop_reason"].as_str(), Some("end_turn" | "stop_sequence"))
        }
        "Gemini" => {
            let candidate = &value["candidates"][0];
            if let Some(parts) = candidate["content"]["parts"].as_array() {
                for part in parts {
                    if part["thought"] == true { continue; }
                    if let Some(s) = part["text"].as_str() { text.push(s); }
                }
            }
            blocked = value["promptFeedback"]["blockReason"].is_string()
                || matches!(candidate["finishReason"].as_str(), Some("SAFETY" | "RECITATION" | "BLOCKLIST" | "PROHIBITED_CONTENT"));
            candidate["finishReason"] == "STOP"
        }
        _ => return Err("Provedor de API não suportado.".into()),
    };
    let text = text.join("\n");
    if text.trim().is_empty() { return Err("O provedor não retornou texto utilizável. Nenhum reenvio foi feito.".into()); }
    if text.len() > 65_536 { return Err("Texto retornado excedeu 64 KiB. A resposta não foi considerada concluída; nenhum reenvio foi feito.".into()); }
    Ok(Reply { text, completed: completed && !blocked,
        note: if blocked { Some("O provedor restringiu a resposta.".into()) }
              else if !completed { Some("Resposta parcial: o provedor não confirmou conclusão.".into()) } else { None } })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::{Read, Write}, net::TcpListener, thread, time::Duration};

    fn mock_provider(status: u16, response: &str) -> (String, thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let response = response.as_bytes().to_vec();
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
            let mut request = Vec::new();
            let mut buffer = [0u8; 4096];
            let mut body_length = None;
            loop {
                let count = stream.read(&mut buffer).unwrap_or(0);
                if count == 0 { break; }
                request.extend_from_slice(&buffer[..count]);
                if body_length.is_none() {
                    if let Some(end) = request.windows(4).position(|part| part == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&request[..end]);
                        let length = headers.lines().find_map(|line| {
                            line.to_ascii_lowercase().strip_prefix("content-length:")
                                .and_then(|value| value.trim().parse::<usize>().ok())
                        }).unwrap_or(0);
                        body_length = Some((end + 4, length));
                    }
                }
                if body_length.is_some_and(|(start, length)| request.len() >= start + length) { break; }
            }
            let reason = if status == 200 { "OK" } else { "Service Unavailable" };
            let headers = format!(
                "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                response.len()
            );
            stream.write_all(headers.as_bytes()).unwrap();
            stream.write_all(&response).unwrap();
            drop(stream);
            listener.set_nonblocking(true).unwrap();
            let mut requests = vec![String::from_utf8_lossy(&request).into_owned()];
            let deadline = std::time::Instant::now() + Duration::from_millis(300);
            while std::time::Instant::now() < deadline {
                match listener.accept() {
                    Ok((mut retry, _)) => {
                        let mut bytes = Vec::new();
                        let _ = retry.read_to_end(&mut bytes);
                        requests.push(String::from_utf8_lossy(&bytes).into_owned());
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
            requests
        });
        (format!("http://{address}/mock"), worker)
    }

    fn local_contract(endpoint: String) -> Contract {
        let history = [Message { role: "user".into(), text: "literal marker ação_日本語🦫".into() }];
        let mut contract = contract("OpenAI", "chosen-model", &history).unwrap();
        contract.endpoint = endpoint;
        contract
    }

    #[test]
    fn chat_api_http_success_preserves_identity_headers_and_literal_payload() {
        let response = r#"{"status":"completed","output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":"mock reply"}]}]}"#;
        let (endpoint, server) = mock_provider(200, response);
        let result = send_contract("OpenAI", "fixture-key", &local_contract(endpoint)).unwrap();
        let requests = server.join().unwrap();
        assert_eq!(result.text, "mock reply");
        assert!(result.completed);
        assert_eq!(requests.len(), 1);
        let (headers, body) = requests[0].split_once("\r\n\r\n").unwrap();
        assert!(headers.to_ascii_lowercase().contains("authorization: bearer fixture-key"));
        let body: Value = serde_json::from_str(body).unwrap();
        assert_eq!(body["model"], "chosen-model");
        assert_eq!(body["input"][0]["content"], "literal marker ação_日本語🦫");
    }

    #[test]
    fn chat_api_http_failure_is_single_shot_and_never_retried() {
        let (endpoint, server) = mock_provider(503, "{}");
        let error = match send_contract("OpenAI", "fixture-key", &local_contract(endpoint)) {
            Ok(_) => panic!("provider failure must remain an error"),
            Err(error) => error,
        };
        let requests = server.join().unwrap();
        assert!(error.contains("HTTP 503"));
        assert_eq!(requests.len(), 1);
        assert!(requests[0].to_ascii_lowercase().starts_with("post /mock http/1.1"));
    }

    #[test]
    fn chat_api_contracts_pin_provider_endpoint_no_fallback() {
        let history = vec![Message{role:"user".into(),text:"hello".into()},Message{role:"assistant".into(),text:"world".into()}];
        let openai = contract("OpenAI", "chosen-model", &history).unwrap();
        assert_eq!(openai.endpoint,"https://api.openai.com/v1/responses");
        assert_eq!(openai.body["store"],false);assert_eq!(openai.body["input"][1]["role"],"assistant");
        let anthropic = contract("Anthropic","chosen-model",&history).unwrap();
        assert_eq!(anthropic.endpoint,"https://api.anthropic.com/v1/messages");assert_eq!(anthropic.body["messages"][0]["content"],"hello");
        let gemini = contract("Gemini","chosen-model",&history).unwrap();
        assert_eq!(gemini.endpoint,"https://generativelanguage.googleapis.com/v1beta/models/chosen-model:generateContent");
        assert_eq!(gemini.body["contents"][1]["role"],"model");
        assert!(contract("unknown","chosen-model",&history).is_err());
        for model in ["../evil","foo?key=secret","foo/bar","-flag",""] { assert!(contract("Gemini",model,&history).is_err()); }
    }
    #[test]
    fn chat_api_completion_requires_provider_success_and_preserves_partial_text() {
        let output=json!([{"type":"message","role":"assistant","content":[{"type":"output_text","text":"done"}]}]);
        assert!(parse("OpenAI",&json!({"status":"completed","output":output})).unwrap().completed);
        let partial=parse("OpenAI",&json!({"status":"incomplete","output":output})).unwrap();
        assert_eq!(partial.text,"done");assert!(!partial.completed);assert!(partial.note.is_some());
        assert!(!parse("OpenAI",&json!({"status":"completed","output":[{"type":"message","role":"assistant","content":[{"type":"refusal","refusal":"declined"}]}]})).unwrap().completed);
        for (reason,success) in [("end_turn",true),("max_tokens",false),("tool_use",false)] {
            assert_eq!(parse("Anthropic",&json!({"type":"message","role":"assistant","content":[{"type":"text","text":"answer"}],"stop_reason":reason})).unwrap().completed,success);
        }
        for (reason,success) in [("STOP",true),("MAX_TOKENS",false),("SAFETY",false)] {
            let reply=parse("Gemini",&json!({"candidates":[{"content":{"parts":[{"text":"private","thought":true},{"text":"answer"}]},"finishReason":reason}]})).unwrap();
            assert_eq!(reply.text,"answer");assert_eq!(reply.completed,success);
        }
        assert!(parse("OpenAI",&json!({"status":"completed","output":[]})).is_err());
    }
}
