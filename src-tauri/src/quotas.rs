use crate::discovery::Sources;
use serde::Serialize;
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

const REFRESH_MS: u64 = 60_000;
const TTL_MS: u64 = 120_000;

#[derive(Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Window {
    pub used_percent: f64,
    pub window_duration_mins: u64,
    pub resets_at: u64,
}
#[derive(Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub provider: String,
    pub account: Option<String>,
    pub bucket: Option<String>,
    pub period: Option<String>,
    pub window: Option<Window>,
    pub observed_at: Option<u64>,
    pub state: String,
    pub message: String,
}
pub struct Sample {
    account: String,
    buckets: Vec<(String, Value)>,
}
pub(crate) struct RawSample {
    pub before: Value,
    pub limits: Value,
    pub after: Value,
    pub identity_changed: bool,
}
impl RawSample {
    pub fn into_sample(self) -> Result<Sample, Error> {
        if self.identity_changed {
            return Err(Error::AccountChanged);
        }
        Sample::parse(self.before, self.limits, self.after)
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Error {
    Connection,
    NoAccount,
    UnsupportedAccount,
    NoIdentity,
    AccountChanged,
    InvalidLimits,
}
impl Error {
    fn message(self) -> &'static str {
        match self {
            Self::Connection => "A conexão com a fonte de cotas do Codex está indisponível.",
            Self::NoAccount => "A fonte do Codex não confirmou uma conta conectada.",
            Self::UnsupportedAccount => "Essa conta do Codex não fornece cotas do ChatGPT.",
            Self::NoIdentity => "A fonte não informou a identidade da conta. Saldo não associado.",
            Self::AccountChanged => "A conta mudou durante a leitura. Amostra descartada.",
            Self::InvalidLimits => "A fonte retornou cotas ausentes ou incompatíveis.",
        }
    }
}

fn text(value: &Value, max: usize) -> Option<&str> {
    value
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= max && !s.chars().any(char::is_control))
}
impl Sample {
    pub(crate) fn parse(before: Value, limits: Value, after: Value) -> Result<Self, Error> {
        let account = before
            .get("account")
            .filter(|a| a.is_object())
            .ok_or(Error::NoAccount)?;
        if account != after.get("account").ok_or(Error::AccountChanged)? {
            return Err(Error::AccountChanged);
        }
        if account["type"] != "chatgpt" {
            return Err(Error::UnsupportedAccount);
        }
        let email = text(&account["email"], 320)
            .filter(|s| s.contains('@'))
            .ok_or(Error::NoIdentity)?
            .to_owned();
        let buckets = if let Some(map) = limits.get("rateLimitsByLimitId").filter(|v| !v.is_null())
        {
            let map = map.as_object().ok_or(Error::InvalidLimits)?;
            if map.len() > 64 {
                return Err(Error::InvalidLimits);
            }
            map.iter()
                .map(|(id, bucket)| {
                    if text(&Value::String(id.clone()), 128).is_none()
                        || bucket["limitId"].as_str() != Some(id)
                    {
                        return Err(Error::InvalidLimits);
                    }
                    Ok((id.clone(), bucket.clone()))
                })
                .collect::<Result<Vec<_>, Error>>()?
        } else {
            let bucket = limits
                .get("rateLimits")
                .filter(|v| v.is_object())
                .ok_or(Error::InvalidLimits)?;
            let id = text(&bucket["limitId"], 128).ok_or(Error::InvalidLimits)?;
            vec![(id.to_owned(), bucket.clone())]
        };
        if buckets.is_empty() {
            return Err(Error::InvalidLimits);
        }
        if buckets
            .iter()
            .any(|(_, bucket)| serde_json::to_vec(bucket).map_or(true, |v| v.len() > 65_536))
        {
            return Err(Error::InvalidLimits);
        }
        Ok(Self {
            account: email,
            buckets,
        })
    }
    fn rows(&self, now: u64) -> Vec<Row> {
        let mut rows = Vec::new();
        for (bucket, value) in &self.buckets {
            for period in ["primary", "secondary"] {
                let window = parse_window(&value[period]);
                let fresh = window.as_ref().is_some_and(|w| {
                    w.resets_at
                        .checked_mul(1000)
                        .is_some_and(|reset| now < reset)
                });
                let provided = window.is_some();
                rows.push(Row {
                    provider: "Codex".into(),
                    account: Some(self.account.clone()),
                    bucket: Some(bucket.clone()),
                    period: Some(period.into()),
                    window: if fresh { window } else { None },
                    observed_at: Some(now),
                    state: if fresh {
                        "fresh"
                    } else if provided {
                        "stale"
                    } else {
                        "unavailable"
                    }
                    .into(),
                    message: if fresh {
                        "Fonte: Codex · atualiza a cada minuto."
                    } else if provided {
                        "A janela atingiu o horário de renovação. Aguardando nova leitura."
                    } else {
                        "A fonte não forneceu uma janela de cota válida."
                    }
                    .into(),
                });
            }
        }
        rows
    }
}
fn parse_window(value: &Value) -> Option<Window> {
    let used = value["usedPercent"].as_f64()?;
    let duration = value["windowDurationMins"].as_u64()?;
    let reset = value["resetsAt"].as_u64()?;
    (used.is_finite()
        && (0.0..=100.0).contains(&used)
        && duration > 0
        && duration <= 525_600
        && reset > 0
        && reset.checked_mul(1000).is_some())
    .then_some(Window {
        used_percent: used,
        window_duration_mins: duration,
        resets_at: reset,
    })
}
fn unavailable(provider: &str, message: &str) -> Row {
    Row {
        provider: provider.into(),
        account: None,
        bucket: None,
        period: None,
        window: None,
        observed_at: None,
        state: "unavailable".into(),
        message: message.into(),
    }
}
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
pub fn read(sources: &Sources) -> Result<Sample, Error> {
    #[cfg(windows)]
    {
        let raw = crate::activity::proxy::quota(&sources.codex).map_err(|_| Error::Connection)?;
        raw.into_sample()
    }
    #[cfg(not(windows))]
    {
        let _ = sources;
        Err(Error::Connection)
    }
}

