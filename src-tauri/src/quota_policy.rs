use crate::{quotas::Row, settings::{self, Preferences}};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf, sync::Mutex};

#[derive(Clone, Deserialize, Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Alert {
    pub id: String,
    pub provider: String,
    pub account: String,
    pub period: String,
    pub percent: u8,
    pub resets_at: u64,
    pub created_at: u64,
}
#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
struct Ledger { seen: BTreeMap<String, Vec<u8>>, alerts: Vec<Alert> }
pub struct Service { path: PathBuf, ledger: Mutex<Ledger> }
impl Service {
    pub fn load(path: PathBuf) -> Self {
        let ledger = settings::read_json(&path).unwrap_or_default();
        Self {path, ledger:Mutex::new(ledger)}
    }
    pub fn snapshot(&self) -> Vec<Alert> {
        self.ledger.lock().map(|l| l.alerts.clone()).unwrap_or_default()
    }
    pub fn dismiss(&self, id: &str) -> Result<(), String> {
        let mut ledger = self.ledger.lock().map_err(|_| "Alertas indisponíveis.")?;
        let previous = ledger.alerts.clone();
        ledger.alerts.retain(|a| a.id != id);
        if let Err(error) = settings::write_json(&self.path, &*ledger) { ledger.alerts = previous; return Err(error); }
        Ok(())
    }
    pub fn evaluate(&self, rows: &[Row], prefs: &Preferences, now: u64) -> Result<Vec<Alert>, String> {
        let mut ledger = self.ledger.lock().map_err(|_| "Alertas indisponíveis.")?;
        let previous = serde_json::to_vec(&*ledger).map_err(|e| e.to_string())?;
        evaluate(&mut ledger, rows, prefs, now);
        if serde_json::to_vec(&*ledger).map_err(|e|e.to_string())? != previous {
            if let Err(error) = settings::write_json(&self.path, &*ledger) {
                *ledger = serde_json::from_slice(&previous).map_err(|e| e.to_string())?;
                return Err(error);
            }
        }
        Ok(ledger.alerts.clone())
    }
}
pub fn fresh(row: &Row, now: u64) -> bool {
    row.state == "fresh" && row.account.as_deref().is_some_and(|a| settings::valid_text(a,320))
        && row.observed_at.is_some_and(|t| t <= now && now - t <= 120_000)
        && row.window.as_ref().is_some_and(|w| w.used_percent.is_finite() && (0.0..=100.0).contains(&w.used_percent)
            && w.window_duration_mins > 0 && w.resets_at.checked_mul(1000).is_some_and(|r| r > now))
}
fn evaluate(ledger: &mut Ledger, rows: &[Row], prefs: &Preferences, now: u64) {
    ledger.alerts.retain(|a| a.resets_at.saturating_mul(1000) > now);
    ledger.seen.retain(|key, _| serde_json::from_str::<(String,String,String,String,u64)>(key).is_ok_and(|k| k.4.saturating_mul(1000)>now));
    for row in rows.iter().filter(|r| fresh(r,now)) {
        let account = row.account.as_ref().unwrap();
        let window = row.window.as_ref().unwrap();
        let rule = prefs.quota_rules.iter().find(|r| r.provider == row.provider && r.account == *account);
        let thresholds = rule.map(|r| r.thresholds.clone()).unwrap_or_else(settings::default_thresholds);
        let key = serde_json::to_string(&(&row.provider,account,row.bucket.as_deref().unwrap_or(""),row.period.as_deref().unwrap_or(""),window.resets_at)).unwrap();
        let seen = ledger.seen.entry(key.clone()).or_default();
        let crossed: Vec<u8> = thresholds.into_iter().filter(|t| t.enabled && f64::from(t.percent)<=window.used_percent && !seen.contains(&t.percent)).map(|t|t.percent).collect();
        if let Some(percent) = crossed.iter().max().copied() {
            seen.extend(crossed);
            let id = format!("{key}:{percent}");
            ledger.alerts.push(Alert {id,provider:row.provider.clone(),account:account.clone(),period:row.period.clone().unwrap_or_default(),percent,resets_at:window.resets_at,created_at:now});
        }
    }
    if ledger.alerts.len()>100 { ledger.alerts.drain(..ledger.alerts.len()-100); }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quotas::Window;
    fn row(account: &str, percent: f64, reset:u64) -> Row { Row {provider:"Claude".into(),account:Some(account.into()),bucket:Some("subscription".into()),period:Some("five_hour".into()),window:Some(Window{used_percent:percent,window_duration_mins:300,resets_at:reset}),observed_at:Some(1_000),state:"fresh".into(),message:String::new()} }
    #[test]
    fn quota_policy_groups_crossings_per_account_window_and_reset() {
        let mut ledger=Ledger::default(); let prefs=Preferences::default();
        evaluate(&mut ledger,&[row("a",75.0,1000)],&prefs,1000);
        assert_eq!(ledger.alerts.iter().map(|a|a.percent).collect::<Vec<_>>(),vec![70]);
        evaluate(&mut ledger,&[row("a",75.0,1000)],&prefs,2000);
        assert_eq!(ledger.alerts.len(),1);
        evaluate(&mut ledger,&[row("a",95.0,1000),row("b",95.0,1000)],&prefs,3000);
        assert_eq!(ledger.alerts.iter().map(|a|a.percent).collect::<Vec<_>>(),vec![70,90,90]);
        ledger.alerts.clear();
        evaluate(&mut ledger,&[row("a",95.0,1000)],&prefs,4000);
        assert!(ledger.alerts.is_empty());
        evaluate(&mut ledger,&[row("a",95.0,2000)],&prefs,5000);
        assert_eq!(ledger.alerts.len(),1);
    }
    #[test]
    fn quota_policy_custom_disabled_and_invalid_samples() {
        let mut prefs=Preferences::default();
        prefs.quota_rules.push(crate::settings::QuotaRule {provider:"Claude".into(),account:"a".into(),thresholds:vec![settings::Threshold{percent:55,enabled:true},settings::Threshold{percent:60,enabled:false}],five_hour_trigger:None,weekly_trigger:None,fallback:vec![]});
        let mut ledger=Ledger::default(); evaluate(&mut ledger,&[row("a",65.0,1000)],&prefs,1000);
        assert_eq!(ledger.alerts[0].percent,55);
        for mode in 0..6 {
            let mut invalid=row("b",90.0,1000);
            match mode {0=>invalid.state="stale".into(),1=>invalid.account=None,2=>invalid.observed_at=Some(1001),3=>invalid.observed_at=Some(0),4=>invalid.window.as_mut().unwrap().used_percent=f64::NAN,_=>invalid.window.as_mut().unwrap().resets_at=1};
            let now=if mode==3 {120001} else {1000};
            assert!(!fresh(&invalid,now));
            evaluate(&mut ledger,&[invalid],&prefs,now);
        }
        assert_eq!(ledger.alerts.len(),1);
    }
    #[test]
    fn quota_policy_ledger_survives_restart_and_dismissal() {
        let path=std::env::temp_dir().join(format!("capy-ledger-{}.json",std::process::id()));
        let _=std::fs::remove_file(&path);
        let service=Service::load(path.clone());
        let alerts=service.evaluate(&[row("a",90.0,1000)],&Preferences::default(),1000).unwrap();
        service.dismiss(&alerts[0].id).unwrap();
        let service=Service::load(path.clone());
        assert!(service.evaluate(&[row("a",90.0,1000)],&Preferences::default(),2000).unwrap().is_empty());
        let _=std::fs::remove_file(path);
    }
}
