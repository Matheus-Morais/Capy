use crate::{chat_api::Message, discovery, settings};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::PathBuf, sync::Mutex};

#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all="camelCase")]
pub struct Target {
    pub kind: String,
    pub profile_id: String,
    pub provider: String,
    pub account: String,
    pub billing: String,
    pub credential_revision: Option<String>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub target: Target,
    pub model: String,
    pub messages: Vec<Message>,
    pub revision: u64,
    pub state: String,
    pub active_nonce: Option<String>,
    pub used_nonces: Vec<String>,
    pub last_error: Option<String>,
    pub cli_started: bool,
    #[serde(default)]
    pub cli_attempted: bool,
    #[serde(default)]
    pub process_policy: Option<String>,
    #[serde(default)]
    pub recovery_review: Option<crate::chat_recovery::Review>,
    #[serde(default)]
    pub interruptions: Vec<crate::chat_recovery::Interruption>,
    pub transferred_to: Option<String>,
}

pub struct Store {
    pub(crate) root: PathBuf,
    pub(crate) ids: Mutex<Vec<String>>,
    load_error: bool,
}

pub(crate) fn valid_target(target: &Target) -> bool {
    settings::valid_text(&target.profile_id,128) && settings::valid_text(&target.account,320)
        && match target.kind.as_str() {
            "claudeCli" => target.provider=="Claude" && ["subscription","api"].contains(&target.billing.as_str()) && target.credential_revision.is_none(),
            "api" => discovery::uuid(&target.profile_id) && crate::api_accounts::providers().contains(&target.provider.as_str())
                && target.billing=="api" && target.credential_revision.as_deref().is_some_and(discovery::uuid),
            _ => false,
        }
}

pub(crate) fn valid(record: &Conversation) -> bool {
    let mut nonces=HashSet::new();
    discovery::uuid(&record.id) && settings::valid_text(&record.title,80) && valid_target(&record.target)
        && crate::chat_api::valid_model(&record.model)
        && ["idle","working","completed","partial","failed","unknown","transferred"].contains(&record.state.as_str())
        && record.messages.len()<=200 && record.messages.iter().all(|m| ["user","assistant"].contains(&m.role.as_str()) && !m.text.trim().is_empty())
        && record.messages.iter().map(|m|m.text.len()).sum::<usize>()<=262_144
        && record.used_nonces.len()<=200 && record.used_nonces.iter().all(|n|discovery::uuid(n)&&nonces.insert(n))
        && match record.active_nonce.as_ref() {
            Some(n) => record.state=="working" && record.used_nonces.contains(n),
            None => record.state!="working",
        }
        && record.transferred_to.as_deref().is_none_or(discovery::uuid)
        && record.last_error.as_ref().is_none_or(|s|s.len()<=4096)
        && record.process_policy.as_deref().is_none_or(|s|s=="windows-job-v1")
        && record.recovery_review.as_ref().is_none_or(|review|crate::chat_recovery::valid_review(record,review))
        && record.interruptions.len()<=200 && record.interruptions.iter().all(|entry|
            record.messages.get(entry.message_index).is_some_and(|message|message.role=="user")
            &&record.used_nonces.contains(&entry.send_nonce)&&record.used_nonces.contains(&entry.approval_nonce))
}

