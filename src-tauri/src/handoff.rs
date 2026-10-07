use crate::{claude_activity::{self,Boundary}, profiles::{self,Profile}, settings, tasks::{self,Task}};
use serde::{Deserialize,Serialize};
use crate::handoff_context::{self, excerpt};
use std::{collections::BTreeSet,path::PathBuf,sync::Mutex};

#[derive(Clone,Deserialize,Serialize)]
#[serde(rename_all="camelCase")]
pub struct Summary {
    pub objective:String,pub decisions:String,pub state:String,pub files:String,
    pub tests:String,pub next_steps:String,pub guides:String,
}
impl Summary {
    pub fn validate(&self)->Result<(),String>{
        if [&self.objective,&self.decisions,&self.state,&self.files,&self.tests,&self.next_steps,&self.guides].into_iter().any(|s|s.trim().is_empty()||s.len()>65_536||s.contains('\0')) {return Err("Revise todos os campos do resumo; cada um deve ter entre 1 e 65536 bytes.".into());}
        if self.prompt().len()>262_144 {return Err("O resumo integral excede 256 KiB. Encurte os trechos e preserve referências ao histórico original.".into());}Ok(())
    }
    pub fn prompt(&self)->String{
        format!("Continue o trabalho a partir deste resumo revisado pelo usuário. Confira o estado atual dos arquivos antes de editar.\n\n# Objetivo\n{}\n\n# Decisões e regras\n{}\n\n# Estado atual\n{}\n\n# Arquivos alterados\n{}\n\n# Testes e evidências\n{}\n\n# Próximos passos\n{}\n\n# Planos e arquivos .md usados\n{}",self.objective,self.decisions,self.state,self.files,self.tests,self.next_steps,self.guides)
    }
}
#[derive(Clone,Deserialize,Serialize)]
#[serde(rename_all="camelCase")]
pub struct Review {
    pub nonce:String,pub source_task_id:String,pub destination_profile_id:String,pub model:String,
    pub source_billing:String,pub destination_billing:String,pub destination_account:Option<String>,
    pub summary:Summary,pub boundary:Boundary,pub automatic:bool,
}
#[derive(Default,Deserialize,Serialize,Clone)]
struct State {reviews:Vec<Review>,dismissed:BTreeSet<String>,completed:BTreeSet<String>}
const STATE_LIMIT:u64=134_217_728;
pub struct Store {path:PathBuf,state:Mutex<State>,load_error:Option<String>}
impl Store {
    pub fn load(path:PathBuf)->Self{
        if !path.exists(){return Self{path,state:Mutex::new(State::default()),load_error:None};}
        match settings::read_json_limit::<State>(&path,STATE_LIMIT){
            Ok(state) if state.reviews.len()<=64=>Self{path,state:Mutex::new(state),load_error:None},
            _=>Self{path,state:Mutex::new(State::default()),load_error:Some("O estado das transferências não pôde ser lido. O arquivo foi preservado; recupere-o antes de continuar.".into())},
        }
    }
    fn ensure_loaded(&self)->Result<(),String>{if let Some(error)=&self.load_error{Err(error.clone())}else{Ok(())}}
    fn persist(&self,state:&State)->Result<(),String>{self.ensure_loaded()?;settings::write_json_limit(&self.path,state,STATE_LIMIT)}
    pub fn list(&self)->Result<Vec<Review>,String>{self.ensure_loaded()?;self.state.lock().map(|v|v.reviews.clone()).map_err(|_|"Transferências indisponíveis.".into())}
    pub fn should_prepare(&self,task:&Task,boundary:&Boundary)->bool{
        self.load_error.is_none()&&self.state.lock().is_ok_and(|state|!state.completed.contains(&task.id)&&!state.reviews.iter().any(|r|r.source_task_id==task.id)&&!state.dismissed.contains(&format!("{}:{}",task.id,boundary.at_ms)))
    }
    pub fn eligible(&self,task:&Task)->bool {self.load_error.is_none()&&self.state.lock().is_ok_and(|state|!state.completed.contains(&task.id)&&!state.reviews.iter().any(|r|r.source_task_id==task.id))}
    pub fn prune(&self,tasks:&[Task])->Result<(),String>{
        self.ensure_loaded()?;
        let retained=tasks.iter().map(|t|t.id.as_str()).collect::<BTreeSet<_>>();
        let mut state=self.state.lock().map_err(|_|"Transferências indisponíveis.")?;
        let mut updated=state.clone();
        updated.completed.retain(|id|retained.contains(id.as_str()));
        updated.dismissed.retain(|key|key.split_once(':').is_some_and(|(id,_)|retained.contains(id)));
        updated.reviews.retain(|review|retained.contains(review.source_task_id.as_str()));
        if updated.completed.len()!=state.completed.len()||updated.dismissed.len()!=state.dismissed.len()||updated.reviews.len()!=state.reviews.len(){
            self.persist(&updated)?;*state=updated;
        }Ok(())
    }
    pub fn prepare(&self,source:&Task,source_profile:&Profile,destination:&Profile,model:String,automatic:bool)->Result<Review,String>{
        self.ensure_loaded()?;
        if !settings::valid_text(&model,128)||model.starts_with('-')||model.contains(char::is_whitespace){return Err("Modelo de destino inválido.".into());}
        let cwd=source.cwd.to_string_lossy();
        let boundary=claude_activity::turn_boundary(&source_profile.config_dir,&source.id,&cwd,crate::quotas::now_ms()).ok_or("Aguarde o fim do turno atual; a Capy precisa de um evento recente do Claude para preparar a continuação.")?;
        let identity=profiles::identity(destination,Some(&source.cwd))?;
        if !identity.logged_in{return Err("Faça login oficial na conta de destino antes de preparar a continuação.".into());}
        let review=Review{nonce:uuid::Uuid::new_v4().to_string(),source_task_id:source.id.clone(),destination_profile_id:destination.id.clone(),model,source_billing:source.billing.clone(),destination_billing:identity.billing,destination_account:identity.account,summary:build_summary(source,source_profile)?,boundary,automatic};
        let mut state=self.state.lock().map_err(|_|"Transferências indisponíveis.")?;
        if let Some(existing)=state.reviews.iter().find(|r|r.source_task_id==source.id){
            if existing.destination_profile_id!=review.destination_profile_id||existing.model!=review.model{return Err("Já existe uma revisão com outro destino. Cancele a anterior para mudar a conta/modelo.".into());}
            return Ok(existing.clone());
        }
        if state.completed.contains(&source.id){return Err("Esta tarefa já foi transferida. Continue a partir da nova sessão.".into());}
        if state.reviews.len()>=64{return Err("Há 64 revisões pendentes. Resolva uma antes de preparar outra transferência.".into());}
        let mut updated=state.clone();updated.reviews.push(review.clone());self.persist(&updated)?;*state=updated;Ok(review)
    }
    pub fn cancel(&self,nonce:&str)->Result<(),String>{
        self.ensure_loaded()?;
        let mut state=self.state.lock().map_err(|_|"Transferências indisponíveis.")?;
        let mut updated=state.clone();
        for review in updated.reviews.iter().filter(|r|r.nonce==nonce){
            updated.dismissed.retain(|key|!key.starts_with(&format!("{}:",review.source_task_id)));
            updated.dismissed.insert(format!("{}:{}",review.source_task_id,review.boundary.at_ms));
        }
        updated.reviews.retain(|r|r.nonce!=nonce);self.persist(&updated)?;*state=updated;Ok(())
    }
    pub fn restore_after_failure(&self,mut review:Review)->Result<(),String>{
        self.ensure_loaded()?;
        let mut state=self.state.lock().map_err(|_|"Transferências indisponíveis.")?;let mut updated=state.clone();
        updated.completed.remove(&review.source_task_id);review.nonce=uuid::Uuid::new_v4().to_string();updated.reviews.push(review);
        self.persist(&updated)?;*state=updated;Ok(())
    }
    pub fn approve(&self,nonce:&str,summary:Summary,billing_confirmed:bool,source:&Task,source_profile:&Profile,destination:&Profile,mode:String)->Result<tasks::Start,String>{
        summary.validate()?;
        let identity=profiles::identity(destination,Some(&source.cwd))?;
        self.approve_verified(nonce,summary,billing_confirmed,source,destination,mode,&identity,|boundary|{
            claude_activity::boundary_unchanged(&source_profile.config_dir,&source.id,&source.cwd.to_string_lossy(),boundary)
        })
    }
    fn approve_verified(&self,nonce:&str,summary:Summary,billing_confirmed:bool,source:&Task,destination:&Profile,mode:String,identity:&profiles::Identity,boundary_valid:impl FnOnce(&Boundary)->bool)->Result<tasks::Start,String>{
        self.ensure_loaded()?;
        summary.validate()?;
        let mut state=self.state.lock().map_err(|_|"Transferências indisponíveis.")?;
        let index=state.reviews.iter().position(|r|r.nonce==nonce).ok_or("Esta revisão expirou ou já foi usada.")?;
        let review=&state.reviews[index];
        if review.source_task_id!=source.id||review.destination_profile_id!=destination.id||review.source_billing!=source.billing{return Err("A origem ou destino não corresponde à revisão.".into());}
        if !identity.logged_in||identity.billing!=review.destination_billing||identity.account!=review.destination_account{return Err("A conta/cobrança de destino mudou. Prepare uma nova revisão.".into());}
        if review.source_billing!=review.destination_billing&&!billing_confirmed{return Err("Confirme a mudança de cobrança desta transferência.".into());}
        let request=tasks::Start{profile_id:destination.id.clone(),model:review.model.clone(),cwd:source.cwd.clone(),prompt:summary.prompt(),mode,expected_account:identity.account.clone(),expected_billing:identity.billing.clone()};
        tasks::validate(&request)?;
        if !boundary_valid(&review.boundary){return Err("A sessão voltou a trabalhar ou mudou. Gere um novo resumo ao fim do turno.".into());}
        let mut updated=state.clone();updated.reviews.remove(index);updated.completed.insert(source.id.clone());self.persist(&updated)?;*state=updated;
        Ok(request)
    }
}
fn transcript(profile:&Profile,id:&str)->Option<PathBuf>{
    if !crate::discovery::uuid(id){return None;}
    std::fs::read_dir(profile.config_dir.join("projects")).ok()?.take(2048).filter_map(Result::ok).filter(|e|e.file_type().is_ok_and(|t|t.is_dir())).map(|e|e.path().join(format!("{id}.jsonl"))).find(|p|p.is_file())
}
pub fn build_summary(task:&Task,profile:&Profile)->Result<Summary,String>{
    let path=transcript(profile,&task.id).ok_or("O histórico da sessão exata não foi encontrado. Abra a conversa original antes de tentar preparar a continuação.")?;
    let mut context=handoff_context::load(&path)?;
    let automatic=match crate::loaded_instructions::load(&profile.config_dir,&task.id,&task.cwd){
        Ok(Some(evidence))=>{
            context.partial|=evidence.partial;
            let references=evidence.guides.iter().map(|g|format!("- {} · {} · {}{}",g.instruction.file_path,g.instruction.memory_type,g.instruction.load_reason,g.instruction.parent_file_path.as_ref().map(|p|format!(" · importado por {p}")).unwrap_or_default())).collect::<Vec<_>>().join("\n");
            let excerpts=evidence.guides.iter().map(|g|format!("Arquivo: {}\nConsulta no hook em {}ms; conteúdo não é cópia garantida do contexto interno do CLI:\n{}",g.instruction.file_path,g.observed_at,g.text.as_deref().unwrap_or("Conteúdo não recuperado; confira o arquivo e a sessão original."))).collect::<Vec<_>>().join("\n\n");
            format!("Referências automáticas observadas por InstructionsLoaded:\n{references}\n\nTrechos consultados no hook:\n{excerpts}\n\nO registro contém somente eventos observados; confira referências ausentes antes de aprovar.")
        },
        Ok(None)=>{context.partial=true;"Sem registro InstructionsLoaded desta sessão; complete as referências carregadas automaticamente antes de aprovar.".into()},
        Err(error)=>{context.partial=true;format!("Referências automáticas indisponíveis: {error} Confira a origem antes de aprovar.")},
    };
    if context.answers.is_empty() && context.instructions.is_empty() {
        return Err("O histórico não contém mensagens recuperáveis. Não foi possível preparar um resumo comprovável.".into());
    }
    let history=format!("Histórico original da sessão: {}",path.display());
    let warning=if context.partial {
        "\nAVISO: contexto parcial (histórico ou instruções) ou trechos limitados. Confira a origem e complete o resumo antes de aprovar."
    }else{""};
    let instructions=context.instructions.iter().rev().take(8).rev().cloned().collect::<Vec<_>>().join("\n\n");
    let answers=context.answers.iter().rev().take(8).rev().cloned().collect::<Vec<_>>().join("\n\n");
    let decisions=excerpt(&format!("Instruções registradas do usuário:\n{instructions}\n\nDecisões e relatos recentes do agente, em ordem; confira quais permanecem válidos:\n{answers}\n\n{history}{warning}"),48_000);
    let last=context.answers.last().map(String::as_str).unwrap_or("Nenhuma resposta textual do agente registrada.");
    let state=format!("Projeto: {}\nConta de origem: {}\nModelo: {}\nSessão: {}\nTurno encerrado por evento Claude; isso não significa tarefa concluída.\n\nÚltimo relato do agente:\n{}\n\n{history}{warning}",
        task.cwd.display(),task.account.as_deref().unwrap_or("perfil selecionado"),task.model,task.id,excerpt(last,8_000));

    let mut git=std::process::Command::new("git");
    git.current_dir(&task.cwd).args(["status","--short","--untracked-files=normal"]);
    #[cfg(windows)] {use std::os::windows::process::CommandExt;git.creation_flags(0x08000000);}
    let git_state=match profiles::output(git,std::time::Duration::from_secs(15)){
        Ok(bytes) if bytes.is_empty()=>"O Git informou uma árvore de trabalho limpa.".into(),
        Ok(bytes)=>format!("Estado atual do projeto pelo Git; não atribui alterações exclusivamente a esta sessão:\n{}",String::from_utf8_lossy(&bytes)),
        Err(_)=>"O Git não confirmou o estado deste projeto. Confira os arquivos antes de continuar.".into(),
    };
    let edits=context.edits.iter().cloned().collect::<Vec<_>>().join("\n");
    let files=excerpt(&format!("{git_state}\n\nOperações de alteração solicitadas no histórico; podem incluir tentativas que falharam:\n{}\n\n{history}{warning}",
        if edits.is_empty(){"Nenhuma operação Edit/Write/NotebookEdit identificada."}else{&edits}),24_000);

    let tests=context.commands.iter().filter(|c|handoff_context::test_command(&c.command)).rev().take(24).collect::<Vec<_>>();
    let evidence=tests.iter().rev().map(|c|format!("Comando: {}\nResultado recebido: {}\nSaída:\n{}",c.command,
        match c.failed{Some(true)=>"a ferramenta informou erro",Some(false)=>"a ferramenta não marcou erro; confira contagens e código de saída",None=>"não observado"},
        c.result.as_deref().unwrap_or("Não há saída correlacionada. Não declare sucesso deste teste."))).collect::<Vec<_>>().join("\n\n");
    let tests_text=excerpt(&format!("{}\n\n{history}{warning}",if evidence.is_empty(){"Nenhum teste identificado no histórico; aprovação de testes não comprovada."}else{&evidence}),40_000);

    let pending=next_steps(&context);
    let guides=context.guides.iter().map(|(path,read)|format!("Arquivo: {path}\n{}",
        if read.is_empty(){"Leitura solicitada, mas seu conteúdo/resultados não foram observados."}else{read})).collect::<Vec<_>>().join("\n\n");
    let guides=excerpt(&format!("{automatic}\n\nLeituras explícitas de .md no histórico:\n{}\n\nArquivos carregados automaticamente pelo CLI não são presumidos como lidos: complete aqui suas referências/regras caso o histórico não as registre.\n\n{history}{warning}",
        if guides.is_empty(){"Nenhuma leitura explícita de .md identificada."}else{&guides}),40_000);
    let objective=excerpt(&format!("Instrução inicial:\n{}\n\nInstruções posteriores registradas:\n{instructions}\n\n{history}{warning}",task.prompt),40_000);
    let summary=Summary{objective,decisions,state,files,tests:tests_text,next_steps:excerpt(&format!("{pending}\n\n{history}{warning}"),24_000),guides};
    summary.validate()?;
    Ok(summary)
}

