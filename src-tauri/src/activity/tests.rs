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
