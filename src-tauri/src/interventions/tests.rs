use super::registry::{Registry, Source};
use super::*;

fn question() -> Value {
    json!({"id":77,"method":"item/tool/requestUserInput","params":{"threadId":"thread-a","turnId":"turn-a","itemId":"item-a","isBlocking":true,"questions":[{"id":"choice","header":"Path","question":"Which test path?","options":[{"label":"Continue","description":"Proceed"},{"label":"Stop","description":"Finish"}],"isOther":false,"isSecret":false}]}})
}
fn command() -> Value {
    json!({"id":"callback-1","method":"item/commandExecution/requestApproval","params":{"threadId":"thread-a","turnId":"turn-a","itemId":"command-a","command":"Write-Output '<test>'","cwd":"C:/test","reason":"Test command","kind":"command"}})
}

#[test]
fn request_contract_and_bounds() {
    let request = Callback::parse(&question(), None).unwrap();
    assert_eq!(request.rpc_id, json!(77));
    assert_eq!(
        (&*request.thread_id, &*request.turn_id, &*request.item_id),
        ("thread-a", "turn-a", "item-a")
    );
    assert_eq!(
        request.answer(&json!({"choice":"Stop"})).unwrap(),
        json!({"id":77,"result":{"answers":{"choice":{"answers":["Stop"]}}}})
    );
    for answers in [
        json!({}),
        json!({"choice":"unknown"}),
        json!({"choice":"Stop","extra":"Stop"}),
        json!({"choice":[]} ),
    ] {
        assert_eq!(request.answer(&answers), Err(Invalid::Decision));
    }
    for field in ["threadId", "turnId", "itemId"] {
        for invalid in [json!(null), json!(""), json!("a\n"), json!("a".repeat(129))] {
            let mut v = question();
            v["params"][field] = invalid;
            assert_eq!(Callback::parse(&v, None), Err(Invalid::Identity));
        }
    }
    for rpc in [
        json!(null),
        json!(true),
        json!({}),
        json!(1.5),
        json!("a".repeat(129)),
    ] {
        let mut v = question();
        v["id"] = rpc;
        assert_eq!(Callback::parse(&v, None), Err(Invalid::Identity));
    }
    let q = question()["params"]["questions"][0].clone();
    for count in [0, 1, 4, 5] {
        let mut v = question();
        v["params"]["questions"] = Value::Array(
            (0..count)
                .map(|i| {
                    let mut x = q.clone();
                    x["id"] = json!(format!("q{i}"));
                    x
                })
                .collect(),
        );
        assert_eq!(
            Callback::parse(&v, None).is_ok(),
            (1..=4).contains(&count),
            "{count}"
        );
    }
    for count in [0, 1, 2, 4, 5] {
        let mut v = question();
        v["params"]["questions"][0]["options"] = Value::Array(
            (0..count)
                .map(|i| json!({"label":format!("option{i}"),"description":""}))
                .collect(),
        );
        assert_eq!(
            Callback::parse(&v, None).is_ok(),
            (2..=4).contains(&count),
            "{count}"
        );
    }
    for len in [4096, 4097] {
        let mut v = question();
        v["params"]["questions"][0]["question"] = json!("x".repeat(len));
        assert_eq!(Callback::parse(&v, None).is_ok(), len == 4096);
    }
    let mut duplicate = question();
    duplicate["params"]["questions"] = json!([q.clone(), q]);
    assert_eq!(Callback::parse(&duplicate, None), Err(Invalid::Questions));
    let mut duplicate = question();
    duplicate["params"]["questions"][0]["options"][1]["label"] = json!("Continue");
    assert_eq!(Callback::parse(&duplicate, None), Err(Invalid::Questions));
    let mut oversized = question();
    oversized["padding"] = json!("x".repeat(65_536));
    assert_eq!(Callback::parse(&oversized, None), Err(Invalid::Payload));
    let mut other = question();
    other["method"] = json!("item/permissions/requestApproval");
    assert_eq!(Callback::parse(&other, None), Err(Invalid::Unsupported));
}