fn next_steps(context:&handoff_context::Context)->String {
    let mut steps=Vec::new();
    let failed=context.commands.iter().filter(|c|handoff_context::test_command(&c.command)&&c.failed==Some(true)).rev().take(8).collect::<Vec<_>>();
    for command in failed.iter().rev() {
        steps.push(format!("Teste com erro observado: {}\nInvestigue a saída registrada antes de repetir ou declarar aprovação.",command.command));
    }
    if let Some(last)=context.answers.last() {
        let mut capture=false;
        let mut sections=Vec::new();
        for line in last.lines(){
            let lower=line.to_lowercase();
            let marker=["próximo","proximo","next step","next:","pendente","ainda falta","resta ","todo","to do"].iter().any(|word|lower.contains(word));
            if line.starts_with('#'){capture=marker;}
            if capture||marker {sections.push(line);}
        }
        if !sections.is_empty(){steps.push(format!("Próximos passos/pendências declarados no último relato:\n{}",sections.join("\n")));}
        else {steps.push(format!("O último relato não enumera próximos passos. Base registrada para a revisão:\n{}",excerpt(last,8_000)));}
    }
    if let Some(instruction)=context.instructions.last(){steps.push(format!("Última instrução registrada do usuário:\n{}",excerpt(instruction,8_000)));}
    steps.join("\n\n")
}

