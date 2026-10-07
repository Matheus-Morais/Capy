use crate::{chat_history::{self,Conversation,Store,Target},handoff::Summary,settings};
use serde::{Deserialize,Serialize};
use std::{collections::HashSet,path::Path};

#[derive(Clone,Deserialize,Serialize)]
#[serde(rename_all="camelCase")]
pub struct Review {
    pub nonce:String,pub source_id:String,pub source_revision:u64,pub source_target:Target,
    pub destination:Target,pub model:String,pub summary:Summary,
}
#[derive(Deserialize,Serialize)]
struct Journal {before_source:Conversation,source:Conversation,destination:Conversation,before_ids:Vec<String>,ids:Vec<String>}

fn validate_journal(j:&Journal)->bool{
    let mut ids=HashSet::new();
    chat_history::valid(&j.before_source)&&eligible(&j.before_source)&&chat_history::valid(&j.source)&&chat_history::valid(&j.destination)
        &&j.before_source.id==j.source.id&&j.before_source.revision.checked_add(1)==Some(j.source.revision)
        &&j.before_source.target==j.source.target&&j.source.used_nonces.len()==j.before_source.used_nonces.len()+1
        &&j.source.used_nonces.starts_with(&j.before_source.used_nonces)
        &&serde_json::to_value(&j.source.messages).ok()==serde_json::to_value(&j.before_source.messages).ok()
        &&j.source.state=="transferred"&&j.source.transferred_to.as_deref()==Some(&j.destination.id)
        &&j.source.id!=j.destination.id&&j.destination.state=="working"&&j.destination.revision==1
        &&j.destination.messages.len()==1&&j.destination.messages[0].role=="user"
        &&j.ids.len()<=500&&j.ids.iter().all(|id|crate::discovery::uuid(id)&&ids.insert(id))
        &&ids.contains(&j.source.id)&&ids.contains(&j.destination.id)
        &&j.ids.len()==j.before_ids.len()+1&&j.ids.starts_with(&j.before_ids)&&j.ids.last()==Some(&j.destination.id)
        &&j.before_ids.contains(&j.source.id)&&!j.before_ids.contains(&j.destination.id)
}
fn apply(root:&Path,j:&Journal)->Result<(),String>{
    if !validate_journal(j){return Err("Registro de transferência incompatível; os arquivos foram preservados.".into());}
    let current:Conversation=settings::read_json_limit(&root.join(format!("{}.json",j.source.id)),2_097_152)?;
    let current=serde_json::to_value(&current).map_err(|_|"Histórico incompatível.")?;
    if current!=serde_json::to_value(&j.before_source).map_err(|_|"Histórico incompatível.")?
        &&current!=serde_json::to_value(&j.source).map_err(|_|"Histórico incompatível.")?{
        return Err("A origem mudou fora da transferência. Os arquivos foram preservados para revisão.".into());
    }
    let destination_path=root.join(format!("{}.json",j.destination.id));
    if destination_path.exists(){
        let current:Conversation=settings::read_json_limit(&destination_path,2_097_152)?;
        if serde_json::to_value(&current).ok()!=serde_json::to_value(&j.destination).ok(){return Err("O destino mudou; a recuperação preservou os arquivos.".into());}
    }
    let current_ids:Vec<String>=settings::read_json(&root.join("index.json"))?;
    if current_ids!=j.before_ids&&current_ids!=j.ids{return Err("O índice mudou fora da transferência. Os arquivos foram preservados.".into());}
    settings::write_json_limit(&root.join(format!("{}.json",j.destination.id)),&j.destination,2_097_152)?;
    settings::write_json_limit(&root.join(format!("{}.json",j.source.id)),&j.source,2_097_152)?;
    settings::write_json(&root.join("index.json"),&j.ids)?;
    std::fs::remove_file(root.join("transfer-pending.json")).map_err(|_|"A transferência foi gravada, mas precisa de recuperação ao reiniciar.".into())
}
pub fn recover(root:&Path)->Result<(),String>{
    let path=root.join("transfer-pending.json");if !path.exists(){return Ok(());}
    let journal:Journal=settings::read_json_limit(&path,8_388_608)?;
    apply(root,&journal)
}