impl Store {
    pub fn workspace(&self,id:&str)->Result<PathBuf,String>{
        self.get(id)?;
        let path=self.root.join("workspaces").join(id);
        std::fs::create_dir_all(&path).map_err(|_|"Não foi possível preparar a pasta própria do chat.")?;
        path.canonicalize().map_err(|_|"Pasta própria do chat indisponível.".into())
    }
    pub fn load(root:PathBuf)->Self {
        if crate::chat_transfer::recover(&root).is_err(){return Self{root,ids:Mutex::new(Vec::new()),load_error:true};}
        let path=root.join("index.json");
        let (ids,load_error)=if !path.exists(){(Vec::new(),false)}else{
            match settings::read_json::<Vec<String>>(&path) {
                Ok(ids) => {
                    let mut unique=HashSet::new();
                    if ids.len()<=500 && ids.iter().all(|id|discovery::uuid(id)&&unique.insert(id.clone())) {(ids,false)}else{(Vec::new(),true)}
                }
                Err(_)=>(Vec::new(),true),
            }
        };
        Self{root,ids:Mutex::new(ids),load_error}
    }
    pub(crate) fn read(&self,id:&str)->Result<Conversation,String>{
        if !discovery::uuid(id){return Err("Conversa inválida.".into());}
        let record:Conversation=settings::read_json_limit(&self.root.join(format!("{id}.json")),2_097_152)?;
        if record.id!=id || !valid(&record){return Err("Histórico incompatível; o arquivo foi preservado.".into());}
        Ok(record)
    }
    pub(crate) fn write(&self,record:&Conversation)->Result<(),String>{
        if !valid(record){return Err("Histórico excedeu os limites ou é incompatível.".into());}
        settings::write_json_limit(&self.root.join(format!("{}.json",record.id)),record,2_097_152)
    }
    pub(crate) fn available(&self)->Result<(),String>{
        if self.load_error {Err("O índice de conversas não pôde ser lido; o arquivo foi preservado.".into())}else{Ok(())}
    }
    pub(crate) fn consistent(&self)->Result<(),String>{
        self.available()?;
        if self.root.join("transfer-pending.json").exists(){return Err("Uma transferência precisa de recuperação. Reinicie a Capy; nenhum envio será repetido.".into());}
        Ok(())
    }
    pub fn list(&self)->Result<Vec<Conversation>,String>{
        self.available()?;
        let ids=self.ids.lock().map_err(|_|"Conversas indisponíveis.")?;
        self.consistent()?;
        ids.iter().map(|id|self.read(id)).collect()
    }
    pub fn get(&self,id:&str)->Result<Conversation,String>{
        self.available()?;
        let ids=self.ids.lock().map_err(|_|"Conversas indisponíveis.")?;
        self.consistent()?;
        if !ids.iter().any(|known|known==id){return Err("Conversa não encontrada.".into());}
        self.read(id)
    }
    pub fn create(&self,title:String,target:Target,model:String)->Result<Conversation,String>{
        self.available()?;
        let mut ids=self.ids.lock().map_err(|_|"Conversas indisponíveis.")?;
        self.consistent()?;
        if ids.len()>=500{return Err("Limite de 500 conversas. Exporte ou remova uma conversa antes de criar outra.".into());}
        let process_policy=(target.kind=="claudeCli").then(||"windows-job-v1".into());
        let record=Conversation{id:uuid::Uuid::new_v4().to_string(),title,target,model,messages:vec![],revision:0,state:"idle".into(),process_policy,
            active_nonce:None,used_nonces:vec![],last_error:None,cli_started:false,cli_attempted:false,recovery_review:None,interruptions:vec![],transferred_to:None};
        self.write(&record)?;
        let mut updated=ids.clone();updated.push(record.id.clone());
        if let Err(error)=settings::write_json(&self.root.join("index.json"),&updated){
            let _=std::fs::remove_file(self.root.join(format!("{}.json",record.id)));return Err(error);
        }
        *ids=updated;Ok(record)
    }
    pub fn begin(&self,id:&str,revision:u64,nonce:String,model:String,text:String)->Result<Conversation,String>{
        self.available()?;
        let ids=self.ids.lock().map_err(|_|"Conversas indisponíveis.")?;
        self.consistent()?;
        if !ids.iter().any(|known|known==id){return Err("Conversa não encontrada.".into());}
        let mut record=self.read(id)?;
        if record.revision!=revision || record.state=="working" || record.transferred_to.is_some()
            || !discovery::uuid(&nonce) || record.used_nonces.contains(&nonce) {
            return Err("A conversa mudou, está ocupada ou este envio já foi usado. Atualize antes de enviar.".into());
        }
        if record.state=="unknown" {return Err("O envio anterior foi interrompido. Revise o histórico antes de continuar; ele não será reenviado.".into());}
        if record.target.kind=="claudeCli"&&record.cli_attempted&&!record.cli_started {
            return Err("O histórico CLI não foi confirmado. Prepare uma transferência revisada para uma nova conversa.".into());
        }
        if text.trim().is_empty() || text.len()>196_608 || record.messages.iter().map(|m|m.text.len()).sum::<usize>()+text.len()>196_608
            || record.messages.len()>198 || record.used_nonces.len()>=200 {
            return Err("Histórico cheio. Prepare uma transferência revisada antes de continuar.".into());
        }
        record.model=model;record.messages.push(Message{role:"user".into(),text});record.used_nonces.push(nonce.clone());
        record.active_nonce=Some(nonce);record.state="working".into();record.last_error=None;
        record.revision=record.revision.checked_add(1).ok_or("Versão da conversa esgotada.")?;
        self.write(&record)?;Ok(record)
    }
    pub fn mark_cli_attempt(&self,id:&str,nonce:&str)->Result<Conversation,String>{
        self.available()?;
        let ids=self.ids.lock().map_err(|_|"Conversas indisponíveis.")?;
        self.consistent()?;
        if !ids.iter().any(|known|known==id){return Err("Conversa não encontrada.".into());}
        let mut record=self.read(id)?;
        if record.target.kind!="claudeCli" || record.state!="working" || record.active_nonce.as_deref()!=Some(nonce){
            return Err("A tentativa CLI pertence a outro envio.".into());
        }
        record.cli_attempted=true;
        record.revision=record.revision.checked_add(1).ok_or("Versão da conversa esgotada.")?;
        self.write(&record)?;Ok(record)
    }
    pub fn finish(&self,id:&str,nonce:&str,result:Result<crate::chat_api::Reply,String>,cli_started:bool)->Result<Conversation,String>{
        self.available()?;
        let ids=self.ids.lock().map_err(|_|"Conversas indisponíveis.")?;
        self.consistent()?;
        if !ids.iter().any(|known|known==id){return Err("Conversa não encontrada.".into());}
        let mut record=self.read(id)?;
        if record.active_nonce.as_deref()!=Some(nonce) || record.state!="working" {return Err("Resultado pertence a outro envio.".into());}
        match result {
            Ok(reply)=>{
                if reply.text.trim().is_empty() || reply.text.len()>65_536{return Err("Resposta inválida ou acima do espaço reservado; envio permanece pendente para recuperação.".into());}
                record.messages.push(Message{role:"assistant".into(),text:reply.text});
                record.state=if reply.completed {"completed"} else {"partial"}.into();record.last_error=reply.note;
            }
            Err(error)=>{record.state="failed".into();record.last_error=Some(error.chars().take(1024).collect());}
        }
        record.cli_started|=cli_started;record.active_nonce=None;
        record.revision=record.revision.checked_add(1).ok_or("Versão da conversa esgotada.")?;
        self.write(&record)?;Ok(record)
    }
    pub fn recover_interrupted(&self)->Result<(),String>{
        self.available()?;
        let ids=self.ids.lock().map_err(|_|"Conversas indisponíveis.")?;
        self.consistent()?;
        for id in ids.iter(){
            let mut record=self.read(id)?;
            if record.state=="working" {
                record.state="unknown".into();record.active_nonce=None;
                record.last_error=Some("A Capy fechou durante o envio. O consumo pode ter ocorrido; a mensagem não foi reenviada.".into());
                record.revision=record.revision.checked_add(1).ok_or("Versão da conversa esgotada.")?;self.write(&record)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn target()->Target{Target{kind:"api".into(),profile_id:uuid::Uuid::new_v4().to_string(),provider:"OpenAI".into(),account:"Local key label".into(),billing:"api".into(),credential_revision:Some(uuid::Uuid::new_v4().to_string())}}
    #[test]
    fn chat_cli_attempt_survives_interruption_without_assuming_result_or_resending(){
        let root=std::env::temp_dir().join(format!("capy-chat-attempt-{}",uuid::Uuid::new_v4()));
        let store=Store::load(root.clone());
        let target=Target{kind:"claudeCli".into(),profile_id:"claude-default".into(),provider:"Claude".into(),account:"test@example.invalid".into(),billing:"subscription".into(),credential_revision:None};
        let chat=store.create("Interrupted".into(),target,"sonnet".into()).unwrap();
        let nonce=uuid::Uuid::new_v4().to_string();
        let pending=store.begin(&chat.id,0,nonce.clone(),"sonnet".into(),"Only send once".into()).unwrap();
        assert!(store.mark_cli_attempt(&chat.id,&uuid::Uuid::new_v4().to_string()).is_err());
        let attempted=store.mark_cli_attempt(&chat.id,&nonce).unwrap();
        assert!(attempted.cli_attempted);assert!(!attempted.cli_started);assert!(attempted.revision>pending.revision);
        let restarted=Store::load(root.clone());restarted.recover_interrupted().unwrap();
        let recovered=restarted.get(&chat.id).unwrap();
        assert_eq!(recovered.state,"unknown");assert!(recovered.cli_attempted);assert!(!recovered.cli_started);
        assert_eq!(recovered.messages.len(),1);assert_eq!(recovered.messages[0].text,"Only send once");
        assert!(recovered.used_nonces.contains(&nonce));assert!(recovered.active_nonce.is_none());
        assert!(restarted.begin(&chat.id,recovered.revision,uuid::Uuid::new_v4().to_string(),"sonnet".into(),"New turn".into()).is_err());
        assert!(restarted.mark_cli_attempt(&chat.id,&nonce).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn chat_send_is_single_flight_revision_and_result_success(){
        let root=std::env::temp_dir().join(format!("capy-chat-{}",uuid::Uuid::new_v4()));
        let store=Store::load(root.clone());let chat=store.create("Test".into(),target(),"chosen-model".into()).unwrap();
        let nonce=uuid::Uuid::new_v4().to_string();let started=store.begin(&chat.id,0,nonce.clone(),"chosen-model".into(),"Hello".into()).unwrap();
        assert!(store.begin(&chat.id,0,uuid::Uuid::new_v4().to_string(),"chosen-model".into(),"Duplicate".into()).is_err());
        assert!(store.begin(&chat.id,started.revision,uuid::Uuid::new_v4().to_string(),"chosen-model".into(),"Concurrent".into()).is_err());
        assert!(store.finish(&chat.id,&uuid::Uuid::new_v4().to_string(),Err("error".into()),false).is_err());
        let ended=store.finish(&chat.id,&nonce,Ok(crate::chat_api::Reply{text:"Reply".into(),completed:true,note:None}),false).unwrap();
        assert_eq!(ended.state,"completed");assert_eq!(ended.messages.len(),2);
        assert!(store.finish(&chat.id,&nonce,Err("Late".into()),false).is_err());
        let restarted=Store::load(root.clone());assert!(restarted.begin(&chat.id,ended.revision,nonce,"chosen-model".into(),"Replay".into()).is_err());
        let nonce=uuid::Uuid::new_v4().to_string();let pending=restarted.begin(&chat.id,ended.revision,nonce,"changed-model".into(),"Next".into()).unwrap();
        assert_eq!(pending.model,"changed-model");
        let recovered=Store::load(root.clone());recovered.recover_interrupted().unwrap();
        let unknown=recovered.get(&chat.id).unwrap();assert_eq!(unknown.state,"unknown");assert_eq!(unknown.messages.len(),3);
        assert!(recovered.begin(&chat.id,unknown.revision,uuid::Uuid::new_v4().to_string(),"changed-model".into(),"Retry".into()).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
