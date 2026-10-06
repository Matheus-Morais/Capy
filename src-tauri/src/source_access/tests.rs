use super::*;
const ID: &str = "11111111-1111-1111-1111-111111111111";
fn session() -> Session {
    crate::discovery::session("Codex", "codex", ID, "C:\\work\\Capy")
}
fn report(session: &Session) -> Report {
    Report {
        sessions: vec![session.clone()],
        integrations: vec![],
    }
}

#[test]
fn opens_only_the_current_codex_identity() {
    let session = session();
    let mut sent = Vec::new();
    open_verified(&session, true, &report(&session), true, |url| {
        sent.push(url.to_owned());
        Ok(())
    })
    .unwrap();
    assert_eq!(
        sent,
        ["codex://threads/11111111-1111-1111-1111-111111111111"]
    );
}

#[test]
fn stale_or_unsupported_requests_never_dispatch() {
    let session = session();
    let mut other = session.clone();
    other.origin = "C:\\different".into();
    let mut malformed = session.clone();
    malformed.id = "codex:11111111-1111-1111-1111-111111111111?prompt=oops".into();
    let mut unsupported = session.clone();
    unsupported.kind = "claude".into();
    for (expected, real, current) in [
        (&session, true, Report::default()),
        (&session, true, report(&other)),
        (&session, false, report(&session)),
        (&malformed, true, report(&malformed)),
        (&unsupported, true, report(&unsupported)),
    ] {
        let result = open_verified(expected, real, &current, true, |_| {
            panic!("must not dispatch")
        });
        assert!(result.is_err());
    }
}

#[test]
fn missing_handler_and_dispatch_errors_are_visible() {
    assert!(registered_handler(|packaged| !packaged));
    assert!(registered_handler(|packaged| packaged));
    assert!(!registered_handler(|_| false));
    assert!(dispatched(33).is_ok());
    assert_eq!(dispatched(32).unwrap_err(), "O Windows não conseguiu abrir a conversa no Codex (código 32). Abra-a no aplicativo de origem.");
    assert_eq!(dispatched(0).unwrap_err(), "O Windows não conseguiu abrir a conversa no Codex (código 0). Abra-a no aplicativo de origem.");
    let session = session();
    let missing = open_verified(&session, true, &report(&session), false, |_| {
        panic!("must not dispatch")
    });
    assert_eq!(
        missing.unwrap_err(),
        "O link do Codex não está registrado neste Windows."
    );
    let failure = open_verified(&session, true, &report(&session), true, |_| {
        Err("Windows refused".into())
    });
    assert_eq!(failure.unwrap_err(), "Windows refused");
}
