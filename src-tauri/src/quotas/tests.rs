use super::*;
use serde_json::json;
fn account() -> Value {
    json!({"account":{"type":"chatgpt","email":"person@example.test","planType":"pro"}})
}
fn limits() -> Value {
    json!({"rateLimitsByLimitId":{"codex":{"limitId":"codex","primary":{"usedPercent":25,"windowDurationMins":300,"resetsAt":2000},"secondary":{"usedPercent":42,"windowDurationMins":10080,"resetsAt":3000}}}})
}
fn sample() -> Sample {
    Sample::parse(account(), limits(), account()).unwrap()
}

#[test]
fn quota_protocol_is_read_only_and_checks_identity_twice() {
    use std::net::{TcpListener, TcpStream};
    use std::time::Duration;
    for notify in [false, true] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut socket = tungstenite::accept(stream).unwrap();
            let mut methods = Vec::new();
            for expected in [
                "initialize",
                "initialized",
                "account/read",
                "account/rateLimits/read",
                "account/read",
            ] {
                let req: Value =
                    serde_json::from_str(socket.read().unwrap().to_text().unwrap()).unwrap();
                assert_eq!(req["method"], expected);
                methods.push(expected);
                if expected == "initialized" {
                    continue;
                }
                let response = match expected {
                    "account/read" => {
                        assert_eq!(req["params"], json!({"refreshToken":false}));
                        account()
                    }
                    "account/rateLimits/read" => {
                        assert_eq!(req["params"], json!({}));
                        if notify {
                            socket.send(tungstenite::Message::Text(json!({"method":"account/updated","params":{"authMode":"chatgpt"}}).to_string().into())).unwrap();
                        }
                        limits()
                    }
                    _ => json!({}),
                };
                socket
                    .send(tungstenite::Message::Text(
                        json!({"id":req["id"],"result":response}).to_string().into(),
                    ))
                    .unwrap();
            }
            assert_eq!(
                methods,
                [
                    "initialize",
                    "initialized",
                    "account/read",
                    "account/rateLimits/read",
                    "account/read"
                ]
            );
            assert!(
                socket.read().is_err(),
                "No mutation or thread call may follow quota reads"
            );
        });
        let stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let raw = crate::activity::protocol::quota(stream).unwrap();
        assert_eq!(raw.before, account());
        assert_eq!(raw.after, account());
        assert_eq!(raw.identity_changed, notify);
        if notify {
            assert_eq!(raw.into_sample().err(), Some(Error::AccountChanged));
        } else {
            assert_eq!(raw.into_sample().unwrap().account, "person@example.test");
        }
        server.join().unwrap();
    }
}