#[cfg(test)]
mod tests{
    use super::*;
    struct Fixture {root:PathBuf,store:Store,task:Task,destination:Profile,identity:profiles::Identity,review:Review}
    impl Fixture {
        fn new()->Self{
            let root=std::env::temp_dir().join(format!("capy-handoff-{}",uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&root).unwrap();
            let summary=Summary{objective:"Build parser".into(),decisions:"Keep API".into(),state:"Parser ready".into(),files:"parser.rs".into(),tests:"2 passed; 1 failed".into(),next_steps:"Fix timeout".into(),guides:"plan.md § API: keep compatibility".into()};
            let task=Task{id:uuid::Uuid::new_v4().to_string(),profile_id:"source".into(),account:Some("source@example.test".into()),billing:"subscription".into(),model:"sonnet".into(),cwd:root.clone(),prompt:"Build parser".into(),mode:"external".into(),created_at:0};
            let destination=Profile{id:"destination".into(),label:"Destination".into(),provider:"Claude".into(),config_dir:root.clone(),billing:"api".into()};
            let identity=profiles::Identity{logged_in:true,account:Some("destination@example.test".into()),billing:"api".into(),message:"Fixture".into()};
            let review=Review{nonce:uuid::Uuid::new_v4().to_string(),source_task_id:task.id.clone(),destination_profile_id:destination.id.clone(),model:"opus".into(),source_billing:"subscription".into(),destination_billing:identity.billing.clone(),destination_account:identity.account.clone(),summary,boundary:Boundary{at_ms:1_000,pid:42,proc_start:"100".into()},automatic:true};
            settings::write_json(&root.join("handoffs.json"),&State{reviews:vec![review.clone()],..State::default()}).unwrap();
            let store=Store::load(root.join("handoffs.json"));Self{root,store,task,destination,identity,review}
        }
        fn approve(&self,billing:bool,identity:&profiles::Identity,boundary:bool)->Result<tasks::Start,String>{
            self.store.approve_verified(&self.review.nonce,self.review.summary.clone(),billing,&self.task,&self.destination,"external".into(),identity,|_|boundary)
        }
    }
    impl Drop for Fixture {fn drop(&mut self){let _=std::fs::remove_dir_all(&self.root);}}