#[test]
fn approval_context_and_decisions() {
    let request = Callback::parse(&command(), None).unwrap();
    assert_eq!(
        request.body,
        Body::Command {
            command: "Write-Output '<test>'".into(),
            cwd: "C:/test".into(),
            reason: Some("Test command".into())
        }
    );
    assert_eq!(
        request.approval("decline").unwrap(),
        json!({"id":"callback-1","result":{"decision":"decline"}})
    );
    for decision in [
        "acceptForSession",
        "acceptWithExecpolicyAmendment",
        "allow",
        "unknown",
    ] {
        assert_eq!(request.approval(decision), Err(Invalid::Decision));
    }
    for field in [
        "networkApprovalContext",
        "additionalPermissions",
        "environmentId",
    ] {
        let mut v = command();
        v["params"][field] = json!({"requested":true});
        assert_eq!(Callback::parse(&v, None), Err(Invalid::Unsupported));
    }
    let mut v = command();
    v["params"]["kind"] = json!("stdin");
    assert_eq!(Callback::parse(&v, None), Err(Invalid::Unsupported));
    let mut v = command();
    v["params"]["availableDecisions"] =
        json!(["decline","acceptForSession",{"acceptWithExecpolicyAmendment":{}}]);
    let request = Callback::parse(&v, None).unwrap();
    assert_eq!(request.decisions, vec!["decline"]);
    assert_eq!(request.approval("accept"), Err(Invalid::Decision));
    let file = json!({"id":88,"method":"item/fileChange/requestApproval","params":{"threadId":"thread-a","turnId":"turn-a","itemId":"file-a","reason":"Test edit"}});
    let changes = json!([{"path":"C:/test/a.txt","kind":{"type":"add"},"diff":"+test"}]);
    let context = json!({"id":"file-a","type":"fileChange","changes":changes});
    let request = Callback::parse(&file, Some(&context)).unwrap();
    assert_eq!(
        request.body,
        Body::Files {
            changes,
            reason: Some("Test edit".into())
        }
    );
    assert_eq!(Callback::parse(&file, None), Err(Invalid::Context));
    let mut wrong = context.clone();
    wrong["id"] = json!("file-b");
    assert_eq!(Callback::parse(&file, Some(&wrong)), Err(Invalid::Context));
    let mut v = file;
    v["params"]["grantRoot"] = json!("C:/test");
    assert_eq!(
        Callback::parse(&v, Some(&context)),
        Err(Invalid::Unsupported)
    );
}

fn source() -> Source {
    Source {
        session_id: "codex:11111111-2222-3333-4444-555555555555".into(),
        thread_id: "thread-a".into(),
        origin: "C:/test".into(),
    }
}
fn registered() -> (Registry, registry::Context) {
    let mut registry = Registry::default();
    registry.begin_connection();
    registry.subscribe(source()).unwrap();
    let context = registry
        .receive(Callback::parse(&question(), None).unwrap())
        .unwrap();
    (registry, context)
}

#[test]
fn exact_request_and_single_submission() {
    let (mut registry, context) = registered();
    let reply = json!({"answers":{"choice":"Stop"}});
    for field in [
        "nonce",
        "generation",
        "sessionId",
        "threadId",
        "turnId",
        "itemId",
    ] {
        let mut wrong = serde_json::to_value(&context).unwrap();
        wrong[field] = json!("another");
        assert_eq!(
            registry.prepare(&serde_json::from_value(wrong).unwrap(), &source(), &reply),
            Err(Invalid::Identity),
            "{field}"
        );
        assert_eq!(registry.views()[0].status, "pending");
    }
    for changed in [
        Source {
            origin: "C:/other".into(),
            ..source()
        },
        Source {
            session_id: "another".into(),
            ..source()
        },
        Source {
            thread_id: "thread-b".into(),
            ..source()
        },
    ] {
        assert_eq!(
            registry.prepare(&context, &changed, &reply),
            Err(Invalid::Identity)
        );
    }
    assert_eq!(
        registry.prepare(&context, &source(), &json!({"decision":"accept"})),
        Err(Invalid::Decision)
    );
    assert_eq!(
        registry.prepare(&context, &source(), &reply).unwrap(),
        json!({"id":77,"result":{"answers":{"choice":{"answers":["Stop"]}}}})
    );
    assert_eq!(registry.views()[0].status, "submitting");
    assert_eq!(
        registry.prepare(&context, &source(), &reply),
        Err(Invalid::Identity)
    );
    let repeated = registry
        .receive(Callback::parse(&question(), None).unwrap())
        .unwrap();
    assert_eq!(repeated, context);
    assert_eq!(registry.views().len(), 1);
    assert_eq!(registry.views()[0].status, "submitting");
    registry.unsubscribe("thread-a");
    assert_eq!(
        registry.prepare(&context, &source(), &reply),
        Err(Invalid::Identity)
    );
    let session = crate::demo::Session {
        id: "codex:11111111-2222-3333-4444-555555555555".into(),
        kind: "codex".into(),
        origin: "C:/test".into(),
        project: "test".into(),
        agent: "Codex".into(),
        symbol: "C".into(),
        state: "waiting".into(),
        request: None,
        command: None,
        message: String::new(),
        hidden: false,
        source_action: None,
        completion: None,
    };
    assert!(Source::from_session(&session, true).is_ok());
    assert_eq!(
        Source::from_session(&session, false),
        Err(Invalid::Identity)
    );
    let mut hidden = session.clone();
    hidden.hidden = true;
    assert_eq!(Source::from_session(&hidden, true), Err(Invalid::Identity));
    let mut wrong = session.clone();
    wrong.kind = "claude".into();
    assert_eq!(Source::from_session(&wrong, true), Err(Invalid::Identity));
    let service = super::service::Service::new(
        std::path::PathBuf::from("C:/capy-denied-test-source"),
        |_| false,
    );
    assert_eq!(
        service.connect(source()),
        Err("A sessão mudou ou não está disponível para respostas.".into())
    );
    assert!(service.snapshot().0.is_empty());
    assert!(service.snapshot().1.is_empty());
    service.reset();
    assert_eq!(
        service.connect(source()),
        Err("A sessão mudou ou não está disponível para respostas.".into())
    );
    let mut wrong = session;
    wrong.id = "codex:invalid".into();
    assert_eq!(Source::from_session(&wrong, true), Err(Invalid::Identity));
}

