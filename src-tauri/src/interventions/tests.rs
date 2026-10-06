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