    #[test]
    fn handoff_rejects_billing_identity_and_turn_drift_without_consuming_review(){
        let f=Fixture::new();
        assert!(f.approve(false,&f.identity,true).err().unwrap().contains("cobrança"));
        let mut changed=f.identity.clone();changed.account=Some("different@example.test".into());
        assert!(f.approve(true,&changed,true).err().unwrap().contains("mudou"));
        changed=f.identity.clone();changed.logged_in=false;assert!(f.approve(true,&changed,true).is_err());
        changed=f.identity.clone();changed.billing="subscription".into();assert!(f.approve(true,&changed,true).is_err());
        assert!(f.approve(true,&f.identity,false).err().unwrap().contains("voltou a trabalhar"));
        assert_eq!(f.store.list().unwrap().len(),1);
        assert!(!f.store.eligible(&f.task));
    }

    #[test]
    fn handoff_approval_is_single_use_and_persists_across_restart(){
        let f=Fixture::new();
        let request=f.approve(true,&f.identity,true).unwrap();
        assert_eq!(request.profile_id,f.destination.id);assert_eq!(request.expected_account,f.identity.account);
        assert_eq!(request.expected_billing,"api");assert_eq!(request.model,"opus");
        assert!(request.prompt.contains("Fix timeout"));assert!(request.prompt.contains("1 failed"));
        assert!(f.approve(true,&f.identity,true).is_err());
        let restarted=Store::load(f.root.join("handoffs.json"));
        assert!(restarted.list().unwrap().is_empty());assert!(!restarted.eligible(&f.task));
        assert!(restarted.approve_verified(&f.review.nonce,f.review.summary.clone(),true,&f.task,&f.destination,"external".into(),&f.identity,|_|true).is_err());
    }

