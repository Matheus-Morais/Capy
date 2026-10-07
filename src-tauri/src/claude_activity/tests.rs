use super::*;
use crate::demo::Session;
const ID: &str = "11111111-2222-3333-4444-555555555555";
struct Fixture {
    sources: Sources,
}
impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "capy-claude-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let sources = Sources {
            claude: dir.clone(),
            codex: dir.join("codex"),
            antigravity: dir.join("antigravity"),
        };
        fs::create_dir_all(dir.join("sessions")).unwrap();
        fs::create_dir_all(directory(&sources)).unwrap();
        fs::write(
            dir.join("sessions/42.json"),
            serde_json::json!({"pid":42,"sessionId":ID,"cwd":"C:/test","procStart":"100"})
                .to_string(),
        )
        .unwrap();
        Self { sources }
    }
    fn path(&self) -> PathBuf {
        directory(&self.sources).join(format!("{ID}.json"))
    }
    fn observation(&self, state: &str) -> Observation {
        Observation {
            version: 1,
            session_id: ID.into(),
            cwd: "C:/test".into(),
            pid: 42,
            proc_start: "100".into(),
            at_ms: 100_000,
            state: state.into(),
        }
    }
    fn put(&self, o: &Observation) {
        fs::write(self.path(), serde_json::to_vec(o).unwrap()).unwrap();
    }
    fn report(&self) -> Report {
        Report {
            sessions: vec![Session {
                id: format!("claude:{ID}"),
                project: "test".into(),
                agent: "Claude Code".into(),
                symbol: "C".into(),
                kind: "claude".into(),
                origin: "C:/test".into(),
                state: "waiting".into(),
                request: Some("old".into()),
                message: "old".into(),
                command: Some("old".into()),
                hidden: false,
                source_action: None,
                completion: None,
            }],
            integrations: vec![],
        }
    }
    fn state(&self, now: u64) -> String {
        let mut r = self.report();
        enrich_with(&self.sources, &mut r, now, |_| Some(100));
        r.sessions[0].state.clone()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.sources.claude);
    }
}
#[test]
fn event_contract() {
    for (event, expected) in [
        ("UserPromptSubmit", "working"),
        ("PreToolUse", "working"),
        ("PostToolUse", "working"),
        ("PostToolUseFailure", "working"),
        ("PostToolBatch", "working"),
        ("PermissionDenied", "working"),
        ("PermissionRequest", "waiting"),
        ("SessionStart", "unknown"),
        ("Stop", "unknown"),
        ("StopFailure", "unknown"),
        ("SessionEnd", "unknown"),
        ("new-event", "unknown"),
    ] {
        let mut h:Hook=serde_json::from_value(serde_json::json!({"session_id":ID,"cwd":"C:/test","hook_event_name":event,"stop_hook_active":true})).unwrap();
        assert_eq!(event_state(&h), Some(expected), "{event}");
        h.agent_id = Some("child".into());
        assert_eq!(event_state(&h), None);
    }
    for (kind, expected) in [
        ("permission_prompt", "waiting"),
        ("idle_prompt", "idle"),
        ("auth_success", "unknown"),
        ("elicitation_dialog", "unknown"),
    ] {
        let h:Hook=serde_json::from_value(serde_json::json!({"session_id":ID,"cwd":"C:/test","hook_event_name":"Notification","notification_type":kind})).unwrap();
        assert_eq!(event_state(&h), Some(expected));
    }
}
#[test]
fn identity_expiry_and_limits() {
    let f = Fixture::new();
    let o = f.observation("working");
    f.put(&o);
    assert_eq!(f.state(130_000), "working");
    assert_eq!(f.state(130_001), "unknown");
    assert_eq!(f.state(99_999), "unknown");
    for field in ["version", "session_id", "cwd", "pid", "proc_start", "state"] {
        let mut x = serde_json::to_value(&o).unwrap();
        x[field] = match field {
            "version" | "pid" => serde_json::json!(99),
            _ => serde_json::json!("invalid"),
        };
        fs::write(f.path(), x.to_string()).unwrap();
        assert_eq!(f.state(100_000), "unknown", "{field}");
    }
    f.put(&o);
    let mut r = f.report();
    enrich_with(&f.sources, &mut r, 100_000, |_| Some(101));
    assert_eq!(r.sessions[0].state, "unknown");
    fs::write(f.path(), vec![b' '; 65_537]).unwrap();
    assert_eq!(f.state(100_000), "unknown");
    fs::write(f.path(), b"{").unwrap();
    assert_eq!(f.state(100_000), "unknown");
    f.put(&o);
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .open(f.path())
        .unwrap();
    lock.try_lock().unwrap();
    assert_eq!(f.state(100_000), "unknown");
    drop(lock);
    let mut r = f.report();
    r.sessions = vec![r.sessions[0].clone(); 65];
    enrich_with(&f.sources, &mut r, 100_000, |_| Some(100));
    assert!(r.sessions[..64].iter().all(|s| s.state == "working"));
    assert_eq!(r.sessions[64].state, "unknown");
}
#[test]
fn waiting_is_invalidated() {
    let f = Fixture::new();
    for state in ["working", "unknown", "idle"] {
        f.put(&f.observation(state));
        let mut r = f.report();
        enrich_with(&f.sources, &mut r, 100_000, |_| Some(100));
        assert_eq!(r.sessions[0].state, state);
        assert!(r.sessions[0].request.is_none());
        assert!(r.sessions[0].command.is_none());
    }
    f.put(&f.observation("waiting"));
    assert_eq!(f.state(100_000), "waiting");
    assert_eq!(f.state(130_001), "unknown");
    fs::remove_file(f.path()).unwrap();
    assert_eq!(f.state(100_000), "unknown");
    f.put(&f.observation("working"));
    fs::write(directory(&f.sources).join("disabled"), b"disabled").unwrap();
    assert_eq!(f.state(100_000), "unknown");
}
#[test]
fn bounded_metadata_and_order() {
    let f = Fixture::new();
    let o = f.observation("waiting");
    write_observation(&f.path(), &o).unwrap();
    let mut older = f.observation("working");
    older.at_ms = 99_999;
    write_observation(&f.path(), &older).unwrap();
    assert_eq!(f.state(100_000), "waiting");
    older.at_ms = 100_001;
    write_observation(&f.path(), &older).unwrap();
    assert_eq!(f.state(100_001), "working");
    let x: serde_json::Value = serde_json::from_slice(&fs::read(f.path()).unwrap()).unwrap();
    assert_eq!(x.as_object().unwrap().len(), 7);
    for field in [
        "version",
        "session_id",
        "cwd",
        "pid",
        "proc_start",
        "at_ms",
        "state",
    ] {
        assert!(x.get(field).is_some());
    }
    assert!(read_json::<Hook>(std::io::repeat(b' ').take(INPUT_LIMIT + 1), INPUT_LIMIT).is_err());
    let input = serde_json::json!({"session_id":ID,"cwd":"C:/test","hook_event_name":"Stop","prompt":"private","last_assistant_message":"private","credentials":"private"});
    let h: Hook = read_json(input.to_string().as_bytes(), INPUT_LIMIT).unwrap();
    assert_eq!(event_state(&h), Some("unknown"));
    for i in 0..140 {
        fs::write(
            directory(&f.sources).join(format!("stale-{i}.json")),
            serde_json::to_vec(&o).unwrap(),
        )
        .unwrap();
    }
    cleanup(&directory(&f.sources), |_| None);
    let remaining = fs::read_dir(directory(&f.sources)).unwrap().count();
    assert!(remaining >= 13);
    assert!(remaining < 141);
}

