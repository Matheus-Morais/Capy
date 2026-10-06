use super::*;
use serde_json::json;

fn sample(id: &str) -> crate::demo::Session {
    crate::demo::Session {
        id: format!("codex:{id}"),
        project: "Capy".into(),
        agent: "Codex".into(),
        symbol: "O".into(),
        kind: "codex".into(),
        origin: "C:\\work\\Capy".into(),
        state: "unknown".into(),
        request: None,
        message: String::new(),
        command: None,
        hidden: false,
        source_action: None,
    }
}
fn observation(id: &str, status: serde_json::Value) -> Observation {
    Observation {
        id: id.into(),
        cwd: "C:\\work\\Capy".into(),
        status,
    }
}
#[test]
fn runtime_status_mapping() {
    for (status, expected) in [
        (json!({"type":"active","activeFlags":[]}), "working"),
        (
            json!({"type":"active","activeFlags":["waitingOnApproval"]}),
            "waiting",
        ),
        (
            json!({"type":"active","activeFlags":["waitingOnUserInput"]}),
            "waiting",
        ),
        (
            json!({"type":"active","activeFlags":["waitingOnApproval","waitingOnUserInput"]}),
            "waiting",
        ),
        (json!({"type":"idle"}), "idle"),
        (json!({"type":"notLoaded"}), "unknown"),
        (json!({"type":"systemError"}), "unknown"),
        (json!({"type":"newType"}), "unknown"),
        (
            json!({"type":"active","activeFlags":["newFlag"]}),
            "unknown",
        ),
        (
            json!({"type":"active","activeFlags":["waitingOnApproval","newFlag"]}),
            "unknown",
        ),
        (json!({"type":"active"}), "unknown"),
        (
            json!({"type":"active","activeFlags":"waitingOnApproval"}),
            "unknown",
        ),
        (json!(null), "unknown"),
    ] {
        assert_eq!(runtime_state(&status).0, expected, "{status}");
    }
}
#[test]
fn only_matching_live_threads_are_enriched() {
    let mut report = crate::discovery::Report {
        sessions: vec![sample("live"), sample("other")],
        integrations: vec![],
    };
    let mut wrong = observation("other", json!({"type":"idle"}));
    wrong.cwd = "C:\\another".into();
    apply(
        &mut report,
        Ok(vec![
            observation("live", json!({"type":"active","activeFlags":[]})),
            wrong,
            observation("history", json!({"type":"idle"})),
        ]),
    );
    assert_eq!(report.sessions.len(), 2);
    assert_eq!(report.sessions[0].state, "working");
    assert_eq!(report.sessions[1].state, "unknown");
}
#[test]
fn waiting_is_replaced_on_every_observation() {
    let mut report = crate::discovery::Report {
        sessions: vec![sample("live")],
        integrations: vec![],
    };
    for next in [
        Some(json!({"type":"active","activeFlags":[]})),
        Some(json!({"type":"idle"})),
        None,
    ] {
        apply(
            &mut report,
            Ok(vec![observation(
                "live",
                json!({"type":"active","activeFlags":["waitingOnApproval"]}),
            )]),
        );
        assert_eq!(report.sessions[0].state, "waiting");
        apply(
            &mut report,
            Ok(next.into_iter().map(|s| observation("live", s)).collect()),
        );
        assert_ne!(report.sessions[0].state, "waiting");
        assert!(report.sessions[0].request.is_none());
    }
    apply(
        &mut report,
        Ok(vec![observation(
            "live",
            json!({"type":"active","activeFlags":["waitingOnUserInput"]}),
        )]),
    );
    apply(&mut report, Err(()));
    assert_eq!(report.sessions[0].state, "unknown");
    assert!(report.sessions[0].request.is_none());
    assert!(report.sessions[0].command.is_none());
}

#[test]
fn waiting_recovers_after_connection_failure() {
    let mut report = crate::discovery::Report {
        sessions: vec![sample("live")],
        integrations: vec![],
    };
    let waiting = || {
        Ok(vec![observation(
            "live",
            json!({"type":"active","activeFlags":["waitingOnUserInput"]}),
        )])
    };
    apply(&mut report, waiting());
    assert_eq!(report.sessions[0].state, "waiting");
    apply(&mut report, Err(()));
    assert_eq!(report.sessions[0].state, "unknown");
    assert!(report.sessions[0].request.is_none());
    assert!(report.sessions[0].command.is_none());
    apply(&mut report, waiting());
    assert_eq!(report.sessions[0].state, "waiting");
    apply(
        &mut report,
        Ok(vec![observation("live", json!({"type":"idle"}))]),
    );
    assert_eq!(report.sessions[0].state, "idle");
    assert!(report.sessions[0].request.is_none());
    assert!(report.sessions[0].command.is_none());
}

