use crate::quotas::{Row, Window};
use serde_json::Value;
use std::{fs::File, io::Read, path::Path, time::UNIX_EPOCH};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

fn status_path() -> Option<std::path::PathBuf> {
    std::env::var_os("AGY_STATUS_JSON").map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(|home| std::path::PathBuf::from(home).join("scripts/agy-statusline-input.json")))
}
pub fn cached_rows() -> Vec<Row> {
    status_path().and_then(|path| read(&path, crate::quotas::now_ms())).unwrap_or_else(|| vec![unavailable(
        "Statusline local do Antigravity ausente ou incompatível. Nenhum saldo confirmado.")])
}

#[derive(Default)]
pub struct Cache { attempted_at: Option<u64>, error: Option<&'static str> }
impl Cache {
    pub fn poll(&mut self, force: bool) -> Vec<Row> {
        self.poll_with(crate::quotas::now_ms(),force,collect,cached_rows)
    }
    fn poll_with(&mut self,now:u64,force:bool,update:impl FnOnce()->Result<(),&'static str>,read:impl FnOnce()->Vec<Row>)->Vec<Row> {
        if force || self.attempted_at.is_none_or(|at|now.checked_sub(at).is_none_or(|age|age>=60_000)) {
            self.attempted_at=Some(now);self.error=update().err();
        }
        let mut rows=read();
        for row in &mut rows {
            if let Some(error)=self.error {row.message=format!("{error} {}",row.message);}
            else if row.state=="fresh" {row.message="Consulta automática oficial agy /usage e statusline identificado. Atualiza a cada minuto.".into();}
        }
        rows
    }
}
fn collect() -> Result<(),&'static str> {
    let started=crate::quotas::now_ms();
    crate::agy_usage::collect(||status_path().and_then(|path|read(&path,crate::quotas::now_ms()))
        .is_some_and(|rows|complete_export(&rows,started)))?;
    let rows=status_path().and_then(|path|read(&path,crate::quotas::now_ms()))
        .ok_or("O agy respondeu, mas não exportou um statusline identificado compatível.")?;
    let finished=crate::quotas::now_ms();
    if !complete_export(&rows,started) || rows.iter().any(|q|q.observed_at.is_some_and(|at|at>finished)) {
        return Err("O agy respondeu, mas o statusline não gerou uma observação nova. Verifique a configuração do export.");
    }
    Ok(())
}
fn complete_export(rows:&[Row],started:u64)->bool {
    rows.len()==4 && rows.iter().all(|q|q.account.is_some() && q.observed_at.is_some_and(|at|at>=started))
        && rows.iter().any(|q|q.state=="fresh" && q.window.is_some())
}

fn unavailable(message: &str) -> Row {
    Row { provider:"Antigravity".into(), account:None, bucket:None, period:None,
        window:None, observed_at:None, state:"unavailable".into(), message:message.into() }
}

fn read(path: &Path, now: u64) -> Option<Vec<Row>> {
    let mut file = File::open(path).ok()?;
    let before = file.metadata().ok()?;
    if before.len() > 65_536 { return None; }
    let modified = before.modified().ok()?;
    let observed = u64::try_from(modified.duration_since(UNIX_EPOCH).ok()?.as_millis()).ok()?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file).take(65_537).read_to_end(&mut bytes).ok()?;
    let after = file.metadata().ok()?;
    if bytes.len() > 65_536 || before.len() != after.len() || modified != after.modified().ok()? { return None; }
    let value = serde_json::from_slice(&bytes).ok()?;
    parse(&value, observed, now)
}