#[test]
fn account_bucket_and_window_validation() {
    let changed = RawSample {
        before: account(),
        limits: limits(),
        after: account(),
        identity_changed: true,
    };
    assert_eq!(changed.into_sample().err(), Some(Error::AccountChanged));
    let rows = sample().rows(1_000_000);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].provider, "Codex");
    assert_eq!(rows[0].account.as_deref(), Some("person@example.test"));
    assert_eq!(rows[0].bucket.as_deref(), Some("codex"));
    assert_eq!(rows[0].period.as_deref(), Some("primary"));
    let primary = rows[0].window.as_ref().unwrap();
    assert_eq!(
        (
            primary.used_percent,
            primary.window_duration_mins,
            primary.resets_at
        ),
        (25.0, 300, 2000)
    );
    let secondary = rows[1].window.as_ref().unwrap();
    assert_eq!(rows[1].period.as_deref(), Some("secondary"));
    assert_eq!(
        (
            secondary.used_percent,
            secondary.window_duration_mins,
            secondary.resets_at
        ),
        (42.0, 10080, 3000)
    );
    assert_eq!(rows[0].observed_at, Some(1_000_000));
    assert_eq!(rows[0].state, "fresh");
    let mut changed = account();
    changed["account"]["email"] = json!("other@example.test");
    assert_eq!(
        Sample::parse(account(), limits(), changed).err(),
        Some(Error::AccountChanged)
    );
    for (invalid, error) in [
        (json!({"account":null}), Error::NoAccount),
        (
            json!({"account":{"type":"apiKey"}}),
            Error::UnsupportedAccount,
        ),
        (
            json!({"account":{"type":"chatgpt","email":null}}),
            Error::NoIdentity,
        ),
        (
            json!({"account":{"type":"chatgpt","email":"x\n@y"}}),
            Error::NoIdentity,
        ),
        (
            json!({"account":{"type":"chatgpt","email":format!("{}@a", "x".repeat(321))}}),
            Error::NoIdentity,
        ),
    ] {
        assert_eq!(
            Sample::parse(invalid.clone(), limits(), invalid).err(),
            Some(error)
        );
    }
    for invalid in [
        json!({}),
        json!({"rateLimitsByLimitId":{}}),
        json!({"rateLimitsByLimitId":[]}),
        json!({"rateLimitsByLimitId":{"a":{"limitId":"b"}}}),
    ] {
        assert_eq!(
            Sample::parse(account(), invalid, account()).err(),
            Some(Error::InvalidLimits)
        );
    }
    let mut invalid = limits();
    invalid["rateLimitsByLimitId"]["codex"]["extra"] = json!("x".repeat(65_536));
    assert_eq!(
        Sample::parse(account(), invalid, account()).err(),
        Some(Error::InvalidLimits)
    );
    let mut map = serde_json::Map::new();
    for i in 0..65 {
        let id = format!("bucket-{i}");
        map.insert(id.clone(), json!({"limitId":id}));
    }
    assert_eq!(
        Sample::parse(account(), json!({"rateLimitsByLimitId":map}), account()).err(),
        Some(Error::InvalidLimits)
    );
    let legacy = json!({"rateLimits":limits()["rateLimitsByLimitId"]["codex"]});
    assert_eq!(
        Sample::parse(account(), legacy, account())
            .unwrap()
            .rows(1_000_000)[0]
            .window
            .as_ref()
            .unwrap()
            .used_percent,
        25.0
    );
    for (field, value) in [
        ("usedPercent", json!(-1)),
        ("usedPercent", json!(101)),
        ("usedPercent", json!("25")),
        ("windowDurationMins", json!(0)),
        ("resetsAt", json!(0)),
        ("resetsAt", json!(u64::MAX)),
    ] {
        let mut invalid = limits();
        invalid["rateLimitsByLimitId"]["codex"]["primary"][field] = value;
        let rows = Sample::parse(account(), invalid, account())
            .unwrap()
            .rows(1_000_000);
        assert!(rows[0].window.is_none());
        assert_eq!(rows[0].state, "unavailable");
        assert!(rows[1].window.is_some());
    }
}

#[test]
fn refresh_expiry_failure_and_recovery() {
    let mut cache = Cache::default();
    assert!(cache.rows(1_000_000, || Ok(sample()))[0].window.is_some());
    assert!(
        cache.rows(1_059_999, || panic!("must not poll before 60s"))[0]
            .window
            .is_some()
    );
    let failed = cache.rows(1_060_000, || Err(Error::Connection));
    assert!(failed.iter().all(|r| r.window.is_none()));
    assert_eq!(failed[0].state, "stale");
    assert!(failed[0].message.contains("última observada"));
    assert_eq!(failed[0].account.as_deref(), Some("person@example.test"));
    assert!(cache.rows(1_120_000, || Ok(sample()))[0].window.is_some());
    assert!(cache.current(1_240_000)[0].window.is_some());
    assert!(cache.current(1_240_001)[0].window.is_none());
    cache.rows(1_300_000, || Ok(sample()));
    assert!(
        cache.current(1_299_999)[0].window.is_none(),
        "future evidence must not have a percentage"
    );
    cache.rows(1_400_000, || Ok(sample()));
    let reset = cache.current(2_000_000);
    assert!(reset[0].window.is_none());
    assert!(
        reset[1].window.is_none(),
        "evidence also expires before secondary reset"
    );
    let at_reset = sample().rows(2_000_000);
    assert!(at_reset[0].window.is_none());
    assert!(at_reset[1].window.is_some());
    assert_eq!(at_reset[0].state, "stale");
    let mut no_account = Cache::default();
    let absent = no_account.rows(1_000_000, || Err(Error::NoAccount));
    assert!(absent[0].account.is_none());
    assert_eq!(absent[0].state, "unavailable");
    assert!(absent[0].message.contains("não confirmou uma conta"));
}