    #[test]
    fn handoff_failure_restores_edited_summary_with_fresh_required_approval(){
        let f=Fixture::new();f.approve(true,&f.identity,true).unwrap();
        let mut edited=f.review.clone();edited.summary.next_steps="User revised: fix only timeout first".into();
        f.store.restore_after_failure(edited).unwrap();
        let restored=f.store.list().unwrap().pop().unwrap();assert_ne!(restored.nonce,f.review.nonce);
        assert!(restored.summary.next_steps.contains("fix only timeout first"));
        assert!(f.approve(true,&f.identity,true).is_err());
        assert!(f.store.approve_verified(&restored.nonce,restored.summary.clone(),false,&f.task,&f.destination,"external".into(),&f.identity,|_|true).is_err());
        let restarted=Store::load(f.root.join("handoffs.json"));
        assert_eq!(restarted.list().unwrap()[0].nonce,restored.nonce);
        assert!(restarted.approve_verified(&restored.nonce,restored.summary,true,&f.task,&f.destination,"external".into(),&f.identity,|_|true).is_ok());
    }

    #[test]
    fn handoff_cancellation_suppresses_only_the_same_source_boundary(){
        let f=Fixture::new();f.store.cancel(&f.review.nonce).unwrap();
        assert!(!f.store.should_prepare(&f.task,&f.review.boundary));
        let mut next=f.review.boundary.clone();next.at_ms+=1;
        assert!(f.store.should_prepare(&f.task,&next));
        let restarted=Store::load(f.root.join("handoffs.json"));
        assert!(!restarted.should_prepare(&f.task,&f.review.boundary));
        assert!(restarted.should_prepare(&f.task,&next));
    }

