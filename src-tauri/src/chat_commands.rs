use crate::{api_accounts, chat_api, chat_cli, chat_history::{self,Conversation,Target},profiles};
use serde::Deserialize;
use std::sync::Arc;
use tauri::{Manager,State};

struct Transport {profile:Option<profiles::Profile>,key:Option<String>}
fn request_target(app:&tauri::AppHandle,request:&Create)->Result<Target,String>{
    match request.kind.as_str(){
        "claudeCli"=>{
            if request.credential_revision.is_some(){return Err("Perfil CLI não usa chave API da Capy.".into());}
            let profile=app.state::<profiles::Store>().get(&request.profile_id)?;
            Ok(Target{kind:request.kind.clone(),profile_id:profile.id,provider:"Claude".into(),account:request.expected_account.clone(),billing:request.expected_billing.clone(),credential_revision:None})
        }
        "api"=>{
            let account=app.state::<Arc<api_accounts::Store>>().get(&request.profile_id)?;
            if request.expected_billing!="api"||request.expected_account!=account.label||request.credential_revision.as_deref()!=Some(&account.revision){return Err("Confirme a conta e a cobrança API antes de continuar.".into());}
            Ok(Target{kind:request.kind.clone(),profile_id:account.id,provider:account.provider,account:account.label,billing:"api".into(),credential_revision:Some(account.revision)})
        }
        _=>Err("Escolha CLI Claude ou uma conta API cadastrada.".into()),
    }
}
fn verify_target(app:&tauri::AppHandle,target:&Target,cwd:Option<&std::path::Path>)->Result<Transport,String>{
    if !chat_history::valid_target(target){return Err("Destino incompatível.".into());}
    match target.kind.as_str(){
        "claudeCli"=>{
            let profile=app.state::<profiles::Store>().get(&target.profile_id)?;
            let identity=profiles::identity(&profile,cwd)?;
            if !identity.logged_in||identity.account.as_deref()!=Some(&target.account)||identity.billing!=target.billing{
                return Err("A conta ou cobrança do Claude mudou. Verifique o perfil antes de continuar.".into());
            }
            Ok(Transport{profile:Some(profile),key:None})
        }
        "api"=>{
            let store=app.state::<Arc<api_accounts::Store>>();let account=store.get(&target.profile_id)?;
            if account.provider!=target.provider||account.label!=target.account||target.credential_revision.as_deref()!=Some(&account.revision){return Err("A conta, chave ou provedor mudou. Confirme novamente antes de continuar.".into());}
            Ok(Transport{profile:None,key:Some(store.key(&account)?)})
        }
        _=>Err("Destino incompatível.".into()),
    }
}
fn execute_started(app:&tauri::AppHandle,store:&chat_history::Store,mut started:Conversation,transport:Transport,text:&str)->Result<Conversation,String>{
    if transport.profile.is_some(){
        let nonce=started.active_nonce.as_deref().ok_or("Envio sem identidade.")?;
        started=store.mark_cli_attempt(&started.id,nonce)?;
    }
    crate::chat_presence::publish(app,&started);
    let nonce=started.active_nonce.as_deref().ok_or("Envio sem identidade.")?;
    let (result,cli_started)=match store.workspace(&started.id){
        Ok(cwd)=>if let Some(profile)=transport.profile {chat_cli::send(&profile,&started,&cwd,text)}
            else if let Some(key)=transport.key {(chat_api::send(&started.target.provider,&started.model,&key,&started.messages),false)}
            else {(Err("Destino indisponível; nenhum envio foi feito.".into()),false)},
        Err(error)=>(Err(error),false),
    };
    let finished=store.finish(&started.id,nonce,result,cli_started)?;
    crate::chat_presence::publish(app,&finished);Ok(finished)
}

#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Create {
    pub title:String,
    pub kind:String,
    pub profile_id:String,
    pub model:String,
    pub expected_account:String,
    pub expected_billing:String,
    pub credential_revision:Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Send {
    pub id:String,
    pub revision:u64,
    pub nonce:String,
    pub model:String,
    pub text:String,
}

