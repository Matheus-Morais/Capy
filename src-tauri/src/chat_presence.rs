use crate::{chat_history::Conversation,demo::Session};
use std::{collections::HashMap,sync::{Arc,Mutex}};
use tauri::{Emitter,Manager};

#[derive(Default)]
pub struct Service { completions:Mutex<HashMap<String,(String,u64)>> }
impl Service {
    pub fn observe(&self,chat:&Conversation,now:u64){
        if let Ok(mut events)=self.completions.lock(){
            events.retain(|_,(_,at)|now>=*at&&now-*at<=10_000);
            if chat.state=="completed" {
                events.insert(chat.id.clone(),(format!("{}:{}",chat.id,chat.revision),now));
            }else{events.remove(&chat.id);}
        }
    }
    pub fn rows(&self,chats:&[Conversation],now:u64)->Vec<Session>{
        let Ok(mut events)=self.completions.lock()else{return vec![];};
        events.retain(|_,(_,at)|now>=*at&&now-*at<=10_000);
        let mut ordinary=0;
        chats.iter().rev().filter(|chat|{
            if chat.state=="working"||events.contains_key(&chat.id){true}else{ordinary+=1;ordinary<=64}
        }).map(|chat|Session{
            id:format!("chat:{}",chat.id),project:chat.title.clone(),agent:chat.target.provider.clone(),symbol:"C".into(),kind:"chat".into(),
            origin:format!("{} · {}",chat.target.account,if chat.target.billing=="subscription"{"assinatura"}else{"API por uso"}),
            state:match chat.state.as_str(){"working"=>"working","unknown"=>"waiting","failed"=>"unknown",_=>"idle"}.into(),
            request:(chat.state=="unknown").then(||"Revisar envio interrompido no chat".into()),
            message:match chat.state.as_str(){
                "working"=>"Aguardando resposta no chat da Capy.","completed"=>"Resposta concluída. Abra o chat para conferir.",
                "partial"=>"Resposta parcial. Confira o aviso no chat.","failed"|"unknown"=>"O envio precisa de revisão. Confira o histórico antes de continuar.",
                "transferred"=>"Conversa transferida; confira a continuação.",_=>"Conversa disponível na Capy.",
            }.into(),command:None,hidden:false,
            source_action:Some(crate::source_access::SourceAction{label:"Abrir chat na Capy".into(),available:true,reason:None}),
            completion:events.get(&chat.id).map(|(token,_)|token.clone()),
        }).collect()
    }
}

pub fn publish(app:&tauri::AppHandle,chat:&Conversation){
    let service=app.state::<Service>();let now=crate::quotas::now_ms();service.observe(chat,now);
    let _=app.emit("chat-updated",chat);
    refresh(app);
}
pub fn refresh(app:&tauri::AppHandle){
    let service=app.state::<Service>();let now=crate::quotas::now_ms();
    let state=app.state::<crate::DesktopState>();
    let _=(||{
        let mut data=state.demo.lock().ok()?;
        if data.scenario!="real"{return None;}
        let chats=app.state::<Arc<crate::chat_history::Store>>().list().ok()?;
        let reviews=app.state::<Arc<crate::chat_history::Store>>().transfer_reviews().ok()?;
        let mut rows=service.rows(&chats,now);
        state.preferences.lock().ok()?.apply(&mut rows);
        data.sessions.retain(|s|s.kind!="chat");data.sessions.extend(rows);data.chat_transfers=reviews;
        let _=app.emit("demo-updated",data.clone());Some(())
    })();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chat_presence_interruption_requests_review_and_never_celebrates_acknowledgement(){
        let root=std::env::temp_dir().join(format!("capy-presence-interrupted-{}",uuid::Uuid::new_v4()));
        let store=crate::chat_history::Store::load(root.clone());
        let target=crate::chat_history::Target{kind:"api".into(),profile_id:uuid::Uuid::new_v4().to_string(),provider:"OpenAI".into(),account:"Fixture".into(),billing:"api".into(),credential_revision:Some(uuid::Uuid::new_v4().to_string())};
        let chat=store.create("Own interruption".into(),target,"model".into()).unwrap();
        store.begin(&chat.id,0,uuid::Uuid::new_v4().to_string(),"model".into(),"Uncertain".into()).unwrap();store.recover_interrupted().unwrap();
        let unknown=store.get(&chat.id).unwrap();let service=Service::default();service.observe(&unknown,100);
        let rows=service.rows(&[unknown.clone()],100);assert_eq!(rows[0].state,"waiting");assert!(rows[0].request.is_some());assert!(rows[0].completion.is_none());
        let prepared=store.prepare_recovery(&chat.id,unknown.revision,false).unwrap();
        let nonce=prepared.recovery_review.as_ref().unwrap().nonce.clone();
        let acknowledged=store.approve_recovery(&chat.id,prepared.revision,&nonce,true,&chat.target,false).unwrap();service.observe(&acknowledged,200);
        let rows=service.rows(&[acknowledged],200);assert_eq!(rows[0].state,"unknown");assert!(rows[0].request.is_none());assert!(rows[0].completion.is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn chat_presence_completion_is_ephemeral_exact_and_never_inferred_from_history(){
        let root=std::env::temp_dir().join(format!("capy-presence-{}",uuid::Uuid::new_v4()));
        let store=crate::chat_history::Store::load(root.clone());
        let target=crate::chat_history::Target{kind:"api".into(),profile_id:uuid::Uuid::new_v4().to_string(),provider:"OpenAI".into(),account:"Test".into(),billing:"api".into(),credential_revision:Some(uuid::Uuid::new_v4().to_string())};
        let chat=store.create("Test".into(),target,"chosen-model".into()).unwrap();
        let service=Service::default();let nonce=uuid::Uuid::new_v4().to_string();
        let working=store.begin(&chat.id,0,nonce.clone(),chat.model.clone(),"Hello".into()).unwrap();
        service.observe(&working,100);let rows=service.rows(&[working],100);assert_eq!(rows[0].state,"working");assert!(rows[0].completion.is_none());
        let completed=store.finish(&chat.id,&nonce,Ok(crate::chat_api::Reply{text:"Success".into(),completed:true,note:None}),false).unwrap();
        assert!(service.rows(&[completed.clone()],200)[0].completion.is_none());
        service.observe(&completed,200);let first=service.rows(&[completed.clone()],201);
        assert_eq!(first[0].id,format!("chat:{}",chat.id));assert!(first[0].completion.is_some());assert_eq!(first[0].state,"idle");
        assert_eq!(first[0].completion,service.rows(&[completed.clone()],500)[0].completion);
        service.observe(&completed,600);assert_eq!(first[0].completion,service.rows(&[completed.clone()],600)[0].completion);
        assert!(Service::default().rows(&[completed.clone()],500)[0].completion.is_none());
        assert!(service.rows(&[completed],10_601)[0].completion.is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
}