#[test]
fn handoff_boundary_never_uses_stop_that_another_hook_can_block(){
    let f=Fixture::new();
    let cwd=f.sources.claude.canonicalize().unwrap();
    let pid=std::process::id();let birth=discovery::process_birth(pid).unwrap();
    let at=now_ms().saturating_sub(1_001);
    let path=directory(&f.sources).join(format!("{ID}.boundary.json"));
    for event in ["Stop","PreToolUse","UserPromptSubmit","PermissionRequest","SessionEnd"]{
        let receipt=TurnReceipt{version:1,session_id:ID.into(),cwd:cwd.to_string_lossy().into_owned(),pid,proc_start:birth.to_string(),at_ms:at,event:event.into()};
        fs::write(&path,serde_json::to_vec(&receipt).unwrap()).unwrap();
        assert!(turn_boundary(&f.sources.claude,ID,&cwd.to_string_lossy(),now_ms()).is_none(),"{event}");
    }
    for event in ["idle_prompt","StopFailure"]{
        let receipt=TurnReceipt{version:1,session_id:ID.into(),cwd:cwd.to_string_lossy().into_owned(),pid,proc_start:birth.to_string(),at_ms:at,event:event.into()};
        fs::write(&path,serde_json::to_vec(&receipt).unwrap()).unwrap();
        let boundary=turn_boundary(&f.sources.claude,ID,&cwd.to_string_lossy(),now_ms()).unwrap();
        assert!(boundary_unchanged(&f.sources.claude,ID,&cwd.to_string_lossy(),&boundary));
        assert!(turn_boundary(&f.sources.claude,ID,&cwd.to_string_lossy(),at+TTL_MS+1).is_none());
        let resumed=TurnReceipt{at_ms:at+1,event:"UserPromptSubmit".into(),..receipt};
        fs::write(&path,serde_json::to_vec(&resumed).unwrap()).unwrap();
        assert!(!boundary_unchanged(&f.sources.claude,ID,&cwd.to_string_lossy(),&boundary));
    }
}
