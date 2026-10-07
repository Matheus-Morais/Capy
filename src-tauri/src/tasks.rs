use crate::{profiles::{self,Profile}, settings, terminal};
use serde::{Deserialize,Serialize};
use std::{path::{Path,PathBuf},sync::Mutex};

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct Task {
    pub id:String, pub profile_id:String, pub account:Option<String>,pub billing:String,
    pub model:String,pub cwd:PathBuf,pub prompt:String,pub mode:String,pub created_at:u64,
}
#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Start {
    pub profile_id:String,pub model:String,pub cwd:PathBuf,pub prompt:String,pub mode:String,
    pub expected_account:Option<String>,pub expected_billing:String,
}
const TASK_RECORD_LIMIT:u64=2_097_152;
pub struct Store {root:PathBuf,tasks:Mutex<Vec<Task>>,load_error:Option<String>}
impl Store {
    pub fn load(root:PathBuf)->Self {
        let index=root.join("task-index.json");
        let result=(||->Result<Vec<Task>,String>{
            if index.exists(){
                let ids:Vec<String>=settings::read_json(&index)?;
                let mut seen=std::collections::HashSet::new();
                if ids.len()>500||ids.iter().any(|id|!crate::discovery::uuid(id)||!seen.insert(id)){return Err("Índice de tarefas inválido.".into());}
                ids.into_iter().map(|id|{
                    let task:Task=settings::read_json_limit(&root.join("tasks").join(format!("{id}.json")),TASK_RECORD_LIMIT)?;
                    if task.id!=id{return Err("Identidade do registro de tarefa não corresponde ao índice.".into());}Ok(task)
                }).collect()
            }else if root.join("tasks.json").exists(){
                settings::read_json_limit(&root.join("tasks.json"),134_217_728)
            }else{Ok(Vec::new())}
        })();
        match result {
            Ok(tasks)=>Self{root,tasks:Mutex::new(tasks),load_error:None},
            Err(_)=>Self{root,tasks:Mutex::new(Vec::new()),load_error:Some("O histórico de tarefas não pôde ser lido. Os arquivos foram preservados; recupere o índice/registros antes de iniciar outra tarefa.".into())},
        }
    }
    pub fn list(&self)->Result<Vec<Task>,String>{
        if let Some(error)=&self.load_error{return Err(error.clone());}
        self.tasks.lock().map(|v|v.clone()).map_err(|_|"Tarefas indisponíveis.".into())
    }
    pub fn get(&self,id:&str)->Result<Task,String>{self.list()?.into_iter().find(|t|t.id==id).ok_or("Tarefa não encontrada.".into())}
    pub fn prepare(&self,profile:&Profile,request:Start)->Result<Task,String>{
        validate(&request)?;
        let identity=profiles::identity(profile,Some(&request.cwd))?;
        if !identity.logged_in || identity.account!=request.expected_account || identity.billing!=request.expected_billing {
            return Err("A conta ou cobrança mudou. Verifique o vínculo novamente antes de iniciar.".into());
        }
        Ok(Task{id:uuid::Uuid::new_v4().to_string(),profile_id:profile.id.clone(),account:identity.account,billing:identity.billing,model:request.model,cwd:request.cwd.canonicalize().map_err(|_|"Pasta indisponível.")?,prompt:request.prompt,mode:request.mode,created_at:crate::quotas::now_ms()})
    }
    fn remember(&self,task:Task)->Result<(),String>{
        if let Some(error)=&self.load_error{return Err(error.clone());}
        let mut tasks=self.tasks.lock().map_err(|_|"Tarefas indisponíveis.")?;
        let mut updated=tasks.clone();updated.push(task);
        if updated.len()>500 {updated.drain(..updated.len()-500);}
        if !self.root.join("task-index.json").exists(){
            for previous in tasks.iter(){self.write_task(previous)?;}
        }
        self.write_task(updated.last().ok_or("Registro de tarefa ausente.")?)?;
        self.write_index(&updated)?;
        for removed in tasks.iter().filter(|old|!updated.iter().any(|new|new.id==old.id)){
            let _=std::fs::remove_file(self.root.join("tasks").join(format!("{}.json",removed.id)));
        }
        *tasks=updated;Ok(())
    }
    fn write_task(&self,task:&Task)->Result<(),String>{
        if !crate::discovery::uuid(&task.id){return Err("Identificador de tarefa inválido.".into());}
        settings::write_json_limit(&self.root.join("tasks").join(format!("{}.json",task.id)),task,TASK_RECORD_LIMIT)
    }
    fn write_index(&self,tasks:&[Task])->Result<(),String>{
        settings::write_json(&self.root.join("task-index.json"),&tasks.iter().map(|t|&t.id).collect::<Vec<_>>())
    }
    fn forget(&self,id:&str)->Result<(),String>{
        let mut tasks=self.tasks.lock().map_err(|_|"Tarefas indisponíveis.")?;
        let updated=tasks.iter().filter(|t|t.id!=id).cloned().collect::<Vec<_>>();
        self.write_index(&updated)?;*tasks=updated;
        if crate::discovery::uuid(id){let _=std::fs::remove_file(self.root.join("tasks").join(format!("{id}.json")));}Ok(())
    }
    pub fn launch(&self,task:Task,profile:&Profile,terminal:&terminal::Service,on_output:impl Fn(terminal::Chunk)+Send+'static,on_exit:impl Fn()+Send+'static)->Result<Task,String>{
        let tool_free=std::env::var_os("CAPY_VISUAL_TEST_NO_TOOLS").is_some_and(|value|value=="1");
        let args=task_arguments(&task,&self.root,tool_free)?;
        self.remember(task.clone())?;
        let result=(||{if task.mode=="external" {profiles::external(profile,&task.cwd,args,&self.root)}else{
            let mut command=portable_pty::CommandBuilder::new(profiles::claude_executable()?);
            command.args(args);command.cwd(&task.cwd);command.env("TERM","xterm-256color");
            if let Some(path)=profiles::config_override(profile){command.env("CLAUDE_CONFIG_DIR",path);}else{command.env_remove("CLAUDE_CONFIG_DIR");}
            for key in profiles::AUTH_OVERRIDE_VARS{command.env_remove(key);}
            terminal.start(task.id.clone(),command,on_output,on_exit)
        }})();
        if let Err(error)=result {
            self.forget(&task.id).map_err(|rollback|format!("{error} O registro da tentativa não pôde ser removido: {rollback}"))?;
            return Err(error);
        }
        Ok(task)
    }
    pub fn resume_external(&self,task:&Task,profile:&Profile)->Result<(),String>{
        profiles::external(profile,&task.cwd,vec!["--resume".into(),task.id.clone()],&self.root)
    }
}
pub fn validate(request:&Start)->Result<(),String>{
    if !request.cwd.is_absolute()||!request.cwd.is_dir(){return Err("Informe uma pasta absoluta existente.".into());}
    if !settings::valid_text(&request.model,128)||request.model.starts_with('-')||request.model.contains(char::is_whitespace){return Err("Modelo inválido.".into());}
    if request.prompt.trim().is_empty()||request.prompt.len()>262_144||request.prompt.contains('\0'){return Err("Instrução deve ter entre 1 e 262144 bytes.".into());}
    if !["external","embedded"].contains(&request.mode.as_str()){return Err("Escolha CLI externo ou terminal integrado.".into());}
    if !["subscription","api"].contains(&request.expected_billing.as_str()){return Err("Confirme a cobrança antes de iniciar.".into());}
    Ok(())
}
fn task_arguments(task:&Task,root:&Path,tool_free:bool)->Result<Vec<String>,String>{
    std::fs::create_dir_all(root).map_err(|e|e.to_string())?;
    let exe=std::env::current_exe().map_err(|e|e.to_string())?;
    let shell_path=|p:&Path|->Result<String,String>{let text=p.to_string_lossy();let text=text.strip_prefix(r"\\?\").unwrap_or(&text).replace('\\',"/");if text.chars().any(|c|matches!(c,'"'|'$'|'`'|'\n'|'\r')){return Err("Caminho incompatível com hooks do CLI.".into());}Ok(text)};
    let hook=root.join("task-hooks.ps1");
    std::fs::write(&hook,include_str!("../scripts/task-hooks.ps1")).map_err(|e|e.to_string())?;
    let log=if tool_free{format!(" -LogPath \"{}\"",shell_path(&root.join("task-hooks.log"))?)}else{String::new()};
    let command=format!("powershell -NoProfile -ExecutionPolicy Bypass -File \"{}\" -CapyExe \"{}\"{}",shell_path(&hook)?,shell_path(&exe)?,log);
    let mut hooks=serde_json::Map::new();
    for event in ["SessionStart","InstructionsLoaded","UserPromptSubmit","PreToolUse","PermissionRequest","PermissionDenied","PostToolUse","PostToolUseFailure","PostToolBatch","Notification","Stop","StopFailure","SessionEnd"]{
        // Preserve the short activity/permission deadline. Only asynchronous
        // instruction and turn-boundary receipts need the wider collector window.
        let timeout=if matches!(event,"InstructionsLoaded"|"Notification"|"StopFailure"){10}else{2};
        hooks.insert(event.into(),serde_json::json!([{"hooks":[{"type":"command","command":command,"timeout":timeout}]}]));
    }
    let settings=root.join("task-settings.json");settings::write_json(&settings,&serde_json::json!({"hooks":hooks}))?;
    let mut args=vec!["--session-id".into(),task.id.clone(),"--model".into(),task.model.clone(),"--settings".into(),settings.to_string_lossy().into_owned()];
    if tool_free{args.extend(["--tools".into(),String::new(),"--strict-mcp-config".into(),"--mcp-config".into(),r#"{"mcpServers":{}}"#.into()]);}
    let instruction=if task.prompt.len()>8_192 {
        let folder=root.join("task-prompts").join(&task.id);
        std::fs::create_dir_all(&folder).map_err(|e|e.to_string())?;
        let path=folder.join("instruction.md");std::fs::write(&path,&task.prompt).map_err(|e|e.to_string())?;
        args.extend(["--add-dir".into(),folder.to_string_lossy().into_owned()]);
        format!("Leia e execute a instrução do usuário neste arquivo: @{}",path.to_string_lossy())
    }else{task.prompt.clone()};
    args.extend(["--".into(),instruction]);Ok(args)
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn task_history_preserves_multiple_long_prompts_after_restart(){
        let root=std::env::temp_dir().join(format!("capy-task-history-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&root).unwrap();
        let store=Store::load(root.clone());let mut ids=Vec::new();
        for index in 0..8{
            let task=Task{id:uuid::Uuid::new_v4().to_string(),profile_id:"p".into(),account:Some("a".into()),billing:"subscription".into(),model:"sonnet".into(),cwd:root.clone(),prompt:format!("Instruction {index}: {}", "\u{1}".repeat(200_000)),mode:"external".into(),created_at:index};
            ids.push(task.id.clone());store.remember(task).unwrap();
        }
        let restarted=Store::load(root.clone());assert_eq!(restarted.list().unwrap().len(),8);
        for (index,id) in ids.iter().enumerate(){assert_eq!(restarted.get(id).unwrap().prompt,format!("Instruction {index}: {}","\u{1}".repeat(200_000)));}
        assert!(std::fs::metadata(root.join("task-index.json")).unwrap().len()<1_048_576);
        assert!(std::fs::metadata(root.join("tasks").join(format!("{}.json",ids[0]))).unwrap().len()>1_048_576);
        store.forget(&ids[0]).unwrap();assert!(Store::load(root.clone()).get(&ids[0]).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn task_history_corruption_blocks_new_records_and_preserves_file(){
        let root=std::env::temp_dir().join(format!("capy-task-corrupt-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&root).unwrap();
        let path=root.join("task-index.json");std::fs::write(&path,"invalid index").unwrap();
        let store=Store::load(root.clone());assert!(store.list().is_err());
        let task=Task{id:uuid::Uuid::new_v4().to_string(),profile_id:"p".into(),account:None,billing:"subscription".into(),model:"sonnet".into(),cwd:root.clone(),prompt:"Build".into(),mode:"external".into(),created_at:0};
        assert!(store.remember(task).is_err());assert_eq!(std::fs::read_to_string(path).unwrap(),"invalid index");
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn task_launch_arguments_keep_prompt_as_data_and_exact_session(){
        let root=std::env::temp_dir().join(format!("capy-task-test-{}",std::process::id()));std::fs::create_dir_all(&root).unwrap();
        let task=Task{id:uuid::Uuid::new_v4().to_string(),profile_id:"p".into(),account:Some("a".into()),billing:"subscription".into(),model:"sonnet".into(),cwd:root.clone(),prompt:"$(Get-Content secret) `hello`; & echo x\nnext".into(),mode:"external".into(),created_at:0};
        let args=task_arguments(&task,&root,false).unwrap();assert_eq!(args[1],task.id);assert_eq!(args.last(),Some(&task.prompt));
        assert!(!args.iter().any(|a|a=="--continue"||a=="--dangerously-skip-permissions"));
        for mode in ["external","embedded"]{
            let request=Start{profile_id:"p".into(),model:"sonnet".into(),cwd:root.clone(),prompt:"Build".into(),mode:mode.into(),expected_account:Some("a".into()),expected_billing:"subscription".into()};assert!(validate(&request).is_ok());
        }
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn task_visual_proof_disables_builtin_and_mcp_tools(){
        let root=std::env::temp_dir().join(format!("capy-task-tool-free-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&root).unwrap();
        let task=Task{id:uuid::Uuid::new_v4().to_string(),profile_id:"p".into(),account:Some("a".into()),billing:"subscription".into(),model:"haiku".into(),cwd:root.clone(),prompt:"Print a fixed marker".into(),mode:"embedded".into(),created_at:0};
        let args=task_arguments(&task,&root,true).unwrap();
        assert!(args.windows(2).any(|items|items==["--tools",""]));
        assert!(args.windows(2).any(|items|items==["--mcp-config",r#"{"mcpServers":{}}"#]));
        assert!(args.iter().any(|item|item=="--strict-mcp-config"));
        let settings:serde_json::Value=crate::settings::read_json(Path::new(args[5].as_str())).unwrap();
        let stop=settings["hooks"]["Stop"][0]["hooks"][0].clone();
        let notification=settings["hooks"]["Notification"][0]["hooks"][0].clone();
        let instructions=settings["hooks"]["InstructionsLoaded"][0]["hooks"][0].clone();
        let prompt=settings["hooks"]["UserPromptSubmit"][0]["hooks"][0].clone();
        assert_eq!(stop["timeout"],2);assert_eq!(prompt["timeout"],2);
        assert_eq!(notification["timeout"],10);assert_eq!(instructions["timeout"],10);
        assert!(notification["command"].as_str().is_some_and(|command|command.contains("task-hooks.ps1")&&command.contains("-LogPath")));
        assert_eq!(args.last(),Some(&task.prompt));std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn task_long_handoff_uses_file_without_windows_commandline_overflow(){
        let root=std::env::temp_dir().join(format!("capy-long-task-test-{}",std::process::id()));std::fs::create_dir_all(&root).unwrap();
        let task=Task{id:uuid::Uuid::new_v4().to_string(),profile_id:"p".into(),account:None,billing:"subscription".into(),model:"sonnet".into(),cwd:root.clone(),prompt:"ç $(Get-Content secret)\n".repeat(5000),mode:"external".into(),created_at:0};
        let args=task_arguments(&task,&root,false).unwrap();assert!(args.join(" ").encode_utf16().count()<16_000);
        assert_eq!(std::fs::read_to_string(root.join("task-prompts").join(&task.id).join("instruction.md")).unwrap(),task.prompt);
        assert!(args.contains(&"--add-dir".into()));std::fs::remove_dir_all(root).unwrap();
    }
}
