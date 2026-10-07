use crate::{chat_api::Reply, chat_history::Conversation, profiles};
use std::{io::{Read,Write}, path::Path, process::{Command,Stdio}, sync::mpsc, time::{Duration,Instant}};

fn command(profile:&profiles::Profile,chat:&Conversation,cwd:&Path)->Result<Command,String>{
    if !crate::discovery::uuid(&chat.id)||!crate::chat_api::valid_model(&chat.model){return Err("Sessão ou modelo inválido.".into());}
    let mut command=profiles::claude_command(profile)?;
    command.current_dir(cwd).args(["-p","--output-format","json","--model",&chat.model,
        "--tools","","--disallowedTools","mcp__*","--strict-mcp-config","--mcp-config","{\"mcpServers\":{}}",
        "--disable-slash-commands","--setting-sources","user","--settings","{\"disableAllHooks\":true}"]);
    command.args([if chat.cli_started {"--resume"}else{"--session-id"},&chat.id]);
    command.env_remove("CLAUDE_CODE_RESUME_INTERRUPTED_TURN");
    command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
    Ok(command)
}

pub fn send(profile:&profiles::Profile,chat:&Conversation,cwd:&Path,text:&str)->(Result<Reply,String>,bool){
    let command=match command(profile,chat,cwd){Ok(command)=>command,Err(error)=>return (Err(error),false)};
    if text.trim().is_empty()||text.len()>196_608{return (Err("Mensagem inválida ou longa demais.".into()),false);}
    run(command,text.to_owned(),&chat.id,Duration::from_secs(180))
}

fn run(mut command:Command,text:String,id:&str,timeout:Duration)->(Result<Reply,String>,bool){
    let mut child=match crate::chat_process::spawn(&mut command){Ok(child)=>child,Err(_)=>return (Err("Não foi possível iniciar o Claude oficial com proteção de processo; nenhum envio foi feito.".into()),false)};
    let Some(mut stdin)=child.stdin.take()else{return (Err("Entrada do Claude indisponível.".into()),true)};
    let Some(stdout)=child.stdout.take()else{return (Err("Saída do Claude indisponível.".into()),true)};
    let (write_tx,write_rx)=mpsc::channel();
    std::thread::spawn(move||{let result=stdin.write_all(text.as_bytes());drop(stdin);let _=write_tx.send(result.is_ok());});
    let (read_tx,read_rx)=mpsc::channel();
    std::thread::spawn(move||{
        let mut bytes=Vec::new();let result=stdout.take(2_097_153).read_to_end(&mut bytes);
        let _=read_tx.send(result.map(|_|bytes));
    });
    let start=Instant::now();let mut input_confirmed=false;
    let status=loop{
        if !input_confirmed {
            match write_rx.try_recv(){
                Ok(true)=>input_confirmed=true,
                Ok(false)|Err(mpsc::TryRecvError::Disconnected)=>return (Err("O Claude não recebeu a mensagem inteira; nenhum reenvio foi feito.".into()),true),
                Err(mpsc::TryRecvError::Empty)=>{},
            }
        }
        match child.try_wait(){
            Ok(Some(status))=>break status,
            Ok(None) if start.elapsed()<timeout=>std::thread::sleep(Duration::from_millis(25)),
            _=>return (Err("O Claude foi interrompido ou excedeu 180 segundos. O consumo pode ter ocorrido; nenhum reenvio foi feito.".into()),true),
        }
    };
    if !input_confirmed && write_rx.recv_timeout(Duration::from_secs(2))!=Ok(true){
        return (Err("A entrega da mensagem não foi confirmada; nenhum reenvio foi feito.".into()),true);
    }
    let bytes=match read_rx.recv_timeout(Duration::from_secs(2)){
        Ok(Ok(bytes)) if bytes.len()<=2_097_152=>bytes,
        _=>return (Err("A resposta do Claude foi interrompida ou excedeu 2 MiB; nenhum reenvio foi feito.".into()),true),
    };
    let value=match serde_json::from_slice(&bytes){Ok(value)=>value,Err(_)=>return (Err("O Claude não retornou um resultado JSON válido; nenhum reenvio foi feito.".into()),true)};
    (parse(id,status.success(),&value),true)
}