    #[test]
    fn handoff_long_reviews_survive_restart_without_resetting_authorization(){
        let f=Fixture::new();let mut state=State::default();
        for _ in 0..8{
            let mut review=f.review.clone();review.nonce=uuid::Uuid::new_v4().to_string();review.source_task_id=uuid::Uuid::new_v4().to_string();
            review.summary.objective="ç".repeat(30_000);review.summary.guides="rule\n".repeat(10_000);review.summary.decisions="Decision\n".repeat(6_000);
            review.summary.validate().unwrap();state.reviews.push(review);
        }
        f.store.persist(&state).unwrap();assert!(std::fs::metadata(&f.store.path).unwrap().len()>1_048_576);
        let restarted=Store::load(f.store.path.clone());assert_eq!(restarted.list().unwrap().len(),8);
        for (expected,actual) in state.reviews.iter().zip(restarted.list().unwrap()){
            assert_eq!(expected.nonce,actual.nonce);assert_eq!(expected.summary.guides,actual.summary.guides);
            assert_eq!(expected.destination_account,actual.destination_account);
        }
    }
    #[test]
    fn handoff_corruption_cannot_reset_single_use_history(){
        let f=Fixture::new();std::fs::write(&f.store.path,"invalid state").unwrap();
        let restarted=Store::load(f.store.path.clone());assert!(restarted.list().is_err());assert!(!restarted.eligible(&f.task));
        assert!(restarted.cancel(&f.review.nonce).is_err());
        assert_eq!(std::fs::read_to_string(&f.store.path).unwrap(),"invalid state");
        std::fs::write(&f.store.path,"{\"reviews\":[],\"dismissed\":[]}").unwrap();
        assert!(Store::load(f.store.path.clone()).list().is_err());
    }
    #[test]
    fn handoff_summary_cites_actual_guides_decisions_commands_and_all_sections(){
        let root=std::env::temp_dir().join(format!("capy-summary-{}",uuid::Uuid::new_v4()));
        let cwd=root.join("project");let config=root.join("config");
        std::fs::create_dir_all(&cwd).unwrap();std::fs::create_dir_all(config.join("projects/fixture")).unwrap();
        let task=Task{id:uuid::Uuid::new_v4().to_string(),profile_id:"p".into(),account:Some("a".into()),billing:"subscription".into(),model:"sonnet".into(),cwd,prompt:"Build a parser".into(),mode:"external".into(),created_at:0};
        let profile=Profile{id:"p".into(),label:"Fixture".into(),provider:"Claude".into(),config_dir:config.clone(),billing:"subscription".into()};
        let rows=[
            serde_json::json!({"type":"user","message":{"content":"Keep the API compatible"}}),
            serde_json::json!({"type":"assistant","message":{"content":[
                {"type":"tool_use","id":"read","name":"Read","input":{"file_path":".design/plan.md"}},
                {"type":"tool_use","id":"test","name":"Bash","input":{"command":"npm test"}},
                {"type":"tool_use","id":"edit","name":"Edit","input":{"file_path":"src/parser.ts"}},
                {"type":"text","text":"Decision: retain the public API.\n# Next steps\nFix the timeout in parser.test.ts, then rerun npm test."}
            ]}}),
            serde_json::json!({"type":"user","message":{"content":[
                {"type":"tool_result","tool_use_id":"test","is_error":true,"content":"FAIL timeout; 2 passed, 1 failed"},
                {"type":"tool_result","tool_use_id":"read","content":"# API contract\nUse a stable public signature."}
            ]}}),
        ];
        let transcript=config.join("projects/fixture").join(format!("{}.jsonl",task.id));
        std::fs::write(&transcript,rows.iter().map(serde_json::Value::to_string).collect::<Vec<_>>().join("\n")).unwrap();
        let summary=build_summary(&task,&profile).unwrap();
        assert!(summary.objective.contains("Keep the API compatible"));
        assert!(summary.decisions.contains("retain the public API"));
        assert!(summary.state.contains(&task.id));assert!(summary.files.contains("src/parser.ts"));
        assert!(summary.tests.contains("1 failed"));assert!(summary.tests.contains("informou erro"));
        assert!(summary.next_steps.contains("Fix the timeout in parser.test.ts"));
        assert!(summary.guides.contains(".design/plan.md"));assert!(summary.guides.contains("# API contract"));
        assert!(summary.guides.contains("stable public signature"));
        for title in ["Objetivo","Decisões e regras","Estado atual","Arquivos alterados","Testes e evidências","Próximos passos","Planos e arquivos .md usados"]{assert!(summary.prompt().contains(title));}
        let mut empty=summary;empty.next_steps.clear();assert!(empty.validate().is_err());
        std::fs::remove_file(&transcript).unwrap();assert!(build_summary(&task,&profile).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn handoff_summary_includes_observed_automatic_instructions(){
        let root=std::env::temp_dir().join(format!("capy-auto-summary-{}",uuid::Uuid::new_v4()));let cwd=root.join("project");let config=root.join("config");
        std::fs::create_dir_all(&cwd).unwrap();std::fs::create_dir_all(config.join("projects/fixture")).unwrap();
        let task=Task{id:uuid::Uuid::new_v4().to_string(),profile_id:"p".into(),account:Some("a".into()),billing:"subscription".into(),model:"haiku".into(),cwd:cwd.clone(),prompt:"Keep the contract".into(),mode:"embedded".into(),created_at:0};
        let profile=Profile{id:"p".into(),label:"Own fixture".into(),provider:"Claude".into(),config_dir:config.clone(),billing:"subscription".into()};
        std::fs::write(config.join("projects/fixture").join(format!("{}.jsonl",task.id)),"{\"type\":\"user\",\"message\":{\"content\":\"Keep the contract\"}}\n{\"type\":\"assistant\",\"message\":{\"content\":\"Next: test it\"}}").unwrap();
        let guide=cwd.join("plan.md");std::fs::write(&guide,"# Contract\nPreserve public fields").unwrap();
        let instruction=crate::loaded_instructions::Instruction{file_path:guide.to_string_lossy().into(),memory_type:"Project".into(),load_reason:"include".into(),parent_file_path:Some(cwd.join("CLAUDE.md").to_string_lossy().into())};
        crate::loaded_instructions::record(&config,&task.id,&cwd,instruction,100).unwrap();std::fs::write(&guide,"changed after load").unwrap();
        let summary=build_summary(&task,&profile).unwrap();assert!(summary.guides.contains("plan.md"));assert!(summary.guides.contains("CLAUDE.md"));assert!(summary.guides.contains("Preserve public fields"));assert!(summary.guides.contains("include"));assert!(!summary.guides.contains("changed after load"));
        assert_eq!(root.parent(),Some(std::env::temp_dir().as_path()));assert!(root.file_name().unwrap().to_string_lossy().starts_with("capy-auto-summary-"));std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn handoff_summary_rejects_combined_prompt_over_task_limit(){
        let text="x".repeat(40_000);
        let summary=Summary{objective:text.clone(),decisions:text.clone(),state:text.clone(),files:text.clone(),tests:text.clone(),next_steps:text.clone(),guides:text};
        assert!(summary.validate().is_err());
    }
}