fn parse(value: &Value, observed: u64, now: u64) -> Option<Vec<Row>> {
    let account = value["email"].as_str().filter(|s| !s.is_empty() && s.len() <= 320
        && s.contains('@') && !s.chars().any(char::is_control))?;
    let quotas = value["quota"].as_object()?;
    let fresh = now.checked_sub(observed).is_some_and(|age| age <= 120_000);
    Some([("gemini-5h","Gemini",300,"five_hour"), ("gemini-weekly","Gemini",10080,"seven_day"),
        ("3p-5h","Outros modelos",300,"five_hour"), ("3p-weekly","Outros modelos",10080,"seven_day")]
        .into_iter().map(|(key,bucket,duration,period)| {
            let source = quotas.get(key);
            let disabled = source.is_some_and(|q| q["disabled"] == true);
            let window = source.and_then(|q| {
                let remaining = q["remaining_fraction"].as_f64().filter(|r| r.is_finite() && (0.0..=1.0).contains(r))?;
                let reset = OffsetDateTime::parse(q["reset_time"].as_str()?, &Rfc3339).ok()?.unix_timestamp();
                let reset = u64::try_from(reset).ok()?;
                if reset.checked_mul(1000)? <= now { return None; }
                Some(Window { used_percent:100.0*(1.0-remaining),window_duration_mins:duration,resets_at:reset })
            });
            let valid = fresh && !disabled && window.is_some();
            let (state,message) = if disabled { ("unavailable","Esta janela está desabilitada na fonte.") }
                else if !fresh { ("stale","A observação local expirou. Aguarde nova atualização do statusline do Antigravity.") }
                else if window.is_none() { ("unavailable","Janela ausente, vencida ou incompatível. Saldo não estimado.") }
                else { ("fresh","Statusline local do Antigravity; conta e horário informados pela própria observação.") };
            Row { provider:"Antigravity".into(),account:Some(account.into()),bucket:Some(bucket.into()),period:Some(period.into()),
                window:if valid {window} else {None}, observed_at:Some(observed),state:state.into(),message:message.into() }
        }).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn sample() -> Value {
        let w=json!({"remaining_fraction":0.25,"reset_time":"2026-10-08T20:00:00Z"});
        json!({"email":"own@example.test","quota":{"gemini-5h":w,"gemini-weekly":w,"3p-5h":w,"3p-weekly":w}})
    }
    fn now() -> u64 { OffsetDateTime::parse("2026-10-08T19:00:00Z",&Rfc3339).unwrap().unix_timestamp() as u64 * 1000 }
    #[test] fn preserves_all_four_windows_and_identity() {
        let rows=parse(&sample(),now(),now()).unwrap(); assert_eq!(rows.len(),4);
        assert!(rows.iter().all(|q|q.account.as_deref()==Some("own@example.test") && q.window.as_ref().unwrap().used_percent==75.0));
        assert_eq!(rows.iter().filter(|q|q.period.as_deref()==Some("seven_day")).count(),2);
    }
    #[test] fn rejects_missing_identity_and_never_refreshes_observation_time() {
        let mut v=sample();v["email"]=Value::Null;assert!(parse(&v,now(),now()).is_none());
        for at in [now()-120_001,now()+1] { let rows=parse(&sample(),at,now()).unwrap();assert!(rows.iter().all(|q|q.window.is_none() && q.observed_at==Some(at))); }
    }
    #[test] fn disabled_invalid_and_expired_are_not_zero_balances() {
        let mut v=sample();v["quota"]["gemini-5h"]["disabled"]=json!(true);
        v["quota"]["gemini-weekly"]["remaining_fraction"]=json!(1.01);
        v["quota"]["3p-5h"]["reset_time"]=json!("invalid");
        v["quota"]["3p-weekly"]["reset_time"]=json!("2026-10-08T18:00:00Z");
        assert!(parse(&v,now(),now()).unwrap().iter().all(|q|q.window.is_none()));
    }
    #[test] fn missing_or_oversized_file_is_unavailable() {
        let dir=std::env::temp_dir().join(format!("capy-agy-{}",uuid::Uuid::new_v4()));std::fs::create_dir(&dir).unwrap();
        let path=dir.join("status.json");assert!(read(&path,now()).is_none());
        std::fs::write(&path,vec![b' ';65_537]).unwrap();assert!(read(&path,now()).is_none());
        std::fs::remove_file(path).unwrap();std::fs::remove_dir(dir).unwrap();
    }
    #[test] fn automatic_refresh_is_initial_periodic_and_forced() {
        use std::cell::Cell;
        let calls=Cell::new(0);let mut cache=Cache::default();
        for (at,force,expected) in [(1000,false,1),(60_999,false,1),(61_000,false,2),(61_001,true,3),(61_002,false,3)] {
            cache.poll_with(at,force,||{calls.set(calls.get()+1);Ok(())},Vec::new);
            assert_eq!(calls.get(),expected);
        }
    }
    #[test] fn automatic_failure_keeps_original_observation_and_expires() {
        let mut cache=Cache::default();
        for at in [now(),now()+120_001] {
            let rows=cache.poll_with(at,true,||Err("Own refresh failure."),||parse(&sample(),now(),at).unwrap());
            assert!(rows.iter().all(|q|q.observed_at==Some(now()) && q.message.contains("Own refresh failure.")));
            assert_eq!(rows[0].window.is_some(),at==now());
        }
    }
    #[test] fn automatic_export_waits_for_quotas_not_only_identity() {
        let mut intermediate=sample();intermediate["quota"]=json!({});
        assert!(!complete_export(&parse(&intermediate,now(),now()).unwrap(),now()));
        assert!(!complete_export(&parse(&sample(),now()-1,now()).unwrap(),now()));
        assert!(complete_export(&parse(&sample(),now(),now()).unwrap(),now()));
    }
    #[test]
    #[ignore="Read-only live agy /usage requires installed authenticated CLI and statusline export"]
    fn agy_automatic_live_updates_identified_sample_without_terminal() {
        let started=crate::quotas::now_ms();collect().unwrap();let rows=cached_rows();
        assert_eq!(rows.len(),4);assert!(rows.iter().all(|q|q.account.is_some() && q.observed_at.is_some_and(|at|at>=started)));
        assert!(rows.iter().any(|q|q.state=="fresh" && q.window.is_some()));
    }
}
