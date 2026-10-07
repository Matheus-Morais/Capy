use crate::{discovery, profiles::{self, Profile}, quotas::{Row, Window}, settings};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{io::Read, path::{Path, PathBuf}};

#[derive(Deserialize,Serialize)]
#[serde(rename_all="camelCase")]
struct Observation { session_id:String, account:String, observed_at:u64, started:bool, windows:Value }
#[derive(Deserialize,Serialize)]
#[serde(rename_all="camelCase")]
struct Bridge { executable:PathBuf, config_dir:PathBuf, original:Option<Value>, bash:Option<PathBuf> }
fn root(profile:&Profile)->PathBuf {profile.config_dir.join("capy-quotas")}
fn wrapper_command(profile:&Profile)->Result<String,String>{
    let path=root(profile).join("statusline.ps1");
    let native=path.to_string_lossy();
    let native=native.strip_prefix(r"\\?\").unwrap_or(&native);
    let path=if let Some(unc)=native.strip_prefix(r"UNC\") {format!("//{}",unc.replace('\\',"/"))}else{native.replace('\\',"/")};
    if path.chars().any(|c|matches!(c,'"'|'$'|'`'|'\n'|'\r')) {return Err("A pasta contém caracteres incompatíveis com a configuração de statusline.".into());}
    Ok(format!("powershell -NoProfile -ExecutionPolicy Bypass -File \"{path}\""))
}
fn bash()->Option<PathBuf>{
    let output=std::process::Command::new("where.exe").arg("git.exe").output().ok()?;
    for path in String::from_utf8_lossy(&output.stdout).lines() {
        let parent=Path::new(path.trim()).parent()?.parent()?;
        let candidate=parent.join("bin/bash.exe"); if candidate.is_file(){return Some(candidate);}
    }
    None
}
pub fn configure(profile:&Profile,enabled:bool)->Result<(),String>{
    let folder=root(profile);std::fs::create_dir_all(&folder).map_err(|e|e.to_string())?;
    let path=profile.config_dir.join("settings.json");
    let original_bytes=std::fs::read(&path).ok();
    let mut config=match original_bytes.as_ref(){Some(bytes)=>serde_json::from_slice::<Value>(bytes).map_err(|_|"settings.json contém JSON inválido.")?,None=>json!({})};
    let object=config.as_object_mut().ok_or("settings.json deve conter um objeto JSON.")?;
    let command=wrapper_command(profile)?;
    let bridge_path=folder.join("bridge.json");
    if enabled {
        let current=object.get("statusLine").cloned();
        let previous=if current.as_ref().is_some_and(|v|v["command"]==command){
            settings::read_json::<Bridge>(&bridge_path)?.original
        }else{current};
        if previous.as_ref().is_some_and(|v|v["type"]!="command"||v["command"].as_str().is_none()) {return Err("A statusline existente não é um comando compatível.".into());}
        let bridge=Bridge{executable:std::env::current_exe().map_err(|e|e.to_string())?,config_dir:profile.config_dir.clone(),original:previous,bash:bash()};
        settings::write_json(&bridge_path,&bridge)?;
        std::fs::write(folder.join("statusline.ps1"),include_str!("../scripts/claude-statusline.ps1")).map_err(|e|e.to_string())?;
        let mut status=bridge.original.unwrap_or_else(||json!({}));
        status["type"]=json!("command");status["command"]=json!(command);
        status.as_object_mut().unwrap().remove("refreshInterval");
        object.insert("statusLine".into(),status);
    }else{
        if object.get("statusLine").is_some_and(|v|v["command"]!=command){return Err("A statusline mudou na origem. A Capy preservou a configuração atual.".into());}
        if let Ok(bridge)=settings::read_json::<Bridge>(&bridge_path){
            match bridge.original{Some(value)=>{object.insert("statusLine".into(),value);},None=>{object.remove("statusLine");}}
        }else {object.remove("statusLine");}
    }
    if std::fs::read(&path).ok()!=original_bytes{return Err("Configuração alterada durante a edição; tente novamente.".into());}
    settings::write_json(&path,&config)?;
    let disabled=folder.join("disabled");
    if enabled{if disabled.exists(){std::fs::remove_file(disabled).map_err(|e|e.to_string())?;}}
    else{std::fs::write(disabled,"disabled").map_err(|e|e.to_string())?;}
    Ok(())
}
pub fn enabled(profile:&Profile)->bool{
    !root(profile).join("disabled").exists() && settings::read_json::<Value>(&profile.config_dir.join("settings.json")).ok().is_some_and(|v|wrapper_command(profile).is_ok_and(|cmd|v["statusLine"]["command"]==cmd))
}

pub fn hook(config_dir:PathBuf)->Result<(),String>{
    let profile=Profile{id:"statusline".into(),label:"Statusline".into(),provider:"Claude".into(),config_dir,billing:"subscription".into()};
    if !enabled(&profile){return Ok(());}
    let mut bytes=Vec::new();std::io::stdin().take(65_537).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
    if bytes.len()>65_536 {return Err("Statusline excede 64 KiB.".into());}
    let payload:Value=serde_json::from_slice(&bytes).map_err(|_|"Statusline inválida.")?;
    let session=payload["session_id"].as_str().filter(|s|discovery::uuid(s)).ok_or("ID de sessão inválido.")?;
    let identity=profiles::identity(&profile,None)?;
    let account=identity.account.filter(|_|identity.logged_in&&identity.billing=="subscription").ok_or("Conta de assinatura não confirmada.")?;
    let path=root(&profile).join(format!("{session}.json"));
    let previous=settings::read_json::<Observation>(&path).ok();
    let observation=observe(session,&account,&payload,previous,crate::quotas::now_ms());
    settings::write_json(&path,&observation)?;
    let rows=sample_rows(&observation,Some(&account),crate::quotas::now_ms());
    let visible:Vec<String>=rows.into_iter().filter_map(|r|r.window.map(|w|format!("{} {:.0}%",if r.period.as_deref()==Some("five_hour"){ "5h" }else{"semana"},w.used_percent))).collect();
    if !visible.is_empty(){println!("Capy · {}",visible.join(" · "));}
    Ok(())
}
fn observe(session:&str,account:&str,payload:&Value,previous:Option<Observation>,now:u64)->Observation{
    let has_limits=payload["rate_limits"].as_object().is_some_and(|m|!m.is_empty());
    let started=previous.as_ref().is_some_and(|p|p.account==account&&p.started)||!has_limits;
    Observation{session_id:session.into(),account:account.into(),started,observed_at:now,windows:if started{payload["rate_limits"].clone()}else{Value::Null}}
}
fn sample_rows(sample:&Observation,current:Option<&str>,now:u64)->Vec<Row>{
    if !sample.started||current!=Some(sample.account.as_str()) {return vec![];}
    [ ("five_hour",300),("seven_day",10080) ].into_iter().map(|(period,minutes)|{
        let value=&sample.windows[period];
        let window=value["used_percentage"].as_f64().filter(|v|v.is_finite()&&(0.0..=100.0).contains(v)).zip(value["resets_at"].as_u64()).map(|(percent,reset)|Window{used_percent:percent,window_duration_mins:minutes,resets_at:reset});
        let fresh=sample.observed_at<=now&&now-sample.observed_at<=120_000&&window.as_ref().is_some_and(|w|w.resets_at.checked_mul(1000).is_some_and(|t|t>now));
        let provided=window.is_some();
        Row{provider:"Claude".into(),account:Some(sample.account.clone()),bucket:Some("subscription".into()),period:Some(period.into()),window:if fresh{window}else{None},observed_at:Some(sample.observed_at),state:if fresh{"fresh"}else if provided{"stale"}else{"unavailable"}.into(),message:if fresh{"Fonte oficial: statusline Claude Code."}else{"Sem amostra recente desta janela."}.into()}
    }).collect()
}
pub fn poll(profile:&Profile)->Vec<Row>{
    let unavailable=|message:&str|vec![Row{provider:"Claude".into(),account:None,bucket:Some(profile.label.clone()),period:None,window:None,observed_at:None,state:"unavailable".into(),message:message.into()}];
    if !enabled(profile){return unavailable("Conecte as quotas desta conta pelo painel.");}
    let identity=match profiles::identity(profile,None){Ok(identity)=>identity,Err(_)=>return unavailable("O CLI não confirmou a identidade desta conta.")};
    if !identity.logged_in||identity.billing!="subscription"{return unavailable("Quotas da assinatura não disponíveis para este login.");}
    let entries=match std::fs::read_dir(root(profile)){Ok(entries)=>entries,Err(_)=>return unavailable("Ainda não há amostras da statusline.")};
    let mut samples:Vec<Observation>=entries.filter_map(Result::ok).take(1024).filter_map(|e|{
        let path=e.path();
        if !path.file_stem().and_then(|s|s.to_str()).is_some_and(discovery::uuid){return None;}
        settings::read_json::<Observation>(&path).ok().filter(|s|s.started&&Some(s.account.as_str())==identity.account.as_deref())
    }).collect();
    samples.sort_by_key(|s|std::cmp::Reverse(s.observed_at));
    match samples.first(){Some(sample)=>sample_rows(sample,identity.account.as_deref(),crate::quotas::now_ms()),None=>unavailable("Abra uma nova sessão Claude após conectar e aguarde sua primeira resposta.")}
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn claude_quota_identity_start_windows_and_ttl(){
        let session="11111111-1111-1111-1111-111111111111";
        let limits=json!({"rate_limits":{"five_hour":{"used_percentage":51.5,"resets_at":1000},"seven_day":{"used_percentage":82,"resets_at":2000}}});
        let unknown=observe(session,"a",&limits,None,1000);
        assert!(sample_rows(&unknown,Some("a"),1000).is_empty());
        let startup=observe(session,"a",&json!({}),None,900);
        let sample=observe(session,"a",&limits,Some(startup),1000);
        let rows=sample_rows(&sample,Some("a"),1000);
        assert_eq!(rows.len(),2);assert_eq!(rows[0].window.as_ref().unwrap().used_percent,51.5);
        assert!(sample_rows(&sample,Some("b"),1000).is_empty());
        assert!(sample_rows(&sample,Some("a"),121001).iter().all(|r|r.state=="stale"&&r.window.is_none()));
        assert!(sample_rows(&sample,Some("a"),999).iter().all(|r|r.window.is_none()));
        let changed=observe(session,"b",&limits,Some(sample),2000);
        assert!(sample_rows(&changed,Some("b"),2000).is_empty());
    }
    #[test]
    fn claude_quota_missing_and_invalid_windows_are_unavailable(){
        for value in [json!({}),json!({"used_percentage":101,"resets_at":1000}),json!({"used_percentage":50,"resets_at":0})] {
            let sample=Observation{session_id:"s".into(),account:"a".into(),started:true,observed_at:1000,windows:json!({"five_hour":value})};
            assert!(sample_rows(&sample,Some("a"),1000).iter().all(|r|r.window.is_none()));
        }
    }
    #[test]
    fn claude_quota_configuration_preserves_and_restores_custom_statusline(){
        let dir=std::env::temp_dir().join(format!("capy-statusline-test-{}",std::process::id()));std::fs::create_dir_all(&dir).unwrap();
        let profile=Profile{id:"test".into(),label:"Test".into(),provider:"Claude".into(),config_dir:dir.clone(),billing:"subscription".into()};
        let original=json!({"model":"sonnet","statusLine":{"type":"command","command":"echo original","padding":2,"refreshInterval":10},"hooks":{"Stop":[]}});
        settings::write_json(&dir.join("settings.json"),&original).unwrap();
        configure(&profile,true).unwrap();assert!(enabled(&profile));
        configure(&profile,true).unwrap();
        let updated:Value=settings::read_json(&dir.join("settings.json")).unwrap();assert_eq!(updated["model"],"sonnet");assert_eq!(updated["hooks"],original["hooks"]);
        configure(&profile,false).unwrap();assert!(!enabled(&profile));
        assert_eq!(settings::read_json::<Value>(&dir.join("settings.json")).unwrap(),original);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