#[test]
fn session_limit_bounds_reads() {
    use std::net::{TcpListener, TcpStream};
    use std::time::Duration;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let ids: Vec<String> = (0..65).map(|i| format!("session-{i}")).collect();
    let loaded_ids = ids.clone();
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut socket = tungstenite::accept(stream).unwrap();
        let mut reads = Vec::new();
        for _ in 0..67 {
            let request: serde_json::Value =
                serde_json::from_str(socket.read().unwrap().to_text().unwrap()).unwrap();
            let result = match request["method"].as_str().unwrap() {
                "initialize" => json!({}),
                "initialized" => continue,
                "thread/loaded/list" => json!({"data":loaded_ids}),
                "thread/read" => {
                    let id = request["params"]["threadId"].as_str().unwrap();
                    assert_eq!(request["params"]["includeTurns"], false);
                    reads.push(id.to_owned());
                    json!({"thread":{"id":id,"cwd":"C:\\work\\Capy","status":{"type":"idle"}}})
                }
                other => panic!("unexpected method {other}"),
            };
            socket
                .send(tungstenite::Message::Text(
                    json!({"id":request["id"],"result":result})
                        .to_string()
                        .into(),
                ))
                .unwrap();
        }
        assert_eq!(reads, loaded_ids[..64]);
        assert!(socket.read().is_err(), "A 65th read must not be sent");
    });
    let stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let found = protocol::observe(stream, &ids).unwrap();
    assert_eq!(found.len(), 64);
    assert_eq!(found.last().unwrap().id, "session-63");
    server.join().unwrap();
}

#[test]
fn oversized_websocket_message_is_rejected() {
    use std::net::{TcpListener, TcpStream};
    use std::time::Duration;
    for size in [1_048_576, 1_048_577] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut socket = tungstenite::accept(stream).unwrap();
            let request: serde_json::Value =
                serde_json::from_str(socket.read().unwrap().to_text().unwrap()).unwrap();
            let mut response = json!({"id":request["id"],"result":{},"padding":""});
            let overhead = response.to_string().len();
            response["padding"] = json!("x".repeat(size - overhead));
            let payload = response.to_string();
            assert_eq!(payload.len(), size);
            let _ = socket.send(tungstenite::Message::Text(payload.into()));
            if size == 1_048_576 {
                let initialized: serde_json::Value =
                    serde_json::from_str(socket.read().unwrap().to_text().unwrap()).unwrap();
                assert_eq!(initialized["method"], "initialized");
                let loaded: serde_json::Value =
                    serde_json::from_str(socket.read().unwrap().to_text().unwrap()).unwrap();
                assert_eq!(loaded["method"], "thread/loaded/list");
                socket
                    .send(tungstenite::Message::Text(
                        json!({"id":loaded["id"],"result":{"data":[]}})
                            .to_string()
                            .into(),
                    ))
                    .unwrap();
            }
        });
        let stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let result = protocol::observe(stream, &[]);
        assert_eq!(result.is_ok(), size == 1_048_576, "message size {size}");
        server.join().unwrap();
    }
}

#[test]
fn websocket_probe_is_read_only() {
    use std::net::{TcpListener, TcpStream};
    use std::time::Duration;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut socket = tungstenite::accept(stream).unwrap();
        let mut methods = Vec::new();
        for _ in 0..4 {
            let message: serde_json::Value =
                serde_json::from_str(socket.read().unwrap().to_text().unwrap()).unwrap();
            let method = message["method"].as_str().unwrap();
            methods.push(method.to_owned());
            let result = match method {
                "initialize" => json!({}),
                "initialized" => continue,
                "thread/loaded/list" => json!({"data":["live"]}),
                "thread/read" => {
                    assert_eq!(message["params"]["threadId"], "live");
                    assert_eq!(message["params"]["includeTurns"], false);
                    json!({"thread":{"id":"live","cwd":"C:\\work\\Capy","status":{"type":"active","activeFlags":[]}}})
                }
                other => panic!("unexpected method {other}"),
            };
            socket
                .send(tungstenite::Message::Text(
                    json!({"id":message["id"],"result":result})
                        .to_string()
                        .into(),
                ))
                .unwrap();
        }
        assert_eq!(
            methods,
            [
                "initialize",
                "initialized",
                "thread/loaded/list",
                "thread/read"
            ]
        );
    });
    let stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let found = protocol::observe(stream, &["live".into(), "absent".into()]).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, "live");
    assert_eq!(runtime_state(&found[0].status).0, "working");
    server.join().unwrap();
}

#[cfg(windows)]
#[test]
fn unresponsive_child_is_reaped() {
    use std::os::windows::process::CommandExt;
    use std::{
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let mut child = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Start-Sleep -Seconds 30",
        ])
        .creation_flags(0x08000000)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let started = Instant::now();
    assert!(
        proxy::query_child(&mut child, vec!["live".into()], Duration::from_millis(200)).is_err()
    );
    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(child.try_wait().unwrap().is_some());
}