#[test]
fn resolution_and_connection_lifecycle() {
    let reply = json!({"answers":{"choice":"Stop"}});
    for event in [
        json!({"method":"serverRequest/resolved","params":{"threadId":"thread-a","requestId":77}}),
        json!({"method":"turn/completed","params":{"threadId":"thread-a","turn":{"id":"turn-a","status":"completed"}}}),
        json!({"method":"turn/completed","params":{"threadId":"thread-a","turn":{"id":"turn-a","status":"interrupted"}}}),
        json!({"method":"thread/closed","params":{"threadId":"thread-a"}}),
        json!({"method":"account/updated","params":{"authMode":null}}),
    ] {
        let (mut registry, context) = registered();
        registry.prepare(&context, &source(), &reply).unwrap();
        assert_eq!(
            registry.views()[0].status,
            "submitting",
            "send must not imply resolved"
        );
        registry.event(&event);
        assert!(registry.views().is_empty());
        assert_eq!(
            registry.prepare(&context, &source(), &reply),
            Err(Invalid::Identity)
        );
    }
    let (mut registry, context) = registered();
    registry.event(
        &json!({"method":"serverRequest/resolved","params":{"threadId":"another","requestId":77}}),
    );
    registry.event(
        &json!({"method":"serverRequest/resolved","params":{"threadId":"thread-a","requestId":78}}),
    );
    registry.event(&json!({"method":"turn/completed","params":{"threadId":"thread-a","turn":{"id":"another"}}}));
    assert_eq!(registry.views().len(), 1);
    registry.clear();
    assert!(registry.views().is_empty());
    assert_eq!(
        registry.prepare(&context, &source(), &reply),
        Err(Invalid::Identity)
    );
    let next = registry.begin_connection();
    assert_ne!(next, context.generation);
    registry.subscribe(source()).unwrap();
    let new = registry
        .receive(Callback::parse(&question(), None).unwrap())
        .unwrap();
    assert_ne!(new.nonce, context.nonce);
    assert_eq!(
        registry.prepare(&context, &source(), &reply),
        Err(Invalid::Identity)
    );
    let mut different = Callback::parse(&question(), None).unwrap();
    different.turn_id = "next-turn".into();
    assert_eq!(registry.receive(different), Err(Invalid::Identity));
    let mut other = Registry::default();
    assert_ne!(other.begin_connection(), next);

    let (mut registry, context) = registered();
    registry.invalidate_requests();
    assert!(registry.subscribed("thread-a"));
    assert_eq!(
        registry.prepare(&context, &source(), &reply),
        Err(Invalid::Identity)
    );
    let refreshed = registry
        .receive(Callback::parse(&question(), None).unwrap())
        .unwrap();
    assert_ne!(refreshed.generation, context.generation);
    assert_ne!(refreshed.nonce, context.nonce);
}

