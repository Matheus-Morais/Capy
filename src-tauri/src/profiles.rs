use crate::settings;
use serde::{Deserialize, Serialize};
use std::{path::{Path,PathBuf}, process::Command, sync::Mutex, time::{Duration, Instant},collections::HashMap};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all="camelCase")]
pub struct Profile {
    pub id:String,
    pub label:String,
    pub provider:String,
    pub config_dir:PathBuf,
    pub billing:String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all="camelCase")]
pub struct Identity {
    pub logged_in:bool,
    pub account:Option<String>,
    pub billing:String,
    pub message:String,
}
pub struct Store { pub root:PathBuf, value:Mutex<Vec<Profile>>, identities:Mutex<HashMap<String,(u64,Identity)>> }
impl Store {
    pub fn load(root:PathBuf) -> Self {
        let mut profiles:Vec<Profile>=settings::read_json(&root.join("profiles.json")).unwrap_or_default();
        profiles.retain(|p| validate_profile(p).is_ok());
        if profiles.is_empty() {
            profiles.push(Profile{id:"claude-default".into(),label:"Claude existente".into(),provider:"Claude".into(),config_dir:crate::discovery::Sources::local().claude,billing:"subscription".into()});
        }
        Self{root,value:Mutex::new(profiles),identities:Mutex::new(HashMap::new())}
    }
    pub fn list(&self)->Result<Vec<Profile>,String>{self.value.lock().map(|v|v.clone()).map_err(|_|"Perfis indisponíveis.".into())}
    pub fn get(&self,id:&str)->Result<Profile,String>{self.list()?.into_iter().find(|p|p.id==id).ok_or("Perfil não encontrado.".into())}
    pub fn probe(&self,profile:&Profile,cwd:Option<&Path>)->Result<Identity,String>{
        let identity=identity(profile,cwd)?;
        self.identities.lock().map_err(|_|"Identidades indisponíveis.")?.insert(profile.id.clone(),(crate::quotas::now_ms(),identity.clone()));Ok(identity)
    }
    pub fn candidates(&self,now:u64)->Vec<crate::routing::Candidate>{
        let Ok(identities)=self.identities.lock()else{return vec![];};
        self.list().unwrap_or_default().into_iter().filter_map(|profile|{
            let (at,identity)=identities.get(&profile.id)?;
            Some(crate::routing::Candidate{profile_id:profile.id,provider:profile.provider,account:identity.account.clone(),ready:identity.logged_in&&*at<=now&&now-*at<=120_000})
        }).collect()
    }
    pub fn add(&self,label:String,existing:Option<PathBuf>,billing:String)->Result<Profile,String>{
        if !settings::valid_text(&label,80) || !["subscription","api"].contains(&billing.as_str()) {return Err("Nome ou tipo de cobrança inválido.".into());}
        let mut list=self.value.lock().map_err(|_|"Perfis indisponíveis.")?;
        if list.len()>=64 {return Err("Limite de 64 perfis.".into());}
        let id=format!("claude-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos());
        let config_dir=match existing {
            Some(path)=> {if !path.is_absolute() || !path.is_dir(){return Err("Escolha uma pasta de configuração existente e absoluta.".into());} path.canonicalize().map_err(|e|e.to_string())?},
            None=>{let path=self.root.join("accounts").join(&id);std::fs::create_dir_all(&path).map_err(|e|e.to_string())?;path.canonicalize().map_err(|e|e.to_string())?}
        };
        if list.iter().any(|p|p.config_dir.canonicalize().ok().as_ref()==Some(&config_dir)) {return Err("Esta pasta já está cadastrada.".into());}
        let profile=Profile{id,label,provider:"Claude".into(),config_dir,billing};
        validate_profile(&profile)?;
        let mut updated=list.clone();updated.push(profile.clone());
        settings::write_json(&self.root.join("profiles.json"),&updated)?;
        *list=updated;Ok(profile)
    }
}
fn validate_profile(profile:&Profile)->Result<(),String>{
    if !settings::valid_text(&profile.label,80) || !profile.id.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'-') || profile.id.is_empty() || profile.id.len()>128
        || profile.provider!="Claude" || !profile.config_dir.is_absolute() || !["subscription","api"].contains(&profile.billing.as_str()) {
        return Err("Perfil incompatível.".into());
    }Ok(())
}
pub fn claude_executable()->Result<PathBuf,String>{
    let local=std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_default().join(".local/bin/claude.exe");
    if local.is_file(){return Ok(local);}
    let output=Command::new("where.exe").arg("claude.exe").output().map_err(|_|"Claude Code não está instalado.")?;
    String::from_utf8_lossy(&output.stdout).lines().map(|line|PathBuf::from(line.trim())).find(|p|p.is_absolute()&&p.is_file()).ok_or("Claude Code não está instalado.".into())
}
pub fn claude_command(profile:&Profile)->Result<Command,String>{
    let mut command=Command::new(claude_executable()?);
    if let Some(path)=config_override(profile){command.env("CLAUDE_CONFIG_DIR",path);}else{command.env_remove("CLAUDE_CONFIG_DIR");}
    for key in AUTH_OVERRIDE_VARS { command.env_remove(key); }
    #[cfg(windows)] {use std::os::windows::process::CommandExt;command.creation_flags(0x08000000);}
    Ok(command)
}
pub fn config_override(profile:&Profile)->Option<&Path>{
    let native=std::env::var_os("USERPROFILE").map(PathBuf::from).map(|home|home.join(".claude"));
    if std::env::var_os("CLAUDE_CONFIG_DIR").is_none()
        && profile.config_dir.canonicalize().ok().is_some_and(|path|native.and_then(|p|p.canonicalize().ok())==Some(path)) {None}
    else {Some(&profile.config_dir)}
}
pub const AUTH_OVERRIDE_VARS:&[&str]=&["ANTHROPIC_API_KEY","ANTHROPIC_AUTH_TOKEN","CLAUDE_CODE_OAUTH_TOKEN","ANTHROPIC_BASE_URL","CLAUDE_CODE_USE_BEDROCK","CLAUDE_CODE_USE_VERTEX","CLAUDE_CODE_USE_FOUNDRY"];
pub fn output(mut command:Command,timeout:Duration)->Result<Vec<u8>,String>{
    use std::io::Read;
    command.stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null()).stdin(std::process::Stdio::null());
    let mut child=command.spawn().map_err(|_|"Não foi possível iniciar o CLI oficial.")?;
    let stdout=child.stdout.take().ok_or("Saída do CLI indisponível.")?;
    let reader=std::thread::spawn(move||{let mut bytes=Vec::new();stdout.take(1_048_577).read_to_end(&mut bytes).map(|_|bytes)});
    let start=Instant::now();
    let result=loop{
        match child.try_wait(){
            Ok(Some(status))=>break Ok(status),
            Ok(None) if start.elapsed()<timeout=>std::thread::sleep(Duration::from_millis(25)),
            _=>{let _=child.kill();let _=child.wait();break Err("O CLI não respondeu no prazo de 15 segundos.");}
        }
    };
    let bytes=reader.join().map_err(|_|"Leitura do CLI indisponível.")?.map_err(|_|"Leitura do CLI falhou.")?;
    if bytes.len()>1_048_576{return Err("Resposta do CLI excede 1 MiB.".into());}
    let status=result?;
    if bytes.is_empty()&&!status.success(){return Err("O CLI não confirmou a autenticação.".into());}
    Ok(bytes)
}
pub fn identity(profile:&Profile,cwd:Option<&Path>)->Result<Identity,String>{
    let mut command=claude_command(profile)?;
    command.args(["auth","status","--json"]);
    if let Some(path)=cwd{command.current_dir(path);}
    let value:serde_json::Value=serde_json::from_slice(&output(command,Duration::from_secs(15))?).map_err(|_|"Resposta de autenticação incompatível.")?;
    parse_identity(profile,&value)
}
pub fn parse_identity(profile:&Profile,value:&serde_json::Value)->Result<Identity,String>{
    let config=value["configDirectory"].as_str().map(PathBuf::from).ok_or("Atualize o Claude Code: configDirectory não foi informado.")?;
    if config.canonicalize().ok()!=profile.config_dir.canonicalize().ok() || !config.is_dir() {return Err("O CLI respondeu por outra pasta de configuração.".into());}
    let method=value["authMethod"].as_str().unwrap_or("none");
    let billing=match method{"claude.ai"|"oauth_token"=>"subscription","api_key"|"api_key_helper"=>"api",_=>"unknown"};
    let logged_in=value["loggedIn"].as_bool()==Some(true)&&billing!="unknown";
    let account=value["email"].as_str().filter(|s|settings::valid_text(s,320)).map(str::to_owned);
    Ok(Identity{logged_in,account,billing:billing.into(),message:if logged_in {"Identidade confirmada pelo CLI oficial."}else{"Faça login oficial para usar este perfil."}.into()})
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
struct Launch<'a> { executable:PathBuf,config_dir:Option<&'a Path>, cwd:&'a Path, arguments:Vec<String>, remove_env:&'static [&'static str] }
pub fn external(profile:&Profile,cwd:&Path,arguments:Vec<String>,root:&Path)->Result<(),String>{
    if !cwd.is_dir(){return Err("A pasta de trabalho não existe.".into());}
    let launch_root=root.join("launches");
    std::fs::create_dir_all(&launch_root).map_err(|e|e.to_string())?;
    let script=launch_root.join("claude-terminal.ps1");
    std::fs::write(&script,include_str!("../scripts/claude-terminal.ps1")).map_err(|e|e.to_string())?;
    let name=format!("{}-{}.json",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos());
    let path=launch_root.join(name);
    settings::write_json(&path,&Launch{executable:claude_executable()?,config_dir:config_override(profile),cwd,arguments,remove_env:AUTH_OVERRIDE_VARS})?;
    let mut command=Command::new("powershell.exe");
    command.args(["-NoLogo","-NoProfile","-NoExit","-ExecutionPolicy","Bypass","-File"]).arg(&script).arg("-LaunchFile").arg(&path);
    #[cfg(windows)] {use std::os::windows::process::CommandExt;command.creation_flags(0x00000010);}
    command.spawn().map_err(|_|"Não foi possível abrir o terminal oficial.")?;
    Ok(())
}
pub fn login(profile:&Profile,root:&Path)->Result<(),String>{
    let mut args=vec!["auth".into(),"login".into()];
    if profile.billing=="api"{args.push("--console".into());}
    external(profile,&profile.config_dir,args,root)
}
pub fn enrich_sessions(store:&Store,sources:&crate::discovery::Sources,report:&mut crate::discovery::Report){
    let default=sources.claude.canonicalize().ok();
    for profile in store.list().unwrap_or_default(){
        if profile.config_dir.canonicalize().ok()==default{continue;}
        let profile_sources=crate::discovery::Sources{claude:profile.config_dir.clone(),codex:PathBuf::new(),antigravity:PathBuf::new()};
        let mut extra=crate::discovery::scan(&profile_sources);
        crate::claude_activity::enrich(&profile_sources,&mut extra);
        for session in extra.sessions.into_iter().filter(|s|s.kind=="claude"){
            if !report.sessions.iter().any(|s|s.id==session.id){report.sessions.push(session);}
        }
        for mut integration in extra.integrations.into_iter().filter(|i|i.agent=="Claude Code"){
            integration.agent=format!("Claude Code · {}",profile.label);report.integrations.push(integration);
        }
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn profiles_isolate_new_login_and_preserve_existing(){
        let root=std::env::temp_dir().join(format!("capy-profile-test-{}",std::process::id()));
        let existing=root.join("existing");std::fs::create_dir_all(&existing).unwrap();
        let secret=existing.join(".credentials.json");std::fs::write(&secret,"fixture-untouched").unwrap();
        let store=Store::load(root.clone());
        let imported=store.add("Existente".into(),Some(existing.clone()),"subscription".into()).unwrap();
        let new=store.add("Nova".into(),None,"subscription".into()).unwrap();
        assert_ne!(imported.config_dir,new.config_dir);
        assert!(new.config_dir.starts_with(root.canonicalize().unwrap()));
        assert!(!new.config_dir.join(".credentials.json").exists());
        assert_eq!(std::fs::read_to_string(&secret).unwrap(),"fixture-untouched");
        assert!(store.add("Duplicada".into(),Some(existing.clone()),"subscription".into()).is_err());
        let identity=serde_json::json!({"configDirectory":new.config_dir,"loggedIn":true,"authMethod":"claude.ai","email":"a@example.test"});
        assert_eq!(parse_identity(&new,&identity).unwrap().billing,"subscription");
        assert!(parse_identity(&imported,&identity).is_err());
        assert!(Store::load(root.clone()).list().unwrap().iter().any(|p|p.id==new.id));
        std::fs::remove_dir_all(root).unwrap();
    }
}