#[tauri::command]
pub fn list_api_accounts(store:State<'_,Arc<api_accounts::Store>>)->Result<Vec<api_accounts::View>,String>{store.list()}
#[tauri::command]
pub async fn add_api_account(app:tauri::AppHandle,label:String,provider:String,key:String)->Result<api_accounts::View,String>{
    let store=app.state::<Arc<api_accounts::Store>>().inner().clone();
    tauri::async_runtime::spawn_blocking(move||store.add(label,provider,key)).await.map_err(|_|"Não foi possível salvar a conta API.".to_owned())?
}
#[tauri::command]
pub fn list_chats(store:State<'_,Arc<chat_history::Store>>)->Result<Vec<Conversation>,String>{store.list()}
fn recovery_probe(app:&tauri::AppHandle,store:&chat_history::Store,chat:&Conversation)->Result<bool,String>{
    let cwd=store.workspace(&chat.id)?;
    let transport=verify_target(app,&chat.target,Some(&cwd))?;
    if let Some(profile)=transport.profile {crate::chat_recovery::cli_resume_available(&profile,chat,&cwd)}else{Ok(false)}
}
#[tauri::command]
pub async fn prepare_chat_recovery(app:tauri::AppHandle,id:String,revision:u64)->Result<Conversation,String>{
    tauri::async_runtime::spawn_blocking(move||{
        let store=app.state::<Arc<chat_history::Store>>();let chat=store.get(&id)?;
        if chat.revision!=revision{return Err("A conversa mudou. Atualize antes de revisar.".into());}
        let resume=recovery_probe(&app,&store,&chat)?;
        let reviewed=store.prepare_recovery(&id,revision,resume)?;
        crate::chat_presence::publish(&app,&reviewed);Ok(reviewed)
    }).await.map_err(|_|"Não foi possível preparar a revisão do envio incerto.".to_owned())?
}
#[tauri::command]
pub async fn approve_chat_recovery(app:tauri::AppHandle,id:String,revision:u64,nonce:String,reviewed:bool)->Result<Conversation,String>{
    tauri::async_runtime::spawn_blocking(move||{
        let store=app.state::<Arc<chat_history::Store>>();let chat=store.get(&id)?;
        let resume=recovery_probe(&app,&store,&chat)?;
        let recovered=store.approve_recovery(&id,revision,&nonce,reviewed,&chat.target,resume)?;
        crate::chat_presence::publish(&app,&recovered);Ok(recovered)
    }).await.map_err(|_|"A revisão foi interrompida; nenhuma mensagem será reenviada.".to_owned())?
}
#[tauri::command]
pub async fn create_chat(app:tauri::AppHandle,request:Create)->Result<Conversation,String>{
    tauri::async_runtime::spawn_blocking(move||{
        let target=request_target(&app,&request)?;verify_target(&app,&target,None)?;
        let record=app.state::<Arc<chat_history::Store>>().create(request.title,target,request.model)?;
        crate::chat_presence::publish(&app,&record);Ok(record)
    }).await.map_err(|_|"Não foi possível criar a conversa.".to_owned())?
}

#[tauri::command]
pub async fn send_chat(app:tauri::AppHandle,request:Send)->Result<Conversation,String>{
    tauri::async_runtime::spawn_blocking(move||{
        let store=app.state::<Arc<chat_history::Store>>().inner().clone();
        let before=store.get(&request.id)?;
        if before.revision!=request.revision{return Err("A conversa mudou. Atualize antes de enviar.".into());}
        let cwd=store.workspace(&before.id)?;
        let transport=verify_target(&app,&before.target,Some(&cwd))?;
        let started={let exit=app.state::<crate::exit_review::Service>();let _admission=exit.admit()?;
            store.begin(&request.id,request.revision,request.nonce.clone(),request.model,request.text.clone())?};
        execute_started(&app,&store,started,transport,&request.text)
    }).await.map_err(|_|"O envio foi interrompido; revise o histórico antes de continuar.".to_owned())?
}

#[tauri::command]
pub fn chat_transfer_review(store:State<'_,Arc<chat_history::Store>>,source_id:String)->Result<Option<crate::chat_transfer::Review>,String>{
    store.get(&source_id)?;
    if !store.root.join(format!("{source_id}.review.json")).exists(){return Ok(None);}
    store.transfer_review(&source_id).map(Some)
}
#[tauri::command]
pub async fn prepare_chat_transfer(app:tauri::AppHandle,source_id:String,request:Create)->Result<crate::chat_transfer::Review,String>{
    tauri::async_runtime::spawn_blocking(move||{
        let store=app.state::<Arc<chat_history::Store>>();let source=store.get(&source_id)?;
        if ["working","unknown"].contains(&source.state.as_str()){return Err("Aguarde um fim de turno confirmado antes de transferir.".into());}
        let workspace=store.workspace(&source.id)?;
        let source_transport=verify_target(&app,&source.target,Some(&workspace))?;
        let loaded_guides=source_transport.profile.as_ref().map(|profile|
            crate::loaded_instructions::render(&profile.config_dir,&source.id,&workspace)
                .unwrap_or_else(|error|Some(format!("Não foi possível ler referências capturadas: {error}")))).flatten();
        let destination=request_target(&app,&request)?;verify_target(&app,&destination,None)?;
        let review=store.prepare_transfer_with_guides(&source_id,destination,request.model,loaded_guides.as_deref())?;
        crate::chat_presence::refresh(&app);Ok(review)
    }).await.map_err(|_|"Não foi possível preparar a revisão.".to_owned())?
}
#[tauri::command]
pub async fn approve_chat_transfer(app:tauri::AppHandle,source_id:String,nonce:String,summary:crate::handoff::Summary,reviewed:bool,billing_confirmed:bool)->Result<Conversation,String>{
    tauri::async_runtime::spawn_blocking(move||{
        let store=app.state::<Arc<chat_history::Store>>().inner().clone();let review=store.transfer_review(&source_id)?;
        verify_target(&app,&review.source_target,Some(&store.workspace(&source_id)?))?;
        let transport=verify_target(&app,&review.destination,None)?;
        let started={let exit=app.state::<crate::exit_review::Service>();let _admission=exit.admit()?;
            store.approve_transfer(&source_id,&nonce,summary,reviewed,billing_confirmed,&review.source_target,&review.destination)?};
        let source=store.get(&source_id)?;crate::chat_presence::publish(&app,&source);
        let text=started.messages[0].text.clone();execute_started(&app,&store,started,transport,&text)
    }).await.map_err(|_|"Transferência interrompida; revise o histórico. Nenhum envio será repetido automaticamente.".to_owned())?
}
#[tauri::command]
pub fn cancel_chat_transfer(app:tauri::AppHandle,store:State<'_,Arc<chat_history::Store>>,source_id:String,nonce:String)->Result<(),String>{
    store.cancel_transfer(&source_id,&nonce)?;crate::chat_presence::refresh(&app);Ok(())
}
