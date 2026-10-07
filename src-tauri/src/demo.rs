use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct Session {
    pub id: String,
    pub project: String,
    pub agent: String,
    pub symbol: String,
    pub kind: String,
    pub origin: String,
    pub state: String,
    pub request: Option<String>,
    pub message: String,
    pub command: Option<String>,
    pub hidden: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_action: Option<crate::source_access::SourceAction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion: Option<String>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub sessions: Vec<Session>,
    pub scenario: String,
    pub reduce_motion: bool,
    pub integrations: Vec<crate::discovery::Integration>,
    pub quotas: Vec<crate::quotas::Row>,
    pub interventions: Vec<crate::interventions::registry::View>,
    pub subscriptions: Vec<crate::interventions::service::Subscription>,
    pub preferences: crate::settings::Preferences,
    pub quota_alerts: Vec<crate::quota_policy::Alert>,
    pub handoffs: Vec<crate::handoff::Review>,
    pub routing: Vec<crate::routing::Status>,
    pub chat_transfers: Vec<crate::chat_transfer::Review>,
}
impl Default for Snapshot {
    fn default() -> Self {
        Self {
            sessions: serde_json::from_str(include_str!("../../assets/demo.json"))
                .expect("bundled demo fixture is valid"),
            scenario: "waiting".into(),
            reduce_motion: false,
            integrations: vec![],
            quotas: vec![],
            interventions: vec![],
            subscriptions: vec![],
            preferences: crate::settings::Preferences::default(),
            quota_alerts: vec![],
            handoffs: vec![],
            routing: vec![],
            chat_transfers: vec![],
        }
    }
}
impl Snapshot {
    pub fn real() -> Self {
        Self {
            sessions: vec![],
            scenario: "real".into(),
            reduce_motion: false,
            integrations: vec![],
            quotas: vec![],
            interventions: vec![],
            subscriptions: vec![],
            preferences: crate::settings::Preferences::default(),
            quota_alerts: vec![],
            handoffs: vec![],
            routing: vec![],
            chat_transfers: vec![],
        }
    }
    pub fn apply(&mut self, action: &str, id: &str, answer: &str) -> Result<(), String> {
        if self.scenario == "real" && ["allow", "deny", "answer"].contains(&action) {
            return Err("Responder a sessões reais ainda não está disponível".into());
        }
        match action {
            "scenario" => {
                if !["real", "waiting", "working", "done", "sleeping"].contains(&answer) {
                    return Err("Cenário desconhecido".into());
                }
                self.scenario = answer.into();
                self.integrations.clear();
                self.interventions.clear();
                self.subscriptions.clear();
                self.quotas.clear();
                self.chat_transfers.clear();
                self.handoffs.clear();
                self.routing.clear();
                self.sessions = if answer == "sleeping" || answer == "real" {
                    vec![]
                } else {
                    Self::default().sessions
                };
                if answer != "waiting" && answer != "real" {
                    for session in &mut self.sessions {
                        session.state = answer.into();
                        session.message = if answer == "working" {
                            "O agente está executando sua tarefa."
                        } else {
                            "A tarefa foi concluída. Confira o resultado no terminal."
                        }
                        .into();
                    }
                }
            }
            "motion" => {
                if !["true", "false"].contains(&answer) {
                    return Err("Preferência inválida".into());
                }
                self.reduce_motion = answer == "true";
            }
            "restore" => self.sessions.iter_mut().for_each(|s| s.hidden = false),
            "hide" => {
                self.sessions
                    .iter_mut()
                    .find(|s| s.id == id)
                    .ok_or("Sessão não encontrada")?
                    .hidden = true
            }
            "allow" | "deny" | "answer" => {
                let session = self
                    .sessions
                    .iter_mut()
                    .find(|s| s.id == id)
                    .ok_or("Sessão não encontrada")?;
                if session.state != "waiting" {
                    return Err("Este pedido já foi respondido".into());
                }
                if action == "answer"
                    && (session.request.as_deref() != Some("question")
                        || !["Só balão", "Balão e som"].contains(&answer))
                {
                    return Err("Resposta inválida".into());
                }
                if action != "answer" && session.request.as_deref() != Some("permission") {
                    return Err("Este pedido não é uma permissão".into());
                }
                session.state = "working".into();
                session.message = match action {
                    "allow" => "Permissão simulada concedida. Executando os testes.".into(),
                    "deny" => "Permissão simulada negada. Procurando outra alternativa.".into(),
                    _ => format!("Resposta simulada: {answer}. O agente está continuando."),
                };
            }
            _ => return Err("Ação desconhecida".into()),
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_mode_rejects_simulated_responses() {
        let mut data = Snapshot::real();
        for action in ["allow", "deny", "answer"] {
            assert!(data.apply(action, "any", "").is_err());
        }
        data.apply("scenario", "", "waiting").unwrap();
        assert_eq!(data.sessions.len(), 3);
        data.apply("scenario", "", "real").unwrap();
        assert!(data.sessions.is_empty());
        assert_eq!(data.scenario, "real");
    }
    #[test]
    fn permission_and_question_continue_only_their_session() {
        let mut data = Snapshot::default();
        data.apply("allow", "claude", "").unwrap();
        assert_eq!(data.sessions[0].state, "working");
        assert_eq!(data.sessions[1].state, "waiting");
        data.apply("answer", "codex", "Só balão").unwrap();
        assert_eq!(data.sessions[1].state, "working");
        assert!(data.apply("allow", "claude", "").is_err());
    }
    #[test]
    fn hide_restore_and_sleeping_change_followed_count() {
        let mut data = Snapshot::default();
        data.apply("hide", "claude", "").unwrap();
        assert_eq!(data.sessions.iter().filter(|s| !s.hidden).count(), 2);
        data.apply("restore", "", "").unwrap();
        assert_eq!(data.sessions.iter().filter(|s| !s.hidden).count(), 3);
        data.apply("scenario", "", "sleeping").unwrap();
        assert!(data.sessions.is_empty());
    }
}
