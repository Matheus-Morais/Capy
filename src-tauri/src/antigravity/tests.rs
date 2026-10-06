use super::*;
use std::fs::File;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
const ID: &str = "11111111-1111-1111-1111-111111111111";

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "capy-agy-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("antigravity-cli/presence")).unwrap();
        Self(root)
    }
    fn sources(&self) -> Sources {
        Sources {
            claude: self.0.join("claude"),
            codex: self.0.join("codex"),
            antigravity: self.0.clone(),
        }
    }
    fn metadata(&self, uris: &str, parent: &str) {
        let db =
            Connection::open(self.0.join("antigravity-cli/conversation_summaries.db")).unwrap();
        db.execute_batch("CREATE TABLE IF NOT EXISTS conversation_summaries (conversation_id TEXT PRIMARY KEY,workspace_uris TEXT,parent_conversation_id TEXT);").unwrap();
        db.execute(
            "INSERT OR REPLACE INTO conversation_summaries VALUES (?1,?2,?3)",
            [ID, uris, parent],
        )
        .unwrap();
    }
    fn lock(&self) -> File {
        let file = File::options()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.0.join(format!("antigravity-cli/presence/{ID}.lock")))
            .unwrap();
        file.lock().unwrap();
        file
    }
    fn scan(&self) -> Report {
        let mut report = Report::default();
        scan(&self.sources(), &mut report);
        report
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn held_presence_requires_workspace_and_main_session() {
    let fixture = Fixture::new();
    fixture.metadata("[\"file:///C:/work/My%20Project\"]", "");
    let owner = fixture.lock();
    let report = fixture.scan();
    assert_eq!(report.sessions.len(), 1);
    assert_eq!(report.sessions[0].id, format!("antigravity:{ID}"));
    assert_eq!(report.sessions[0].origin, "C:\\work\\My Project");
    assert_eq!(report.sessions[0].project, "My Project");
    assert_eq!(report.sessions[0].state, "unknown");
    drop(owner);
    assert!(fixture.scan().sessions.is_empty());
    let owner = fixture.lock();
    fixture.metadata("[\"file:///C:/work/My%20Project\"]", "parent");
    assert!(fixture.scan().sessions.is_empty());
    drop(owner);
}

#[test]
fn invalid_metadata_is_diagnostic() {
    let fixture = Fixture::new();
    let owner = fixture.lock();
    assert!(fixture.scan().sessions.is_empty());
    assert!(fixture.scan().integrations[0]
        .message
        .contains("1 registros incompatíveis"));
    for uris in [
        "broken",
        "[]",
        "[\"vscode-remote://host/work\"]",
        "[\"file://remote/work\"]",
    ] {
        fixture.metadata(uris, "");
        assert!(fixture.scan().sessions.is_empty());
        assert!(fixture.scan().integrations[0]
            .message
            .contains("1 registros incompatíveis"));
    }
    assert!(workspace(&"x".repeat(65_537)).is_err());
    let db = Connection::open(fixture.0.join("antigravity-cli/conversation_summaries.db")).unwrap();
    db.execute_batch("DROP TABLE conversation_summaries; CREATE TABLE conversation_summaries (conversation_id TEXT);").unwrap();
    assert!(fixture.scan().sessions.is_empty());
    assert!(fixture.scan().integrations[0]
        .message
        .contains("1 registros incompatíveis"));
    drop(owner);
}

fn sample_observation() -> Observation {
    Observation {
        version: 1,
        id: ID.into(),
        cwd: "C:\\work\\Project".into(),
        variant: "antigravity-ide".into(),
        pid: 42,
        birth: 100,
        at_ms: 100_000,
        state: "working".into(),
    }
}
#[test]
fn hook_presence_requires_live_identity() {
    let o = sample_observation();
    assert!(valid_observation(&o, 100_000, |_| Some(100)));
    assert!(!valid_observation(&o, 100_000, |_| None));
    assert!(!valid_observation(&o, 100_000, |_| Some(101)));
    let fixture = Fixture::new();
    let mut h = Hook {
        conversation_id: ID.into(),
        workspace_paths: vec![o.cwd],
        transcript_path: String::new(),
    };
    for name in ["transcript.jsonl", "transcript_full.jsonl"] {
        h.transcript_path = fixture
            .0
            .join(format!(
                "antigravity-cli/brain/{ID}/.system_generated/logs/{name}"
            ))
            .to_string_lossy()
            .replace('\\', "/");
        assert_eq!(
            hook_variant(&fixture.sources(), &h),
            Some("antigravity-cli")
        );
    }
    h.transcript_path = fixture
        .0
        .join("antigravity-cli/brain/other/.system_generated/logs/transcript_full.jsonl")
        .to_string_lossy()
        .into();
    assert_eq!(hook_variant(&fixture.sources(), &h), None);
}
#[test]
fn activity_is_bounded_and_never_done() {
    let mut o = sample_observation();
    assert!(valid_observation(&o, 130_000, |_| Some(100)));
    assert!(!valid_observation(&o, 130_001, |_| Some(100)));
    assert!(!valid_observation(&o, 99_999, |_| Some(100)));
    assert_eq!(event_state("PreInvocation"), "working");
    assert_eq!(event_state("PostToolUse"), "working");
    assert_eq!(event_state("Stop"), "unknown");
    o.state = "done".into();
    assert!(!valid_observation(&o, 100_000, |_| Some(100)));
}
