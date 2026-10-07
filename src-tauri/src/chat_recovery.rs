use crate::{chat_history::{Conversation,Store,Target},discovery,profiles,settings};
use serde::{Deserialize,Serialize};
use std::{io::{BufRead,BufReader,Read},path::Path};

#[derive(Clone,Deserialize,Serialize)]
#[serde(rename_all="camelCase")]
pub struct Review {pub id:String,pub nonce:String,pub revision:u64,pub target:Target,pub resume_cli:bool}
#[derive(Clone,Deserialize,Serialize)]
#[serde(rename_all="camelCase")]
pub struct Interruption {pub message_index:usize,pub send_nonce:String,pub approval_nonce:String}

pub fn valid_review(chat:&Conversation,review:&Review)->bool{
    discovery::uuid(&review.nonce)&&review.id==chat.id&&review.revision==chat.revision&&review.target==chat.target
        &&chat.state=="unknown"&&chat.active_nonce.is_none()&&!chat.used_nonces.contains(&review.nonce)
        &&(!review.resume_cli||chat.target.kind=="claudeCli")
}
fn eligible(chat:&Conversation)->Result<(),String>{
    if chat.state!="unknown"||chat.active_nonce.is_some()||chat.transferred_to.is_some()||chat.used_nonces.len()>=200
        ||chat.messages.last().is_none_or(|message|message.role!="user") {
        return Err("A conversa mudou ou não possui um envio incerto para revisar.".into());
    }
    if chat.target.kind=="claudeCli"&&!chat.cli_attempted&&chat.process_policy.as_deref()!=Some("windows-job-v1") {
        return Err("Este envio usa um launcher antigo sem proteção comprovada. Confira e encerre a sessão na origem; a Capy preservou o histórico.".into());
    }
    Ok(())
}

fn no_live_session(config:&Path,id:&str)->Result<(),String>{
    let folder=config.join("sessions");
    if !folder.exists(){return Ok(());}
    let mut count=0;
    for entry in std::fs::read_dir(folder).map_err(|_|"Não foi possível conferir sessões CLI ativas.")? {
        let entry=entry.map_err(|_|"Não foi possível conferir sessões CLI ativas.")?;
        if entry.path().extension().is_none_or(|ext|ext!="json"){continue;}
        count+=1;if count>4096{return Err("A lista de sessões excedeu o limite de conferência.".into());}
        let value:serde_json::Value=match settings::read_json_limit(&entry.path(),65_536){
            Ok(value)=>value,
            Err(_) if !entry.path().exists()=>continue,
            Err(_)=>return Err("Metadados de sessão ilegíveis; confira a origem antes de recuperar.".into()),
        };
        if value["sessionId"].as_str()==Some(id){
            let pid=value["pid"].as_u64().and_then(|pid|u32::try_from(pid).ok()).ok_or("Identidade do processo indisponível.")?;
            if discovery::process_birth(pid).is_some(){return Err("Esta sessão ainda possui um processo ativo. Encerre o turno na origem antes de revisar.".into());}
        }
    }
    Ok(())
}

pub fn cli_resume_available(profile:&profiles::Profile,chat:&Conversation,cwd:&Path)->Result<bool,String>{
    eligible(chat)?;no_live_session(&profile.config_dir,&chat.id)?;
    let projects=profile.config_dir.join("projects");if !projects.exists(){return Ok(false);}
    let mut count=0;let mut found=false;
    for entry in std::fs::read_dir(projects).map_err(|_|"Históricos CLI indisponíveis.")? {
        let entry=entry.map_err(|_|"Históricos CLI indisponíveis.")?;
        count+=1;if count>2048{return Err("A lista de projetos excedeu o limite de conferência.".into());}
        if !entry.file_type().map_err(|_|"Projeto CLI indisponível.")?.is_dir(){continue;}
        let path=entry.path().join(format!("{}.jsonl",chat.id));if !path.exists(){continue;}
        if found{return Err("O mesmo UUID aparece em mais de um histórico CLI; confira a origem.".into());}
        let file=std::fs::File::open(path).map_err(|_|"Histórico CLI indisponível.")?;
        let mut matched=false;
        for line in BufReader::new(file.take(2_097_153)).lines(){
            let line=line.map_err(|_|"Histórico CLI ilegível.")?;
            let value:serde_json::Value=serde_json::from_str(&line).map_err(|_|"Histórico CLI incompleto; confira a origem.")?;
            if value["type"]=="user"&&value["isSidechain"]!=true&&value["sessionId"].as_str()==Some(&chat.id){
                let folder=value["cwd"].as_str().ok_or("Pasta do histórico CLI indisponível.")?;
                if Path::new(folder).canonicalize().ok().as_deref()!=Some(cwd){return Err("O histórico CLI pertence a outra pasta.".into());}
                matched=true;break;
            }
        }
        if !matched{return Err("O histórico CLI não confirmou este UUID e projeto.".into());}
        found=true;
    }
    Ok(found)
}

