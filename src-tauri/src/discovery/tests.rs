use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
const A: &str = "11111111-1111-1111-1111-111111111111";
const B: &str = "22222222-2222-2222-2222-222222222222";
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "capy-discovery-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn write(&self, relative: &str, value: &str) -> PathBuf {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, value).unwrap();
        path
    }
    fn sources(&self) -> Sources {
        Sources {
            claude: self.0.join("claude"),
            codex: self.0.join("codex"),
            antigravity: self.0.join("antigravity"),
        }
    }
    fn claude(&self, id: &str, pid: u32, birth: &str) {
        self.write(&format!("claude/sessions/{pid}.json"), &serde_json::json!({"pid":pid,"sessionId":id,"cwd":"C:\\work\\Project","procStart":birth}).to_string());
    }
    fn codex(&self, id: &str, source: serde_json::Value) -> File {
        self.write(&format!("codex/sessions/2026/10/05/rollout-date-{id}.jsonl"), &format!("{}\n", serde_json::json!({"type":"session_meta","payload":{"id":id,"cwd":"C:\\work\\Project","source":source}})));
        let path = self.write(&format!("codex/thread-writer-locks/{id}.lock"), "");
        let file = File::options().read(true).write(true).open(path).unwrap();
        file.lock().unwrap();
        file
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn claude_requires_matching_process_identity() {
    let fixture = Fixture::new();
    fixture.claude(A, 10, "99");
    fixture.claude(B, 11, "98");
    let report = scan_with(&fixture.sources(), |pid| {
        if pid == 10 {
            Some(99)
        } else {
            Some(123)
        }
    });
    assert_eq!(report.sessions.len(), 1);
    assert_eq!(report.sessions[0].id, format!("claude:{A}"));
    assert!(report.integrations[0]
        .message
        .contains("1 registro descartado"));
    let report = scan_with(&fixture.sources(), |_| None);
    assert!(report.sessions.is_empty());
    assert!(report.integrations[0]
        .message
        .contains("2 registros descartados"));
    assert!(process_birth(std::process::id()).is_some());
    assert_eq!(process_birth(u32::MAX), None);
}

#[test]
fn codex_requires_held_lock_and_matching_metadata() {
    let fixture = Fixture::new();
    let owner = fixture.codex(A, "cli".into());
    assert_eq!(scan_with(&fixture.sources(), |_| None).sessions.len(), 1);
    drop(owner);
    assert!(scan_with(&fixture.sources(), |_| None).sessions.is_empty());
    let owner = fixture.codex(A, serde_json::json!({"subagent":{"parent_thread_id":B}}));
    assert!(scan_with(&fixture.sources(), |_| None).sessions.is_empty());
    drop(owner);
    let owner = fixture.codex(A, "cli".into());
    fixture.write(&format!("codex/sessions/2026/10/05/rollout-date-{A}.jsonl"), &format!("{}\n", serde_json::json!({"type":"session_meta","payload":{"id":B,"cwd":"C:\\work","source":"cli"}})));
    assert!(scan_with(&fixture.sources(), |_| None).sessions.is_empty());
    drop(owner);
}

#[test]
fn identity_and_unknown_state_are_preserved() {
    let fixture = Fixture::new();
    fixture.claude(A, 10, "99");
    fixture.claude(B, 11, "99");
    let owner = fixture.codex(A, "app-server".into());
    let report = scan_with(&fixture.sources(), |_| Some(99));
    assert_eq!(report.sessions.len(), 3);
    assert!(report.sessions.iter().all(|s| s.project == "Project"
        && s.state == "unknown"
        && s.request.is_none()
        && s.command.is_none()));
    assert_eq!(
        report
            .sessions
            .iter()
            .map(|s| &s.id)
            .collect::<BTreeSet<_>>()
            .len(),
        3
    );
    drop(owner);
}

#[test]
fn missing_and_bad_sources_are_reported() {
    let fixture = Fixture::new();
    let report = scan_with(&fixture.sources(), |_| None);
    assert_eq!(report.integrations.len(), 3);
    assert!(report.integrations[0].message.contains("não encontrada"));
    assert!(report.integrations[1].message.contains("não encontrada"));
    assert!(report.integrations[2].message.contains("pendente"));
    fixture.write("claude/sessions/bad.json", "{");
    fixture.claude(A, 10, "99");
    let owner = fixture.codex(B, "cli".into());
    fixture.write(
        &format!("codex/sessions/2026/10/05/rollout-date-{B}.jsonl"),
        "partial",
    );
    let report = scan_with(&fixture.sources(), |_| Some(99));
    assert_eq!(report.sessions.len(), 1);
    assert!(report.integrations[0]
        .message
        .contains("1 registro descartado"));
    assert!(report.integrations[1]
        .message
        .contains("1 registro descartado"));
    fixture.write("claude/sessions/large.json", &" ".repeat(65_537));
    assert!(
        small_json::<serde_json::Value>(&fixture.0.join("claude/sessions/large.json")).is_err()
    );
    drop(owner);
}

#[test]
fn hidden_preferences_survive_restart() {
    let fixture = Fixture::new();
    let path = fixture.0.join("capy/hidden.json");
    let mut preferences = Preferences::load(path.clone());
    preferences.hidden.insert(format!("claude:{A}"));
    preferences.save().unwrap();
    let mut preferences = Preferences::load(path.clone());
    let mut sessions = vec![
        session("Claude Code", "claude", A, "C:\\work"),
        session("Claude Code", "claude", B, "C:\\work"),
    ];
    preferences.apply(&mut sessions);
    assert!(sessions[0].hidden);
    assert!(!sessions[1].hidden);
    preferences.hidden.clear();
    preferences.save().unwrap();
    assert!(Preferences::load(path).hidden.is_empty());
}

#[test]
fn held_lock_without_metadata_or_with_unknown_source_is_diagnostic() {
    let fixture = Fixture::new();
    let owner = fixture.codex(A, "future-source".into());
    let report = scan_with(&fixture.sources(), |_| None);
    assert!(report.sessions.is_empty());
    assert!(report.integrations[1]
        .message
        .contains("1 registro descartado"));
    fs::remove_file(
        fixture
            .0
            .join(format!("codex/sessions/2026/10/05/rollout-date-{A}.jsonl")),
    )
    .unwrap();
    let report = scan_with(&fixture.sources(), |_| None);
    assert!(report.sessions.is_empty());
    assert!(report.integrations[1]
        .message
        .contains("1 registro descartado"));
    drop(owner);
}
