use crate::discovery::{Integration, Report, Sources};
use serde::Deserialize;
use serde_json::Value;

pub(crate) mod protocol;
#[cfg(windows)]
pub(crate) mod proxy;
const SESSION_LIMIT: usize = 64;

#[derive(Deserialize)]
pub(super) struct Observation {
    id: String,
    cwd: String,
    status: Value,
}

fn runtime_state(status: &Value) -> (&'static str, &'static str) {
    match status["type"].as_str() {
        Some("idle") => ("idle", "Sessão ociosa. Nenhuma tarefa em execução."),
        Some("active") => {
            let Some(flags) = status["activeFlags"].as_array() else {
                return unknown();
            };
            if flags
                .iter()
                .any(|f| !matches!(f.as_str(), Some("waitingOnApproval" | "waitingOnUserInput")))
            {
                return unknown();
            }
            if flags.is_empty() {
                ("working", "O Codex está trabalhando nesta sessão.")
            } else if flags
                .iter()
                .any(|f| f.as_str() == Some("waitingOnApproval"))
            {
                (
                    "waiting",
                    "O Codex aguarda uma permissão. Responda no Codex.",
                )
            } else {
                (
                    "waiting",
                    "O Codex aguarda sua resposta. Responda no Codex.",
                )
            }
        }
        _ => unknown(),
    }
}
fn unknown() -> (&'static str, &'static str) {
    (
        "unknown",
        "Sessão aberta. Não foi possível confirmar a atividade atual.",
    )
}

pub fn enrich(sources: &Sources, report: &mut Report) {
    let ids: Vec<String> = report
        .sessions
        .iter()
        .filter(|s| s.kind == "codex")
        .filter_map(|s| s.id.strip_prefix("codex:").map(str::to_owned))
        .take(SESSION_LIMIT)
        .collect();
    #[cfg(windows)]
    let observations = proxy::query(&sources.codex, ids);
    #[cfg(not(windows))]
    let observations = {
        let _ = (sources, ids);
        Err(())
    };
    apply(report, observations);
    crate::claude_activity::enrich(sources, report);
}

fn apply(report: &mut Report, observations: Result<Vec<Observation>, ()>) {
    let connected = observations.is_ok();
    let observations = observations.unwrap_or_default();
    let mut confirmed = 0;
    for session in report.sessions.iter_mut().filter(|s| s.kind == "codex") {
        let found = observations
            .iter()
            .find(|o| session.id == format!("codex:{}", o.id) && session.origin == o.cwd);
        let (state, message) = found
            .map(|o| runtime_state(&o.status))
            .unwrap_or_else(unknown);
        session.state = state.into();
        session.message = message.into();
        session.request = None;
        session.command = None;
        confirmed += usize::from(state != "unknown");
    }
    let message = if connected {
        let count = if confirmed == 1 {
            "1 sessão".into()
        } else {
            format!("{confirmed} sessões")
        };
        format!(" Atividade local: {count} com estado confirmado. Respostas pelo Codex.")
    } else {
        " Atividade indisponível: daemon ausente, incompatível ou sem resposta; estado desconhecido.".into()
    };
    if let Some(integration) = report.integrations.iter_mut().find(|i| i.agent == "Codex") {
        integration.message.push_str(&message);
    } else {
        report.integrations.push(Integration {
            agent: "Codex".into(),
            message,
        });
    }
}

#[cfg(test)]
mod tests;