impl Store {
    pub fn prepare_recovery(&self,id:&str,revision:u64,resume_cli:bool)->Result<Conversation,String>{
        self.available()?;let ids=self.ids.lock().map_err(|_|"Conversas indisponíveis.")?;self.consistent()?;
        if !ids.iter().any(|known|known==id){return Err("Conversa não encontrada.".into());}
        let mut chat=self.read(id)?;eligible(&chat)?;
        if chat.revision!=revision{return Err("A conversa mudou. Prepare uma revisão atualizada.".into());}
        chat.revision=chat.revision.checked_add(1).ok_or("Versão esgotada.")?;
        chat.recovery_review=Some(Review{id:chat.id.clone(),nonce:uuid::Uuid::new_v4().to_string(),revision:chat.revision,target:chat.target.clone(),resume_cli});
        self.write(&chat)?;Ok(chat)
    }
    pub fn approve_recovery(&self,id:&str,revision:u64,nonce:&str,reviewed:bool,current:&Target,resume_cli:bool)->Result<Conversation,String>{
        self.available()?;let ids=self.ids.lock().map_err(|_|"Conversas indisponíveis.")?;self.consistent()?;
        if !ids.iter().any(|known|known==id){return Err("Conversa não encontrada.".into());}
        let mut chat=self.read(id)?;eligible(&chat)?;
        let review=chat.recovery_review.as_ref().ok_or("Prepare a revisão deste envio antes de aprovar.")?;
        if !reviewed||chat.revision!=revision||review.nonce!=nonce||&chat.target!=current||review.resume_cli!=resume_cli {
            return Err("A revisão ou a identidade mudou. Revise este envio e confirme novamente.".into());
        }
        let send_nonce=chat.used_nonces.last().ok_or("Identidade do envio incerto indisponível.")?.clone();
        chat.interruptions.push(Interruption{message_index:chat.messages.len()-1,send_nonce,approval_nonce:nonce.into()});
        chat.used_nonces.push(nonce.into());
        if chat.target.kind=="claudeCli"{chat.cli_started=resume_cli;}
        // Resolving a lost result is not a provider success and sends nothing.
        chat.state="failed".into();chat.recovery_review=None;
        chat.last_error=Some(format!("Envio interrompido revisado. O consumo pode ter ocorrido e não há resposta confirmada. Nenhuma mensagem foi reenviada. {}",
            if chat.target.kind=="claudeCli"&&!resume_cli{"O histórico CLI não está disponível: prepare uma transferência revisada para uma nova conversa."}else{"Você pode escrever uma nova mensagem ou preparar uma transferência revisada."}));
        chat.revision=chat.revision.checked_add(1).ok_or("Versão esgotada.")?;self.write(&chat)?;Ok(chat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn unknown(store:&Store,cli:bool)->Conversation{
        let target=Target{kind:if cli{"claudeCli"}else{"api"}.into(),profile_id:uuid::Uuid::new_v4().to_string(),provider:if cli{"Claude"}else{"OpenAI"}.into(),account:"fixture".into(),billing:if cli{"subscription"}else{"api"}.into(),credential_revision:(!cli).then(||uuid::Uuid::new_v4().to_string())};
        let chat=store.create("Recovery".into(),target,"model".into()).unwrap();
        store.begin(&chat.id,0,uuid::Uuid::new_v4().to_string(),"model".into(),"Never repeat this".into()).unwrap();
        if cli {let mut pending=store.get(&chat.id).unwrap();pending.cli_attempted=true;store.write(&pending).unwrap();}
        store.recover_interrupted().unwrap();store.get(&chat.id).unwrap()
    }
    #[test]
    fn chat_recovery_requires_fresh_approval_identity_and_keeps_uncertainty_without_send(){
        let root=std::env::temp_dir().join(format!("capy-recovery-{}",uuid::Uuid::new_v4()));let store=Store::load(root.clone());
        let chat=unknown(&store,false);let original_nonce=chat.used_nonces[0].clone();
        let old=store.prepare_recovery(&chat.id,chat.revision,false).unwrap();let old_nonce=old.recovery_review.unwrap().nonce;
        let current=store.prepare_recovery(&chat.id,old.revision,false).unwrap();let nonce=current.recovery_review.clone().unwrap().nonce;
        assert!(store.approve_recovery(&chat.id,current.revision,&old_nonce,true,&chat.target,false).is_err());
        assert!(store.approve_recovery(&chat.id,current.revision,&nonce,false,&chat.target,false).is_err());
        let mut wrong=chat.target.clone();wrong.account="different".into();
        assert!(store.approve_recovery(&chat.id,current.revision,&nonce,true,&wrong,false).is_err());
        assert!(store.approve_recovery(&chat.id,old.revision,&nonce,true,&chat.target,false).is_err());
        let restarted=Store::load(root.clone());let approved=restarted.approve_recovery(&chat.id,current.revision,&nonce,true,&chat.target,false).unwrap();
        assert_eq!(approved.state,"failed");assert_eq!(approved.messages.len(),1);assert_eq!(approved.messages[0].text,"Never repeat this");
        assert!(approved.used_nonces.contains(&original_nonce));assert!(approved.used_nonces.contains(&nonce));assert!(approved.active_nonce.is_none());
        assert!(approved.last_error.unwrap().contains("consumo pode ter ocorrido"));
        assert_eq!(approved.interruptions.len(),1);assert_eq!(approved.interruptions[0].message_index,0);
        assert_eq!(approved.interruptions[0].send_nonce,original_nonce);assert_eq!(approved.interruptions[0].approval_nonce,nonce);
        assert!(restarted.approve_recovery(&chat.id,current.revision,&nonce,true,&chat.target,false).is_err());
        let next=restarted.begin(&chat.id,approved.revision,uuid::Uuid::new_v4().to_string(),"model".into(),"A different message".into()).unwrap();
        assert_eq!(next.messages.len(),2);assert_eq!(next.messages[1].text,"A different message");
        assert_eq!(next.interruptions.len(),1);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn chat_recovery_cli_without_history_never_recreates_same_session(){
        let root=std::env::temp_dir().join(format!("capy-recovery-cli-{}",uuid::Uuid::new_v4()));let store=Store::load(root.clone());
        let chat=unknown(&store,true);let review=store.prepare_recovery(&chat.id,chat.revision,false).unwrap();
        let nonce=review.recovery_review.as_ref().unwrap().nonce.clone();
        let approved=store.approve_recovery(&chat.id,review.revision,&nonce,true,&chat.target,false).unwrap();
        assert!(!approved.cli_started);assert_eq!(approved.messages.len(),1);
        assert!(store.begin(&chat.id,approved.revision,uuid::Uuid::new_v4().to_string(),"model".into(),"Wrong new turn".into()).is_err());
        assert!(store.prepare_transfer(&chat.id,chat.target.clone(),"model".into()).is_ok());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn chat_recovery_missing_history_blocks_resume_flag_from_an_earlier_turn(){
        let root=std::env::temp_dir().join(format!("capy-recovery-old-turn-{}",uuid::Uuid::new_v4()));let store=Store::load(root.clone());
        let mut chat=unknown(&store,true);chat.cli_started=true;store.write(&chat).unwrap();
        let reviewed=store.prepare_recovery(&chat.id,chat.revision,false).unwrap();let nonce=reviewed.recovery_review.unwrap().nonce;
        let approved=store.approve_recovery(&chat.id,reviewed.revision,&nonce,true,&chat.target,false).unwrap();
        assert!(!approved.cli_started);
        assert!(store.begin(&chat.id,approved.revision,uuid::Uuid::new_v4().to_string(),"model".into(),"Do not recreate UUID".into()).is_err());
        assert_eq!(approved.messages.len(),1);assert!(store.prepare_transfer(&chat.id,chat.target,"model".into()).is_ok());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn chat_recovery_cli_checks_exact_history_folder_and_live_process_before_resume(){
        let root=std::env::temp_dir().join(format!("capy-recovery-identity-{}",uuid::Uuid::new_v4()));let store=Store::load(root.join("chat"));
        let chat=unknown(&store,true);let cwd=store.workspace(&chat.id).unwrap();
        let profile=profiles::Profile{id:chat.target.profile_id.clone(),label:"Fixture".into(),provider:"Claude".into(),config_dir:root.join("config"),billing:"subscription".into()};
        assert!(!cli_resume_available(&profile,&chat,&cwd).unwrap());
        let project=profile.config_dir.join("projects/own");std::fs::create_dir_all(&project).unwrap();
        let path=project.join(format!("{}.jsonl",chat.id));
        let row=serde_json::json!({"type":"user","sessionId":chat.id,"cwd":cwd,"message":{"content":"Never repeat this"}});
        std::fs::write(&path,row.to_string()).unwrap();assert!(cli_resume_available(&profile,&chat,&cwd).unwrap());
        // Only identity metadata is read; a large later transcript remains intact.
        let large=format!("{}\n{}",row,"x".repeat(2_097_152));std::fs::write(&path,&large).unwrap();
        assert!(cli_resume_available(&profile,&chat,&cwd).unwrap());assert_eq!(std::fs::metadata(&path).unwrap().len(),large.len() as u64);
        let mut wrong=row.clone();wrong["cwd"]=root.to_string_lossy().as_ref().into();std::fs::write(&path,wrong.to_string()).unwrap();
        assert!(cli_resume_available(&profile,&chat,&cwd).is_err());
        wrong=row.clone();wrong["sessionId"]=uuid::Uuid::new_v4().to_string().into();std::fs::write(&path,wrong.to_string()).unwrap();
        assert!(cli_resume_available(&profile,&chat,&cwd).is_err());
        std::fs::write(&path,row.to_string()).unwrap();
        let sessions=profile.config_dir.join("sessions");std::fs::create_dir_all(&sessions).unwrap();
        settings::write_json(&sessions.join("own.json"),&serde_json::json!({"sessionId":chat.id,"pid":std::process::id()})).unwrap();
        assert!(cli_resume_available(&profile,&chat,&cwd).is_err());
        std::fs::remove_file(sessions.join("own.json")).unwrap();assert!(cli_resume_available(&profile,&chat,&cwd).unwrap());
        let prepared=store.prepare_recovery(&chat.id,chat.revision,true).unwrap();let nonce=prepared.recovery_review.unwrap().nonce;
        assert!(store.approve_recovery(&chat.id,prepared.revision,&nonce,true,&chat.target,false).is_err());
        let approved=store.approve_recovery(&chat.id,prepared.revision,&nonce,true,&chat.target,true).unwrap();
        assert!(approved.cli_started);assert_eq!(approved.state,"failed");assert_eq!(approved.messages.len(),1);
        let next=store.begin(&chat.id,approved.revision,uuid::Uuid::new_v4().to_string(),"model".into(),"New instruction".into()).unwrap();
        assert!(next.cli_started);assert_eq!(next.id,chat.id);
        std::fs::remove_dir_all(root).unwrap();
    }
}