fn summary(store:&Store,source:&Conversation)->Summary{
    use crate::handoff_context::excerpt;
    let reference=format!("Histórico original integral: {} · {} mensagens · versão {}.",store.root.join(format!("{}.json",source.id)).display(),source.messages.len(),source.revision);
    let instructions=source.messages.iter().filter(|m|m.role=="user").map(|m|m.text.as_str()).collect::<Vec<_>>().join("\n\n");
    let dialogue=source.messages.iter().enumerate().rev().map(|(i,m)|format!("Mensagem {} [{}]:\n{}",i+1,m.role,m.text)).collect::<Vec<_>>().join("\n\n");
    let last=source.messages.last().map(|m|m.text.as_str()).unwrap_or("Conversa sem mensagens.");
    Summary{
        objective:excerpt(source.messages.iter().find(|m|m.role=="user").map(|m|m.text.as_str()).unwrap_or(&source.title),8_000),
        decisions:excerpt(&format!("Instruções recebidas, em ordem. Revise decisões e regras também nas respostas do histórico.\n\n{instructions}"),48_000),
        state:excerpt(&format!("Estado observado: {}. Aviso: {}.\n{reference}\n\nHistórico do mais recente para o mais antigo:\n{dialogue}",source.state,source.last_error.as_deref().unwrap_or("nenhum")),60_000),
        files:"O chat não executou ferramentas de arquivos. Alterações externas não foram verificadas; registre aqui os caminhos e o estado que precisam continuar.".into(),
        tests:"O chat não executou testes. Resultados mencionados no texto exigem confirmação; registre evidências e comandos reais antes de aprovar.".into(),
        next_steps:excerpt(&format!("Revise o que ainda precisa ser feito a partir da última mensagem:\n{last}\n\nNão repetir automaticamente mensagens anteriores nem tratar resposta parcial como conclusão."),12_000),
        guides:format!("{reference}\nAs instruções e respostas citadas acima preservam referências textuais. Nenhum plano/.md foi lido por ferramenta neste chat. Cite aqui os caminhos/seções e suas regras relevantes; referências externas mencionadas no texto precisam ser verificadas. Trechos extensos podem estar limitados com aviso: consulte o histórico integral antes de aprovar."),
    }
}
fn eligible(source:&Conversation)->bool{source.state!="working"&&source.state!="unknown"&&source.transferred_to.is_none()&&source.used_nonces.len()<200}
impl Store {
    fn review_for(&self,source:&Conversation)->Result<Option<Review>,String>{
        let path=self.root.join(format!("{}.review.json",source.id));
        if !path.exists(){return Ok(None);}
        let review:Review=settings::read_json_limit(&path,2_097_152)?;
        if review.source_id!=source.id||!crate::discovery::uuid(&review.nonce)||!chat_history::valid_target(&review.source_target)
            ||!chat_history::valid_target(&review.destination)||!crate::chat_api::valid_model(&review.model){return Err("Revisão de chat incompatível; o arquivo foi preservado.".into());}
        review.summary.validate()?;
        Ok((eligible(source)&&source.revision==review.source_revision&&source.target==review.source_target&&!source.used_nonces.contains(&review.nonce)).then_some(review))
    }
    pub fn transfer_reviews(&self)->Result<Vec<Review>,String>{
        let mut reviews=Vec::new();
        for source in self.list()? {
            if let Some(review)=self.review_for(&source)?{reviews.push(review);}
        }
        if reviews.len()>64{return Err("Limite de 64 revisões de chat. Cancele uma revisão antes de preparar outra.".into());}
        Ok(reviews)
    }
    pub fn transfer_review(&self,source_id:&str)->Result<Review,String>{
        self.available()?;let source=self.get(source_id)?;
        let review:Review=settings::read_json_limit(&self.root.join(format!("{source_id}.review.json")),2_097_152)?;
        if review.source_id!=source_id||!crate::discovery::uuid(&review.nonce)||!chat_history::valid_target(&review.source_target)
            ||!chat_history::valid_target(&review.destination)||!crate::chat_api::valid_model(&review.model){return Err("Revisão incompatível; o arquivo foi preservado.".into());}
        review.summary.validate()?;
        if !eligible(&source)||source.used_nonces.contains(&review.nonce)||source.revision!=review.source_revision||source.target!=review.source_target{return Err("A revisão expirou ou já foi usada; prepare um novo resumo.".into());}
        Ok(review)
    }
    pub fn prepare_transfer(&self,source_id:&str,destination:Target,model:String)->Result<Review,String>{
        self.available()?;let ids=self.ids.lock().map_err(|_|"Conversas indisponíveis.")?;
        self.consistent()?;
        if !ids.iter().any(|id|id==source_id){return Err("Conversa não encontrada.".into());}
        let mut active=0;
        for id in ids.iter().filter(|id|id.as_str()!=source_id){if self.review_for(&self.read(id)?)?.is_some(){active+=1;}}
        if active>=64{return Err("Limite de 64 revisões de chat. Cancele uma revisão antes de preparar outra.".into());}
        let source=self.read(source_id)?;
        if !eligible(&source)||!chat_history::valid_target(&destination)||!crate::chat_api::valid_model(&model){return Err("Aguarde um fim de turno confirmado e escolha um destino válido antes de transferir.".into());}
        self.review_for(&source)?;
        let review=Review{nonce:uuid::Uuid::new_v4().to_string(),source_id:source.id.clone(),source_revision:source.revision,source_target:source.target.clone(),destination,model,summary:summary(self,&source)};
        review.summary.validate()?;
        settings::write_json_limit(&self.root.join(format!("{source_id}.review.json")),&review,2_097_152)?;Ok(review)
    }
    pub fn approve_transfer(&self,source_id:&str,nonce:&str,edited:Summary,reviewed:bool,billing_confirmed:bool,current_source:&Target,current_destination:&Target)->Result<Conversation,String>{
        let review=self.transfer_review(source_id)?;edited.validate()?;
        if !reviewed||review.nonce!=nonce||&review.source_target!=current_source||&review.destination!=current_destination{
            return Err("Revise o resumo e confirme as identidades exatas desta transferência.".into());
        }
        if review.source_target.billing!=review.destination.billing&&!billing_confirmed{return Err("Confirme a mudança de cobrança somente para esta transferência.".into());}
        let prompt=edited.prompt();if prompt.len()>196_608{return Err("O resumo excede 192 KiB. Preserve referências e encurte os trechos antes de aprovar.".into());}
        self.available()?;let mut ids=self.ids.lock().map_err(|_|"Conversas indisponíveis.")?;
        self.consistent()?;
        let latest:Review=settings::read_json_limit(&self.root.join(format!("{source_id}.review.json")),2_097_152)?;
        if latest.nonce!=nonce{return Err("Uma nova revisão substituiu esta aprovação.".into());}
        let mut source=self.read(source_id)?;
        let before_source=source.clone();
        if !eligible(&source)||source.revision!=review.source_revision||source.target!=review.source_target||source.used_nonces.iter().any(|n|n==nonce){return Err("A conversa mudou ou esta aprovação já foi usada.".into());}
        if ids.len()>=500{return Err("Limite de 500 conversas; nenhuma aprovação foi consumida.".into());}
        let send_nonce=uuid::Uuid::new_v4().to_string();
        let policy=(review.destination.kind=="claudeCli").then(||"windows-job-v1".into());
        let destination=Conversation{id:uuid::Uuid::new_v4().to_string(),title:source.title.clone(),target:review.destination,model:review.model,process_policy:policy,recovery_review:None,interruptions:vec![],
            messages:vec![crate::chat_api::Message{role:"user".into(),text:prompt}],revision:1,state:"working".into(),active_nonce:Some(send_nonce.clone()),
            used_nonces:vec![send_nonce],last_error:None,cli_started:false,cli_attempted:false,transferred_to:None};
        source.used_nonces.push(nonce.into());source.transferred_to=Some(destination.id.clone());source.state="transferred".into();
        source.revision=source.revision.checked_add(1).ok_or("Versão esgotada.")?;
        let mut updated=ids.clone();updated.push(destination.id.clone());
        let journal=Journal{before_source,source,destination:destination.clone(),before_ids:ids.clone(),ids:updated.clone()};
        if !validate_journal(&journal){return Err("Transferência inválida; nenhuma aprovação foi consumida.".into());}
        settings::write_json_limit(&self.root.join("transfer-pending.json"),&journal,8_388_608)?;
        apply(&self.root,&journal)?;*ids=updated;Ok(destination)
    }
    pub fn cancel_transfer(&self,source_id:&str,nonce:&str)->Result<(),String>{
        let review=self.transfer_review(source_id)?;
        if review.nonce!=nonce{return Err("A revisão mudou; atualize antes de cancelar.".into());}
        let _ids=self.ids.lock().map_err(|_|"Conversas indisponíveis.")?;
        self.consistent()?;
        let current:Review=settings::read_json_limit(&self.root.join(format!("{source_id}.review.json")),2_097_152)?;
        if current.nonce!=nonce{return Err("A revisão mudou; atualize antes de cancelar.".into());}
        std::fs::remove_file(self.root.join(format!("{source_id}.review.json"))).map_err(|_|"Não foi possível cancelar a revisão.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cli()->Target{Target{kind:"claudeCli".into(),profile_id:"claude-test".into(),provider:"Claude".into(),account:"test@example.invalid".into(),billing:"subscription".into(),credential_revision:None}}
    fn api()->Target{Target{kind:"api".into(),profile_id:uuid::Uuid::new_v4().to_string(),provider:"OpenAI".into(),account:"Local key".into(),billing:"api".into(),credential_revision:Some(uuid::Uuid::new_v4().to_string())}}
    #[test]
    fn chat_transfer_requires_revision_identity_billing_single_nonce(){
        let root=std::env::temp_dir().join(format!("capy-transfer-{}",uuid::Uuid::new_v4()));let store=Store::load(root.clone());
        let source=store.create("Task".into(),cli(),"sonnet".into()).unwrap();let target=api();
        let review=store.prepare_transfer(&source.id,target.clone(),"chosen-model".into()).unwrap();
        assert!(store.approve_transfer(&source.id,&review.nonce,review.summary.clone(),false,true,&source.target,&target).is_err());
        assert!(store.approve_transfer(&source.id,&review.nonce,review.summary.clone(),true,false,&source.target,&target).is_err());
        let mut drift=target.clone();drift.credential_revision=Some(uuid::Uuid::new_v4().to_string());
        assert!(store.approve_transfer(&source.id,&review.nonce,review.summary.clone(),true,true,&source.target,&drift).is_err());
        let mut wrong_source=source.target.clone();wrong_source.account="other@example.invalid".into();
        assert!(store.approve_transfer(&source.id,&review.nonce,review.summary.clone(),true,true,&wrong_source,&target).is_err());
        assert_eq!(store.transfer_review(&source.id).unwrap().nonce,review.nonce);
        let previous=review.clone();let review=store.prepare_transfer(&source.id,target.clone(),"chosen-model".into()).unwrap();
        assert_ne!(previous.nonce,review.nonce);
        assert!(store.approve_transfer(&source.id,&previous.nonce,previous.summary,true,true,&source.target,&target).is_err());
        let turn=uuid::Uuid::new_v4().to_string();store.begin(&source.id,0,turn.clone(),"sonnet".into(),"Use plan.md section API; retain compatibility".into()).unwrap();
        assert!(store.prepare_transfer(&source.id,target.clone(),"chosen-model".into()).is_err());
        store.finish(&source.id,&turn,Ok(crate::chat_api::Reply{text:"Decision: preserve API".into(),completed:true,note:None}),true).unwrap();
        assert!(store.approve_transfer(&source.id,&review.nonce,review.summary,true,true,&source.target,&target).is_err());
        let review=store.prepare_transfer(&source.id,target.clone(),"chosen-model".into()).unwrap();
        assert!(review.summary.decisions.contains("plan.md"));assert!(review.summary.state.contains("preserve API"));
        let destination=store.approve_transfer(&source.id,&review.nonce,review.summary.clone(),true,true,&source.target,&target).unwrap();
        assert_eq!(destination.state,"working");assert!(destination.messages[0].text.contains("plan.md"));
        assert_eq!(store.get(&source.id).unwrap().transferred_to.as_deref(),Some(destination.id.as_str()));
        let restored=Store::load(root.clone());
        assert!(restored.approve_transfer(&source.id,&review.nonce,review.summary,true,true,&source.target,&target).is_err());
        assert_eq!(restored.list().unwrap().len(),2);restored.recover_interrupted().unwrap();assert_eq!(restored.get(&destination.id).unwrap().state,"unknown");
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn chat_transfer_journal_recovers_commit_without_repeating_provider_call(){
        let root=std::env::temp_dir().join(format!("capy-transfer-journal-{}",uuid::Uuid::new_v4()));let store=Store::load(root.clone());
        let mut source=store.create("Task".into(),cli(),"sonnet".into()).unwrap();let before_source=source.clone();let mut destination=source.clone();
        destination.id=uuid::Uuid::new_v4().to_string();destination.target=api();destination.state="working".into();destination.revision=1;
        let nonce=uuid::Uuid::new_v4().to_string();destination.active_nonce=Some(nonce.clone());destination.used_nonces=vec![nonce];destination.messages=vec![crate::chat_api::Message{role:"user".into(),text:"Reviewed context".into()}];
        source.state="transferred".into();source.transferred_to=Some(destination.id.clone());source.revision=1;
        source.used_nonces.push(uuid::Uuid::new_v4().to_string());
        let journal=Journal{before_source,source:source.clone(),destination:destination.clone(),before_ids:vec![source.id.clone()],ids:vec![source.id.clone(),destination.id.clone()]};
        settings::write_json_limit(&root.join("transfer-pending.json"),&journal,8_388_608).unwrap();
        settings::write_json_limit(&root.join(format!("{}.json",destination.id)),&destination,2_097_152).unwrap();
        assert!(store.list().is_err());let recovered=Store::load(root.clone());
        assert_eq!(recovered.list().unwrap().len(),2);assert_eq!(recovered.get(&source.id).unwrap().state,"transferred");
        settings::write_json_limit(&root.join("transfer-pending.json"),&journal,8_388_608).unwrap();
        let committed=Store::load(root.clone());assert_eq!(committed.list().unwrap().len(),2);
        committed.recover_interrupted().unwrap();assert_eq!(committed.get(&destination.id).unwrap().state,"unknown");
        assert!(!root.join("transfer-pending.json").exists());std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn chat_transfer_in_progress_does_not_drop_another_conversation_result(){
        use std::{sync::{Arc,mpsc},time::Duration};
        let root=std::env::temp_dir().join(format!("capy-transfer-concurrent-{}",uuid::Uuid::new_v4()));let store=Arc::new(Store::load(root.clone()));
        let chat=store.create("Concurrent".into(),cli(),"sonnet".into()).unwrap();let nonce=uuid::Uuid::new_v4().to_string();
        store.begin(&chat.id,0,nonce.clone(),"sonnet".into(),"Hello".into()).unwrap();
        let guard=store.ids.lock().unwrap();std::fs::write(root.join("transfer-pending.json"),b"owned concurrent write fixture").unwrap();
        let (entered_tx,entered_rx)=mpsc::channel();let (tx,rx)=mpsc::channel();let worker=store.clone();let id=chat.id.clone();
        let thread=std::thread::spawn(move||{entered_tx.send(()).unwrap();tx.send(worker.finish(&id,&nonce,Ok(crate::chat_api::Reply{text:"Retained response".into(),completed:true,note:None}),true)).unwrap();});
        entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();assert!(matches!(rx.recv_timeout(Duration::from_millis(150)),Err(mpsc::RecvTimeoutError::Timeout)));
        std::fs::remove_file(root.join("transfer-pending.json")).unwrap();drop(guard);
        let result=rx.recv_timeout(Duration::from_secs(2)).unwrap().unwrap();assert_eq!(result.state,"completed");assert_eq!(result.messages.last().unwrap().text,"Retained response");
        thread.join().unwrap();std::fs::remove_dir_all(root).unwrap();
    }
}
