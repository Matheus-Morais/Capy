use crate::settings;
use serde::{Deserialize, Serialize};
use std::{path::{Path,PathBuf}, process::Command, sync::Mutex, time::{Duration, Instant},collections::HashMap};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
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
struct ProfileState { profiles:Vec<Profile>, persisted:Option<Vec<u8>> }
pub struct Store { pub root:PathBuf, value:Mutex<ProfileState>, load_error:bool, identities:Mutex<HashMap<String,(u64,Identity)>> }
const PRESERVED:&str="Perfis incompatíveis ou indisponíveis. O arquivo profiles.json foi preservado; restaure o arquivo e reinicie a Capy.";
fn profile_bytes(path:&Path)->Result<Option<Vec<u8>>,String>{
    use std::io::Read;
    let file=match std::fs::File::open(path){Ok(file)=>file,Err(e) if e.kind()==std::io::ErrorKind::NotFound=>return Ok(None),Err(_)=>return Err(PRESERVED.into())};
    let mut bytes=Vec::new();file.take(1_048_577).read_to_end(&mut bytes).map_err(|_|PRESERVED)?;
    if bytes.len()>1_048_576{return Err(PRESERVED.into());}Ok(Some(bytes))
}
fn validate_profiles(profiles:&[Profile])->Result<(),String>{
    let mut ids=std::collections::HashSet::new();let mut paths=std::collections::HashSet::new();
    if profiles.len()>64{return Err(PRESERVED.into());}
    for profile in profiles{validate_profile(profile).map_err(|_|PRESERVED)?;
        if !ids.insert(&profile.id)||!paths.insert(profile.config_dir.canonicalize().unwrap_or_else(|_|profile.config_dir.clone())){return Err(PRESERVED.into());}
    }Ok(())
}
fn new_config_dir(root:&Path,id:&str)->Result<PathBuf,String>{
    std::fs::create_dir_all(root).map_err(|e|e.to_string())?;
    let parent=root.join("accounts");std::fs::create_dir_all(&parent).map_err(|e|e.to_string())?;
    let parent=parent.canonicalize().map_err(|e|e.to_string())?;
    if !parent.starts_with(root.canonicalize().map_err(|e|e.to_string())?){return Err("A pasta de contas aponta para fora do diretório da Capy.".into());}
    let path=parent.join(id);std::fs::create_dir(&path).map_err(|e|e.to_string())?;
    path.canonicalize().map_err(|e|e.to_string())
}
impl Store {
    pub fn load(root:PathBuf) -> Self {
        let loaded=profile_bytes(&root.join("profiles.json")).and_then(|bytes|{
            let profiles=match &bytes{Some(value)=>serde_json::from_slice::<Vec<Profile>>(value).map_err(|_|PRESERVED)?,None=>vec![]};
            validate_profiles(&profiles)?;Ok(ProfileState{profiles,persisted:bytes})
        });
        let load_error=loaded.is_err();let mut value=loaded.unwrap_or(ProfileState{profiles:vec![],persisted:None});
        if !load_error && value.profiles.is_empty() {
            value.profiles.push(Profile{id:"claude-default".into(),label:"Claude existente".into(),provider:"Claude".into(),config_dir:crate::discovery::Sources::local().claude,billing:"subscription".into()});
        }
        Self{root,value:Mutex::new(value),load_error,identities:Mutex::new(HashMap::new())}
    }
    fn unchanged(&self,value:&ProfileState)->Result<(),String>{
        if self.load_error{return Err(PRESERVED.into());}
        if profile_bytes(&self.root.join("profiles.json"))?!=value.persisted{return Err("Perfis alterados fora da Capy. Arquivo preservado; reinicie para carregar a alteração.".into());}Ok(())
    }
    pub fn list(&self)->Result<Vec<Profile>,String>{let value=self.value.lock().map_err(|_|"Perfis indisponíveis.")?;self.unchanged(&value)?;Ok(value.profiles.clone())}
    pub fn get(&self,id:&str)->Result<Profile,String>{self.list()?.into_iter().find(|p|p.id==id).ok_or("Perfil não encontrado.".into())}
    pub fn probe(&self,profile:&Profile,cwd:Option<&Path>)->Result<Identity,String>{
        if self.get(&profile.id)?!=*profile{return Err("Perfil alterado; verifique novamente.".into());}
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
    pub fn chat_targets(&self,now:u64)->Vec<crate::chat_history::Target>{
        let Ok(profiles)=self.list()else{return vec![];};
        let Ok(identities)=self.identities.lock()else{return vec![];};
        profiles.into_iter().filter_map(|profile|{
            let (at,identity)=identities.get(&profile.id)?;
            if !identity.logged_in||*at>now||now-*at>120_000||identity.billing!=profile.billing{return None;}
            let target=crate::chat_history::Target{kind:"claudeCli".into(),profile_id:profile.id,provider:profile.provider,
                account:identity.account.clone()?,billing:identity.billing.clone(),credential_revision:None};
            crate::chat_history::valid_target(&target).then_some(target)
        }).collect()
    }
    pub fn add(&self,label:String,existing:Option<PathBuf>,billing:String)->Result<Profile,String>{
        if !settings::valid_text(&label,80) || !["subscription","api"].contains(&billing.as_str()) {return Err("Nome ou tipo de cobrança inválido.".into());}
        let mut value=self.value.lock().map_err(|_|"Perfis indisponíveis.")?;self.unchanged(&value)?;
        let list=&value.profiles;
        if list.len()>=64 {return Err("Limite de 64 perfis.".into());}
        let id=format!("claude-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos());
        let config_dir=match existing {
            Some(path)=> {if !path.is_absolute() || !path.is_dir(){return Err("Escolha uma pasta de configuração existente e absoluta.".into());} path.canonicalize().map_err(|e|e.to_string())?},
            None=>new_config_dir(&self.root,&id)?
        };
        if list.iter().any(|p|p.config_dir.canonicalize().ok().as_ref()==Some(&config_dir)) {return Err("Esta pasta já está cadastrada.".into());}
        let profile=Profile{id,label,provider:"Claude".into(),config_dir,billing};
        validate_profile(&profile)?;
        let mut updated=list.clone();updated.push(profile.clone());
        validate_profiles(&updated)?;self.unchanged(&value)?;
        let persisted=serde_json::to_vec_pretty(&updated).map_err(|e|e.to_string())?;
        settings::write_json(&self.root.join("profiles.json"),&updated)?;
        value.profiles=updated;value.persisted=Some(persisted);Ok(profile)
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
    fn fixture()->PathBuf{let root=std::env::temp_dir().join(format!("capy-profiles-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&root).unwrap();root}
    fn sample(root:&Path)->Profile{Profile{id:"fixture".into(),label:"Conta".into(),provider:"Claude".into(),config_dir:root.join("config"),billing:"subscription".into()}}
    #[test]
    fn profiles_chat_targets_require_recent_exact_identity_and_billing(){
        let root=fixture();let profile=sample(&root);std::fs::write(root.join("profiles.json"),serde_json::to_vec(&vec![profile.clone()]).unwrap()).unwrap();
        let store=Store::load(root.clone());let identity=Identity{logged_in:true,account:Some("own@example.invalid".into()),billing:"subscription".into(),message:String::new()};
        assert!(store.chat_targets(1000).is_empty());
        store.identities.lock().unwrap().insert(profile.id.clone(),(1000,identity.clone()));
        let targets=store.chat_targets(1000);assert_eq!(targets.len(),1);assert_eq!(targets[0].account,"own@example.invalid");assert_eq!(targets[0].billing,"subscription");assert_eq!(targets[0].profile_id,profile.id);
        assert_eq!(store.chat_targets(121000).len(),1);
        assert!(store.chat_targets(999).is_empty());assert!(store.chat_targets(121001).is_empty());
        for field in ["login","account","empty","billing"]{let mut changed=identity.clone();match field{"login"=>changed.logged_in=false,"account"=>changed.account=None,"empty"=>changed.account=Some(" ".into()),_=>changed.billing="api".into()};
            store.identities.lock().unwrap().insert(profile.id.clone(),(1000,changed));assert!(store.chat_targets(1000).is_empty(),"{field}");
        }
        store.identities.lock().unwrap().insert(profile.id.clone(),(1000,identity));assert_eq!(store.chat_targets(1000).len(),1);
        std::fs::write(root.join("profiles.json"),b"[]").unwrap();assert!(store.chat_targets(1000).is_empty());std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn profiles_invalid_metadata_blocks_operations_without_overwriting(){
        let root=fixture();let path=root.join("profiles.json");let profile=sample(&root);
        let mut unknown=serde_json::to_value(&profile).unwrap();unknown["futureField"]=true.into();
        let mut invalid=profile.clone();invalid.provider="Unknown".into();
        for bytes in [b"broken".to_vec(),serde_json::to_vec(&vec![unknown]).unwrap(),serde_json::to_vec(&vec![invalid]).unwrap(),vec![b' ';1_048_577]]{
            std::fs::write(&path,&bytes).unwrap();let store=Store::load(root.clone());
            assert!(store.list().is_err());assert!(store.get("fixture").is_err());assert!(store.probe(&profile,None).is_err());
            assert!(store.add("Nova".into(),None,"subscription".into()).is_err());
            assert_eq!(std::fs::read(&path).unwrap(),bytes);assert!(!root.join("accounts").exists());
        }std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn profiles_duplicate_metadata_is_preserved(){
        let root=fixture();let profile=sample(&root);let mut same_path=profile.clone();same_path.id="other".into();
        let mut same_id=profile.clone();same_id.config_dir=root.join("different");
        let many=(0..65).map(|i|Profile{id:format!("p-{i}"),config_dir:root.join(format!("config-{i}")),..profile.clone()}).collect();
        for profiles in [vec![profile.clone(),same_path],vec![profile,same_id],many]{
            let bytes=serde_json::to_vec(&profiles).unwrap();std::fs::write(root.join("profiles.json"),&bytes).unwrap();
            let store=Store::load(root.clone());assert!(store.list().is_err());assert!(store.add("Nova".into(),None,"subscription".into()).is_err());
            assert_eq!(std::fs::read(root.join("profiles.json")).unwrap(),bytes);
        }std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn profiles_disk_changes_block_operations_before_login_directory_creation(){
        let root=fixture();let path=root.join("profiles.json");
        for original in [None,Some(b"[]".to_vec())]{
            if let Some(bytes)=original{std::fs::write(&path,bytes).unwrap();}else{let _=std::fs::remove_file(&path);}
            let store=Store::load(root.clone());assert!(store.list().is_ok());
            let edited=serde_json::to_vec(&vec![sample(&root)]).unwrap();std::fs::write(&path,&edited).unwrap();
            assert!(store.list().is_err());assert!(store.add("Nova".into(),None,"subscription".into()).is_err());
            assert_eq!(std::fs::read(&path).unwrap(),edited);assert!(!root.join("accounts").exists());
            let reloaded=Store::load(root.clone());assert_eq!(reloaded.list().unwrap().len(),1);
            std::fs::remove_file(&path).unwrap();assert!(reloaded.list().is_err());assert!(reloaded.add("Nova".into(),None,"subscription".into()).is_err());
        }std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn profiles_new_directory_rejects_outside_accounts_parent(){
        let base=fixture();let root=base.join("app");let outside=base.join("outside");std::fs::create_dir_all(&root).unwrap();std::fs::create_dir(&outside).unwrap();
        #[cfg(windows)] {
            use std::os::windows::process::CommandExt;
            let status=Command::new("powershell.exe").args(["-NoProfile","-NonInteractive","-Command","$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path $env:CAPY_TEST_LINK -Target $env:CAPY_TEST_TARGET | Out-Null"])
                .env("CAPY_TEST_LINK",root.join("accounts")).env("CAPY_TEST_TARGET",&outside).creation_flags(0x08000000).status().unwrap();assert!(status.success());
        }
        #[cfg(unix)] std::os::unix::fs::symlink(&outside,root.join("accounts")).unwrap();
        assert!(new_config_dir(&root,"fixture").is_err());assert!(!outside.join("fixture").exists());
        #[cfg(windows)] std::fs::remove_dir(root.join("accounts")).unwrap();
        #[cfg(unix)] std::fs::remove_file(root.join("accounts")).unwrap();
        std::fs::remove_dir_all(base).unwrap();
    }
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