fn parse(id:&str,process_success:bool,value:&serde_json::Value)->Result<Reply,String>{
    if !process_success||value["type"]!="result"||value["session_id"].as_str()!=Some(id)
        ||value["subtype"]!="success"||value["is_error"]!=false{
        return Err("O Claude não confirmou sucesso para esta sessão. Nenhum reenvio ou troca de cobrança foi feito.".into());
    }
    let text=value["result"].as_str().filter(|s|!s.trim().is_empty()&&s.len()<=65_536)
        .ok_or("O Claude não retornou texto utilizável dentro do limite de 64 KiB.")?;
    Ok(Reply{text:text.into(),completed:true,note:None})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chat_cli_request_uses_exact_session_and_subscription_env(){
        let profile=profiles::Profile{id:"test".into(),label:"Test".into(),provider:"Claude".into(),config_dir:std::env::temp_dir(),billing:"subscription".into()};
        let root=std::env::temp_dir().join(format!("capy-cli-contract-{}",uuid::Uuid::new_v4()));
        let store=crate::chat_history::Store::load(root.clone());
        let target=crate::chat_history::Target{kind:"claudeCli".into(),profile_id:profile.id.clone(),provider:"Claude".into(),account:"test@example.invalid".into(),billing:"subscription".into(),credential_revision:None};
        let mut chat=store.create("Test".into(),target,"sonnet".into()).unwrap();
        let cmd=command(&profile,&chat,&root).unwrap();let args:Vec<_>=cmd.get_args().map(|s|s.to_string_lossy().into_owned()).collect();
        assert!(args.windows(2).any(|w|w==["--session-id",chat.id.as_str()]));assert!(!args.iter().any(|s|s=="--continue"||s=="--bare"));
        assert!(args.windows(2).any(|w|w==["--tools",""]));assert!(args.windows(2).any(|w|w==["--setting-sources","user"]));
        assert!(args.windows(2).any(|w|w==["--settings","{\"disableAllHooks\":true}"]));
        for key in profiles::AUTH_OVERRIDE_VARS {assert!(cmd.get_envs().any(|(k,v)|k==*key&&v.is_none()));}
        assert!(cmd.get_envs().any(|(key,value)|key=="CLAUDE_CONFIG_DIR"&&value==Some(profile.config_dir.as_os_str())),"Isolated profiles must keep their explicit configuration directory");
        chat.cli_started=true;let resumed=command(&profile,&chat,&root).unwrap();
        let args:Vec<_>=resumed.get_args().map(|s|s.to_string_lossy().into_owned()).collect();
        assert!(args.windows(2).any(|w|w==["--resume",chat.id.as_str()]));assert!(!args.iter().any(|s|s=="--session-id"));
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn chat_cli_completion_rejects_exit_alone_error_and_wrong_session(){
        let id=uuid::Uuid::new_v4().to_string();let value=serde_json::json!({"type":"result","session_id":id,"subtype":"success","is_error":false,"result":"Hello"});
        assert!(parse(&id,true,&value).unwrap().completed);assert!(parse(&id,false,&value).is_err());
        assert!(parse(&uuid::Uuid::new_v4().to_string(),true,&value).is_err());
        assert!(parse(&id,true,&serde_json::json!({"type":"system","session_id":id})).is_err());
        let mut failure=value;failure["is_error"]=true.into();assert!(parse(&id,true,&failure).is_err());
    }
    #[test]
    #[ignore = "Consumes two short turns from the explicitly selected local Claude subscription"]
    fn chat_cli_live_subscription_keeps_exact_history_across_model_change(){
        let root=std::env::temp_dir().join(format!("capy-live-chat-{}",uuid::Uuid::new_v4()));
        let profiles=profiles::Store::load(root.clone());let profile=profiles.get("claude-default").unwrap();
        let identity=profiles::identity(&profile,None).unwrap();
        assert!(identity.logged_in);assert_eq!(identity.billing,"subscription","Live proof never falls back to API billing");
        let account=identity.account.unwrap();
        let store=crate::chat_history::Store::load(root.join("chat"));
        let chat=store.create("Own live chat proof".into(),crate::chat_history::Target{kind:"claudeCli".into(),profile_id:profile.id.clone(),provider:"Claude".into(),account:account.clone(),billing:"subscription".into(),credential_revision:None},"sonnet".into()).unwrap();
        let cwd=store.workspace(&chat.id).unwrap();
        let marker=format!("capy_ctx_{}",uuid::Uuid::new_v4());
        let prompt=format!("Memorize esta palavra para a próxima mensagem: {marker}. Responda apenas CAPY_CHAT_STARTED. Não execute ferramentas.");
        let nonce=uuid::Uuid::new_v4().to_string();let pending=store.begin(&chat.id,chat.revision,nonce.clone(),"sonnet".into(),prompt.clone()).unwrap();
        let (reply,started)=send(&profile,&pending,&cwd,&prompt);assert!(started);
        let reply=reply.unwrap();assert!(reply.completed);assert!(reply.text.contains("CAPY_CHAT_STARTED"));
        let completed=store.finish(&chat.id,&nonce,Ok(reply),started).unwrap();
        let reverified=profiles::identity(&profile,Some(&cwd)).unwrap();
        assert_eq!(reverified.account.as_deref(),Some(account.as_str()));assert_eq!(reverified.billing,"subscription");
        let prompt="Qual é a palavra que pedi para memorizar na mensagem anterior? Responda apenas essa palavra, sem ferramentas.";
        let nonce=uuid::Uuid::new_v4().to_string();let pending=store.begin(&chat.id,completed.revision,nonce.clone(),"haiku".into(),prompt.into()).unwrap();
        assert!(pending.cli_started);let (reply,started)=send(&profile,&pending,&cwd,prompt);
        let reply=reply.unwrap();assert!(reply.completed);assert!(reply.text.contains(&marker),"Resumed conversation must retain the exact previous context");
        let final_chat=store.finish(&chat.id,&nonce,Ok(reply),started).unwrap();
        assert_eq!(final_chat.id,chat.id);assert_eq!(final_chat.model,"haiku");assert_eq!(final_chat.messages.len(),4);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[cfg(windows)]
    #[test]
    fn chat_cli_native_pipe_delivers_utf8_literal_and_kills_own_timeout(){
        use std::os::windows::process::CommandExt;
        let id=uuid::Uuid::new_v4().to_string();
        let text="Olá capivara: `$(not-a-command)` --flag\n".repeat(500);
        let shell=std::path::PathBuf::from(std::env::var_os("SystemRoot").unwrap()).join("System32/WindowsPowerShell/v1.0/powershell.exe");
        let mut mock=Command::new(&shell);
        mock.creation_flags(0x08000000).args(["-NoProfile","-NonInteractive","-Command",
            "[Console]::InputEncoding=[Text.UTF8Encoding]::new($false); [Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); $message=[Console]::In.ReadToEnd(); @{type='result';session_id=$env:CAPY_CHAT_PROOF_ID;subtype='success';is_error=$false;result=$message} | ConvertTo-Json -Compress"])
            .env("CAPY_CHAT_PROOF_ID",&id).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
        let (reply,started)=run(mock,text.clone(),&id,Duration::from_secs(15));
        assert!(started);assert_eq!(reply.unwrap().text,text);
        let mut stalled=Command::new(&shell);
        stalled.creation_flags(0x08000000).args(["-NoProfile","-NonInteractive","-Command","Start-Sleep -Seconds 30"])
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
        let at=Instant::now();let (reply,started)=run(stalled,"Hello".into(),&id,Duration::from_millis(150));
        assert!(started);assert!(reply.is_err());assert!(at.elapsed()<Duration::from_secs(5));
    }
}
