use crate::{chat_history::Conversation,tasks::Task};
use serde::Serialize;
use std::sync::{Mutex,MutexGuard};

#[derive(Clone,Debug,PartialEq,Serialize)]
#[serde(rename_all="camelCase")]
pub struct Resource {
    pub kind:String,pub id:String,pub label:String,pub provider:String,pub account:String,
    pub billing:String,pub model:String,pub cwd:Option<String>,pub revision:Option<u64>,pub send_nonce:Option<String>,
}
#[derive(Clone,Serialize)]
#[serde(rename_all="camelCase")]
pub struct Review {pub nonce:String,pub expires_at:u64,pub resources:Vec<Resource>}
#[derive(Default)]
struct State {pending:Option<Review>,closing:bool}
#[derive(Default)]
pub struct Service {state:Mutex<State>}
pub struct Admission<'a> { _guard:MutexGuard<'a,State> }

pub fn resources(chats:Vec<Conversation>,tasks:Vec<Task>,active_terminals:Vec<String>)->Result<Vec<Resource>,String>{
    let mut result:Vec<_>=chats.into_iter().filter(|chat|chat.state=="working").map(|chat|Resource{
        kind:"chat".into(),id:chat.id,label:chat.title,provider:chat.target.provider,account:chat.target.account,
        billing:chat.target.billing,model:chat.model,cwd:None,revision:Some(chat.revision),send_nonce:chat.active_nonce,
    }).collect();
    for id in active_terminals {
        let task=tasks.iter().find(|task|task.id==id&&task.mode=="embedded").ok_or("A identidade de um terminal ativo não pôde ser conferida. A saída foi bloqueada.")?;
        result.push(Resource{kind:"terminal".into(),id:task.id.clone(),label:"Terminal integrado".into(),provider:"Claude".into(),
            account:task.account.clone().unwrap_or_else(||"Conta indisponível".into()),billing:task.billing.clone(),model:task.model.clone(),
            cwd:Some(task.cwd.to_string_lossy().into_owned()),revision:None,send_nonce:None});
    }
    result.sort_by(|a,b|(&a.kind,&a.id).cmp(&(&b.kind,&b.id)));Ok(result)
}
impl Service {
    pub fn reopen_after_close_failure(&self)->Result<(),String>{
        let mut state=self.state.lock().map_err(|_|"Controle de saída indisponível.")?;state.closing=false;state.pending=None;Ok(())
    }
    pub fn admit(&self)->Result<Admission<'_>,String>{
        let guard=self.state.lock().map_err(|_|"Controle de saída indisponível.")?;
        if guard.closing{return Err("A Capy está encerrando. Nenhum novo trabalho foi iniciado.".into());}
        Ok(Admission{_guard:guard})
    }
    pub fn prepare(&self,now:u64,snapshot:impl FnOnce()->Result<Vec<Resource>,String>)->Result<Review,String>{
        let started=std::time::Instant::now();
        let mut state=self.state.lock().map_err(|_|"Controle de saída indisponível.")?;
        if state.closing{return Err("A Capy já está encerrando.".into());}
        state.pending=None;
        let resources=snapshot()?;
        let review=Review{nonce:uuid::Uuid::new_v4().to_string(),expires_at:now.saturating_add(started.elapsed().as_millis() as u64).saturating_add(60_000),resources};
        state.pending=Some(review.clone());Ok(review)
    }
    pub fn cancel(&self,nonce:&str)->Result<(),String>{
        let mut state=self.state.lock().map_err(|_|"Controle de saída indisponível.")?;
        if state.pending.as_ref().is_some_and(|review|review.nonce==nonce){state.pending=None;}
        Ok(())
    }
    pub fn approve(&self,nonce:&str,confirmed:bool,now:u64,snapshot:impl FnOnce()->Result<Vec<Resource>,String>)->Result<(),String>{
        let started=std::time::Instant::now();
        if !confirmed{return Err("Saída não confirmada.".into());}
        let mut state=self.state.lock().map_err(|_|"Controle de saída indisponível.")?;
        let review=state.pending.as_ref().ok_or("Solicite a saída novamente para conferir as sessões ativas.")?;
        if state.closing||review.nonce!=nonce{return Err("Esta confirmação de saída foi substituída ou já utilizada.".into());}
        if now>=review.expires_at {state.pending=None;return Err("A confirmação expirou. Solicite a saída novamente.".into());}
        let actual=snapshot()?;
        if now.saturating_add(started.elapsed().as_millis() as u64)>=review.expires_at {state.pending=None;return Err("A confirmação expirou. Solicite a saída novamente.".into());}
        if review.resources!=actual {state.pending=None;return Err("Os chats ou terminais mudaram. Solicite a saída novamente e confira a lista atual.".into());}
        state.pending=None;state.closing=true;Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn item()->Resource{Resource{kind:"chat".into(),id:uuid::Uuid::new_v4().to_string(),label:"Own chat".into(),provider:"Claude".into(),account:"Fixture".into(),billing:"subscription".into(),model:"haiku".into(),cwd:None,revision:Some(1),send_nonce:Some(uuid::Uuid::new_v4().to_string())}}
    #[test]
    fn exit_review_requires_exact_fresh_single_approval(){
        let service=Service::default();let active=vec![item()];
        let first=service.prepare(100,||Ok(active.clone())).unwrap();
        assert!(service.approve(&first.nonce,false,101,||Ok(active.clone())).is_err());
        let second=service.prepare(101,||Ok(active.clone())).unwrap();
        service.cancel(&first.nonce).unwrap();assert!(service.approve(&first.nonce,true,102,||Ok(active.clone())).is_err());
        assert!(service.approve(&second.nonce,true,60_101,||Ok(active.clone())).is_err());
        let third=service.prepare(200,||Ok(active.clone())).unwrap();let mut changed=active.clone();changed[0].send_nonce=Some(uuid::Uuid::new_v4().to_string());
        assert!(service.approve(&third.nonce,true,201,||Ok(changed)).is_err());
        assert!(service.approve(&third.nonce,true,202,||Ok(active.clone())).is_err());
        for field in ["revision","account","billing","model","id","added","removed"]{
            let review=service.prepare(210,||Ok(active.clone())).unwrap();let mut changed=active.clone();
            match field {"revision"=>changed[0].revision=Some(2),"account"=>changed[0].account="Other".into(),
                "billing"=>changed[0].billing="api".into(),"model"=>changed[0].model="sonnet".into(),
                "id"=>changed[0].id=uuid::Uuid::new_v4().to_string(),"added"=>changed.push(item()),"removed"=>changed.clear(),_=>unreachable!()}
            assert!(service.approve(&review.nonce,true,211,||Ok(changed)).is_err(),"{field}");
        }
        let fourth=service.prepare(300,||Ok(active.clone())).unwrap();service.cancel(&fourth.nonce).unwrap();
        assert!(service.approve(&fourth.nonce,true,301,||Ok(active.clone())).is_err());
        let unreadable=service.prepare(310,||Ok(active.clone())).unwrap();
        assert!(service.prepare(311,||Err("Unreadable history".into())).is_err());
        assert!(service.approve(&unreadable.nonce,true,312,||Ok(active.clone())).is_err());
        let last=service.prepare(400,||Ok(active.clone())).unwrap();
        assert!(service.approve(&last.nonce,true,401,||Err("Unavailable".into())).is_err());
        service.approve(&last.nonce,true,402,||Ok(active)).unwrap();
        assert!(service.approve(&last.nonce,true,403,||Ok(vec![])).is_err());assert!(service.admit().is_err());
        service.reopen_after_close_failure().unwrap();assert!(service.admit().is_ok());
        assert!(service.approve(&last.nonce,true,404,||Ok(vec![])).is_err());
    }
    #[test]
    fn exit_review_blocks_admission_after_approval_and_serializes_new_work(){
        let service=std::sync::Arc::new(Service::default());let active=std::sync::Arc::new(Mutex::new(vec![item()]));
        let review=service.prepare(0,||Ok(active.lock().unwrap().clone())).unwrap();
        let admission=service.admit().unwrap();
        let worker_service=service.clone();let worker_active=active.clone();let (sender,receiver)=std::sync::mpsc::channel();
        let worker=std::thread::spawn(move||{sender.send(()).unwrap();worker_service.approve(&review.nonce,true,1,||Ok(worker_active.lock().unwrap().clone()))});
        receiver.recv().unwrap();active.lock().unwrap().push(item());drop(admission);
        assert!(worker.join().unwrap().is_err());assert!(service.admit().is_ok());
        let current=service.prepare(2,||Ok(active.lock().unwrap().clone())).unwrap();service.approve(&current.nonce,true,3,||Ok(active.lock().unwrap().clone())).unwrap();
        assert!(service.admit().is_err());assert!(service.prepare(4,||Ok(vec![])).is_err());
    }
    #[test]
    fn exit_review_expiry_includes_time_waiting_for_admission(){
        let service=std::sync::Arc::new(Service::default());let review=service.prepare(0,||Ok(vec![])).unwrap();
        service.state.lock().unwrap().pending.as_mut().unwrap().expires_at=10;
        let admission=service.admit().unwrap();let worker_service=service.clone();let (sender,receiver)=std::sync::mpsc::channel();
        let worker=std::thread::spawn(move||worker_service.approve(&review.nonce,true,0,||{std::thread::sleep(std::time::Duration::from_millis(30));sender.send(()).unwrap();Ok(vec![])}));
        std::thread::sleep(std::time::Duration::from_millis(30));drop(admission);
        receiver.recv().unwrap();assert!(worker.join().unwrap().is_err());assert!(service.admit().is_ok());
    }
    #[test]
    fn exit_resources_include_only_working_owned_chats_and_active_terminals(){
        let root=std::env::temp_dir().join(format!("capy-exit-{}",uuid::Uuid::new_v4()));let store=crate::chat_history::Store::load(root.clone());
        let target=crate::chat_history::Target{kind:"claudeCli".into(),profile_id:"own".into(),provider:"Claude".into(),account:"Fixture".into(),billing:"subscription".into(),credential_revision:None};
        let chat=store.create("Own".into(),target,"haiku".into()).unwrap();let idle=store.create("Idle".into(),chat.target.clone(),"haiku".into()).unwrap();
        let working=store.begin(&chat.id,0,uuid::Uuid::new_v4().to_string(),"haiku".into(),"Own instruction".into()).unwrap();
        let task=Task{id:uuid::Uuid::new_v4().to_string(),profile_id:"own".into(),account:Some("Fixture".into()),billing:"subscription".into(),model:"sonnet".into(),cwd:root.clone(),prompt:"Own".into(),mode:"embedded".into(),created_at:0};
        let result=resources(vec![working.clone(),idle],vec![task.clone()],vec![task.id.clone()]).unwrap();
        assert_eq!(result.len(),2);assert_eq!(result[0].id,working.id);assert_eq!(result[0].send_nonce,working.active_nonce);assert_eq!(result[0].revision,Some(1));
        assert_eq!(result[1].id,task.id);assert_eq!(result[1].cwd,Some(root.to_string_lossy().into_owned()));
        assert!(resources(vec![],vec![task.clone()],vec!["other".into()]).is_err());
        let mut external=task;external.mode="external".into();assert!(resources(vec![],vec![external.clone()],vec![external.id]).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