#[derive(Default)]
pub struct Cache {
    attempted_at: Option<u64>,
    rows: Vec<Row>,
}
impl Cache {
    pub fn refresh(&mut self) { self.attempted_at = None; }
    pub fn poll(&mut self, sources: &Sources) -> Vec<Row> {
        self.rows(now_ms(), || read(sources));
        self.current(now_ms())
    }
    pub fn rows(&mut self, now: u64, read: impl FnOnce() -> Result<Sample, Error>) -> Vec<Row> {
        let due = self
            .attempted_at
            .is_none_or(|at| now.checked_sub(at).is_none_or(|age| age >= REFRESH_MS));
        if due {
            self.attempted_at = Some(now);
            match read() {
                Ok(sample) => self.rows = sample.rows(now),
                Err(error) => {
                    if self.rows.is_empty() {
                        self.rows.push(unavailable("Codex", error.message()));
                    }
                    for row in &mut self.rows {
                        row.window = None;
                        row.state = if row.account.is_some() {
                            "stale"
                        } else {
                            "unavailable"
                        }
                        .into();
                        row.message = if row.account.is_some() {
                            format!(
                                "{} A conta exibida é a última observada; saldo indisponível.",
                                error.message()
                            )
                        } else {
                            error.message().into()
                        };
                    }
                }
            }
        }
        self.current(now)
    }
    fn current(&mut self, now: u64) -> Vec<Row> {
        for row in &mut self.rows {
            if let Some(window) = &row.window {
                let valid = row
                    .observed_at
                    .is_some_and(|at| now.checked_sub(at).is_some_and(|age| age <= TTL_MS))
                    && window
                        .resets_at
                        .checked_mul(1000)
                        .is_some_and(|reset| now < reset);
                if !valid {
                    row.window = None;
                    row.state = "stale".into();
                    row.message = "Dado expirado. A conta exibida é a última observada; aguardando atualização.".into();
                }
            }
        }
        let mut rows = self.rows.clone();
        rows.push(unavailable(
            "Claude",
            "Fonte real de cotas ainda não conectada.",
        ));
        rows.push(unavailable(
            "Antigravity",
            "Fonte real de cotas ainda não conectada.",
        ));
        rows
    }
}

#[cfg(test)]
mod tests;
