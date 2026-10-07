use serde::{Deserialize, Serialize};
use std::{fs, io::Read, path::{Path, PathBuf}, sync::Mutex};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Threshold { pub percent: u8, pub enabled: bool }

pub fn default_thresholds() -> Vec<Threshold> {
    [50, 60, 70, 80, 90].into_iter().map(|percent| Threshold { percent, enabled: true }).collect()
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QuotaRule {
    pub provider: String,
    pub account: String,
    pub thresholds: Vec<Threshold>,
    #[serde(default)]
    pub five_hour_trigger: Option<u8>,
    #[serde(default)]
    pub weekly_trigger: Option<u8>,
    #[serde(default)]
    pub fallback: Vec<Destination>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Destination {
    pub profile_id: String,
    pub model: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Preferences {
    pub sounds: bool,
    pub reduce_motion: bool,
    pub quota_rules: Vec<QuotaRule>,
}

impl Preferences {
    pub fn validate(&self) -> Result<(), String> {
        if self.quota_rules.len() > 64 { return Err("Limite de 64 regras por conta.".into()); }
        let mut accounts = std::collections::HashSet::new();
        for rule in &self.quota_rules {
            if !valid_text(&rule.provider, 64) || !valid_text(&rule.account, 320)
                || !accounts.insert((&rule.provider, &rule.account)) {
                return Err("Provedor/conta inválidos ou duplicados.".into());
            }
            let mut percents = std::collections::HashSet::new();
            if rule.thresholds.len() > 100 || rule.thresholds.iter().any(|t| t.percent == 0 || t.percent > 100 || !percents.insert(t.percent)) {
                return Err("Alertas devem ter percentuais únicos entre 1% e 100%.".into());
            }
            if [rule.five_hour_trigger, rule.weekly_trigger].into_iter().flatten().any(|p| p == 0 || p > 100) {
                return Err("Gatilhos devem estar entre 1% e 100%, ou desligados.".into());
            }
            if rule.fallback.len() > 32 || rule.fallback.iter().any(|d| !valid_text(&d.profile_id, 128) || !valid_text(&d.model, 128)) {
                return Err("Cadeia de fallback inválida.".into());
            }
        }
        Ok(())
    }
}
pub(crate) fn valid_text(text: &str, max: usize) -> bool {
    !text.trim().is_empty() && text.len() <= max && !text.chars().any(char::is_control)
}

pub(crate) fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    read_json_limit(path,1_048_576)
}
pub(crate) fn read_json_limit<T: serde::de::DeserializeOwned>(path: &Path,limit:u64) -> Result<T, String> {
    let mut bytes = Vec::new();
    fs::File::open(path).map_err(|e| e.to_string())?.take(limit+1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit { return Err(format!("Arquivo excede o limite de {limit} bytes.")); }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}
pub(crate) fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    write_json_limit(path,value,1_048_576)
}
pub(crate) fn write_json_limit<T: Serialize>(path: &Path,value:&T,limit:u64)->Result<(),String>{
    let parent = path.parent().ok_or("Caminho de configuração inválido.")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    if bytes.len() as u64>limit{return Err(format!("Estado excede o limite de {limit} bytes; nenhuma alteração foi salva."));}
    let temp=parent.join(format!(".{}.{}.pending",path.file_name().ok_or("Nome de arquivo inválido.")?.to_string_lossy(),uuid::Uuid::new_v4()));
    let result=(||{fs::write(&temp, bytes).map_err(|e| e.to_string())?;fs::rename(&temp, path).map_err(|e| e.to_string())})();
    if result.is_err(){let _=fs::remove_file(&temp);}result
}

pub struct Store { path: PathBuf, value: Mutex<Preferences> }
impl Store {
    pub fn load(path: PathBuf) -> Self {
        let value = read_json::<Preferences>(&path).ok().filter(|p| p.validate().is_ok()).unwrap_or_default();
        Self { path, value: Mutex::new(value) }
    }
    pub fn get(&self) -> Result<Preferences, String> {
        self.value.lock().map(|p| p.clone()).map_err(|_| "Preferências indisponíveis.".into())
    }
    pub fn save(&self, value: Preferences) -> Result<(), String> {
        value.validate()?;
        let mut current = self.value.lock().map_err(|_| "Preferências indisponíveis.")?;
        write_json(&self.path, &value)?;
        *current = value;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preferences_roundtrip_defaults_and_validation() {
        let path = std::env::temp_dir().join(format!("capy-settings-{}.json",std::process::id()));
        let store = Store::load(path.clone());
        assert!(!store.get().unwrap().sounds);
        let rule = QuotaRule { provider:"Claude".into(), account:"a@example.test".into(), thresholds:default_thresholds(), five_hour_trigger:Some(90), weekly_trigger:Some(80), fallback:vec![] };
        let value = Preferences { sounds:true, reduce_motion:true, quota_rules:vec![rule] };
        store.save(value.clone()).unwrap();
        assert_eq!(Store::load(path.clone()).get().unwrap(), value);
        let mut invalid = value.clone(); invalid.quota_rules[0].thresholds.push(Threshold {percent:90,enabled:false});
        assert!(store.save(invalid).is_err());
        assert_eq!(store.get().unwrap(), value);
        let _ = fs::remove_file(path);
    }
}