#[test]
fn subscription_and_response_protocol() {
    use std::net::{TcpListener, TcpStream};
    use std::time::{Duration, Instant};
    use tungstenite::Message;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut socket = tungstenite::accept(stream).unwrap();
        let read = |socket: &mut tungstenite::WebSocket<TcpStream>| -> Value {
            match socket.read().unwrap() {
                Message::Text(v) => serde_json::from_str(&v).unwrap(),
                other => panic!("Unexpected frame: {other:?}"),
            }
        };
        let init = read(&mut socket);
        assert_eq!(init["method"], "initialize");
        assert_eq!(
            init["params"],
            json!({"clientInfo":{"name":"capy_interventions","version":env!("CARGO_PKG_VERSION")},"capabilities":{"experimentalApi":true}})
        );
        socket
            .send(Message::Text(
                json!({"id":init["id"],"result":{}}).to_string().into(),
            ))
            .unwrap();
        assert_eq!(
            read(&mut socket),
            json!({"method":"initialized","params":{}})
        );
        let account = read(&mut socket);
        assert_eq!(account["method"], "account/read");
        assert_eq!(account["params"], json!({"refreshToken":false}));
        socket
            .send(Message::Text(
                json!({"method":"account/updated","params":{"authMode":"chatgpt"}})
                    .to_string()
                    .into(),
            ))
            .unwrap();
        socket
            .send(Message::Text(
                json!({"id":account["id"],"result":{"account":{"type":"chatgpt"},"requiresOpenaiAuth":true}})
                    .to_string()
                    .into(),
            ))
            .unwrap();
        let resume = read(&mut socket);
        assert_eq!(resume["method"], "thread/resume");
        assert_eq!(
            resume["params"],
            json!({"threadId":"thread-a","excludeTurns":true})
        );
        socket
            .send(Message::Text(question().to_string().into()))
            .unwrap();
        let mut second = question();
        second["id"] = json!(78);
        second["params"]["itemId"] = json!("item-b");
        socket
            .send(Message::Text(second.to_string().into()))
            .unwrap();
        socket
            .send(Message::Text(
                json!({"id":resume["id"],"result":{"thread":{"id":"thread-a","turns":[]}}})
                    .to_string()
                    .into(),
            ))
            .unwrap();
        assert_eq!(
            read(&mut socket),
            json!({"id":77,"result":{"answers":{"choice":{"answers":["Stop"]}}}})
        );
        socket.send(Message::Text(json!({"method":"serverRequest/resolved","params":{"threadId":"thread-a","requestId":77}}).to_string().into())).unwrap();
        let unsubscribe = read(&mut socket);
        assert_eq!(unsubscribe["method"], "thread/unsubscribe");
        assert_eq!(unsubscribe["params"], json!({"threadId":"thread-a"}));
        socket
            .send(Message::Text(
                json!({"id":unsubscribe["id"],"result":{}})
                    .to_string()
                    .into(),
            ))
            .unwrap();
        assert!(
            socket.read().is_err(),
            "No start, turn or policy request may follow"
        );
    });
    let stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut connection = protocol::Connection::connect(stream).unwrap();
    connection
        .stream_mut()
        .set_read_timeout(Some(Duration::from_millis(50)))
        .unwrap();
    connection.resume(source()).unwrap();
    let context = connection.views()[0].context.clone();
    connection
        .respond(&context, &source(), &json!({"answers":{"choice":"Stop"}}))
        .unwrap();
    assert_eq!(connection.views()[0].status, "submitting");
    assert!(connection
        .respond(
            &context,
            &source(),
            &json!({"answers":{"choice":"Continue"}})
        )
        .is_err());
    let deadline = Instant::now() + Duration::from_secs(2);
    while connection.views().len() == 2 && Instant::now() < deadline {
        connection.pump().unwrap();
    }
    assert_eq!(connection.views().len(), 1);
    assert_eq!(connection.views()[0].context.item_id, "item-b");
    assert_eq!(connection.views()[0].status, "pending");
    assert_eq!(connection.take_resolved(), vec![context.clone()]);
    assert!(connection
        .respond(&context, &source(), &json!({"answers":{"choice":"Stop"}}))
        .is_err());
    connection.unsubscribe("thread-a").unwrap();
    drop(connection);
    server.join().unwrap();
}
