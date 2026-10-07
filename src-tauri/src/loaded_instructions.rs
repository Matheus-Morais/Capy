use crate::{handoff_context::excerpt,settings};
use serde::{Deserialize,Serialize};
use std::{fs::{File,OpenOptions},io::Read,path::{Path,PathBuf},time::Duration};

const STORE_LIMIT:u64=8*1024*1024;
const TEXT_LIMIT:usize=8_000;
const GUIDE_LIMIT:usize=64;

#[derive(Clone,Deserialize,Serialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct Instruction {
    pub file_path:String,pub memory_type:String,pub load_reason:String,pub parent_file_path:Option<String>,
}
#[derive(Deserialize,Serialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct Guide {pub instruction:Instruction,pub text:Option<String>,pub observed_at:u64,pub partial:bool}
#[derive(Deserialize,Serialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct Evidence {version:u8,session_id:String,cwd:PathBuf,pub guides:Vec<Guide>,pub partial:bool}

fn path(config:&Path,session:&str)->PathBuf{config.join("capy-guides").join(format!("{session}.json"))}
pub fn status(config:&Path,session:&str,stage:&str)->Result<(),String>{
    if !crate::discovery::uuid(session){return Err("Sessão inválida.".into());}
    let dir=config.join("capy-guides");
    std::fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
    if std::fs::symlink_metadata(&dir).map_err(|e|e.to_string())?.file_type().is_symlink(){return Err("Diretório de referências incompatível.".into());}
    settings::write_json(&dir.join(format!("{session}.status.json")),&serde_json::json!({"version":1,"sessionId":session,"stage":stage}))
}
fn same_folder(a:&Path,b:&Path)->bool{a.canonicalize().ok().zip(b.canonicalize().ok()).is_some_and(|(a,b)|a==b)}
fn valid(instruction:&Instruction)->bool{
    Path::new(&instruction.file_path).is_absolute()&&instruction.file_path.len()<=32_768
        &&matches!(instruction.memory_type.as_str(),"User"|"Project"|"Local"|"Managed")
        &&matches!(instruction.load_reason.as_str(),"session_start"|"nested_traversal"|"path_glob_match"|"include"|"compact")
        &&instruction.parent_file_path.as_ref().is_none_or(|p|Path::new(p).is_absolute()&&p.len()<=32_768)
}
fn validate(evidence:&Evidence,session:&str,cwd:&Path)->Result<(),String>{
    if evidence.version!=1||evidence.session_id!=session||!same_folder(&evidence.cwd,cwd)
        ||evidence.guides.len()>GUIDE_LIMIT||evidence.guides.iter().any(|g|!valid(&g.instruction)||g.text.as_ref().is_some_and(|t|t.len()>TEXT_LIMIT+256)){
        return Err("Referências de instruções incompatíveis; registro preservado.".into());
    }Ok(())
}
fn read_bytes(file:impl Read)->Result<Vec<u8>,String>{
    let mut bytes=Vec::new();file.take(STORE_LIMIT+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
    if bytes.len() as u64>STORE_LIMIT{return Err("Referências de instruções excedem 8 MiB.".into());}
    Ok(bytes)
}
fn stored_bytes(path:&Path)->Result<Option<Vec<u8>>,String>{
    match File::open(path){Ok(file)=>read_bytes(file).map(Some),Err(e) if e.kind()==std::io::ErrorKind::NotFound=>Ok(None),Err(e)=>Err(e.to_string())}
}
fn mutex(path:&Path)->Result<File,String>{
    let lock=path.with_extension("lock");
    if lock.exists()&&!std::fs::symlink_metadata(&lock).is_ok_and(|m|m.file_type().is_file()){return Err("Mutex de referências incompatível.".into());}
    OpenOptions::new().create(true).truncate(false).read(true).write(true).open(lock).map_err(|e|e.to_string())
}
pub fn record(config:&Path,session:&str,cwd:&Path,instruction:Instruction,at:u64)->Result<(),String>{
    if !crate::discovery::uuid(session)||!valid(&instruction){return Err("Evento de instruções incompatível.".into());}
    let snapshot=(||->Result<(String,bool),()>{
        let mut bytes=Vec::new();File::open(&instruction.file_path).map_err(|_|())?.take(TEXT_LIMIT as u64+4).read_to_end(&mut bytes).map_err(|_|())?;
        let truncated=bytes.len()>TEXT_LIMIT;
        if let Err(error)=std::str::from_utf8(&bytes){if error.error_len().is_some(){return Err(());}bytes.truncate(error.valid_up_to());}
        Ok((String::from_utf8(bytes).map_err(|_|())?,truncated))
    })().ok();
    let partial=snapshot.as_ref().is_none_or(|(_,truncated)|*truncated);
    let text=snapshot.map(|(text,_)|text);
    let guide=Guide{instruction,text:text.map(|t|excerpt(&t,TEXT_LIMIT)),observed_at:at,partial};
    let file_path=path(config,session);let parent=file_path.parent().unwrap();
    std::fs::create_dir_all(parent).map_err(|e|e.to_string())?;
    if std::fs::symlink_metadata(parent).map_err(|e|e.to_string())?.file_type().is_symlink()
        ||file_path.exists()&&!std::fs::symlink_metadata(&file_path).is_ok_and(|m|m.file_type().is_file()){
        return Err("Diretório de referências incompatível.".into());
    }
    let file=mutex(&file_path)?;
    let mut locked=false;for _ in 0..40{if file.try_lock().is_ok(){locked=true;break;}std::thread::sleep(Duration::from_millis(10));}
    if !locked{return Err("Referências ocupadas; carga não registrada.".into());}
    let original=stored_bytes(&file_path)?;
    let mut evidence=match &original{
        None=>Evidence{version:1,session_id:session.into(),cwd:cwd.into(),guides:Vec::new(),partial:false},
        Some(bytes)=>serde_json::from_slice(bytes).map_err(|_|"Referências de instruções inválidas; registro preservado.")?,
    };
    validate(&evidence,session,cwd)?;
    evidence.partial|=partial;
    if let Some(old)=evidence.guides.iter_mut().find(|g|g.instruction.file_path==guide.instruction.file_path){
        if old.observed_at<=at{*old=guide;}
    }else if evidence.guides.len()<GUIDE_LIMIT{evidence.guides.push(guide);}else{evidence.partial=true;}
    if stored_bytes(&file_path)?!=original{return Err("Referências alteradas fora da Capy; registro preservado.".into());}
    settings::write_json_limit(&file_path,&evidence,STORE_LIMIT)
}
pub fn load(config:&Path,session:&str,cwd:&Path)->Result<Option<Evidence>,String>{
    if !crate::discovery::uuid(session){return Err("Sessão inválida para as referências.".into());}
    let file_path=path(config,session);
    let metadata=match std::fs::symlink_metadata(&file_path){Ok(metadata)=>metadata,Err(e) if e.kind()==std::io::ErrorKind::NotFound=>return Ok(None),Err(e)=>return Err(e.to_string())};
    if !metadata.file_type().is_file(){return Err("Registro de referências incompatível.".into());}
    let file=mutex(&file_path)?;
    file.try_lock_shared().map_err(|_|"Referências ocupadas; confira antes de transferir.")?;
    let bytes=stored_bytes(&file_path)?.ok_or("Registro de referências mudou durante a leitura.")?;
    let evidence:Evidence=serde_json::from_slice(&bytes).map_err(|_|"Referências de instruções inválidas; registro preservado.")?;validate(&evidence,session,cwd)?;Ok(Some(evidence))
}

#[cfg(test)]
mod tests{
    use super::*;
    const SESSION:&str="11111111-1111-1111-1111-111111111111";
    fn fixture()->PathBuf{let p=std::env::temp_dir().join(format!("capy-loaded-guides-{}",uuid::Uuid::new_v4()));std::fs::create_dir_all(&p).unwrap();p}
    fn instruction(path:&Path)->Instruction{Instruction{file_path:path.to_string_lossy().into(),memory_type:"Project".into(),load_reason:"session_start".into(),parent_file_path:None}}
    fn cleanup(root:&Path){let temp=std::env::temp_dir().canonicalize().unwrap();let own=root.canonicalize().unwrap();assert_eq!(own.parent(),Some(temp.as_path()));assert!(own.file_name().unwrap().to_string_lossy().starts_with("capy-loaded-guides-"));std::fs::remove_dir_all(own).unwrap();}
    #[test]
    fn loaded_instructions_preserve_exact_session_snapshot_and_reject_drift(){
        let root=fixture();let guide=root.join("CLAUDE.md");std::fs::write(&guide,"# Plano\nNão alterar API").unwrap();
        record(&root,SESSION,&root,instruction(&guide),100).unwrap();std::fs::write(&guide,"changed later").unwrap();
        let evidence=load(&root,SESSION,&root).unwrap().unwrap();assert_eq!(evidence.guides[0].text.as_deref(),Some("# Plano\nNão alterar API"));
        let other=root.join("other");std::fs::create_dir(&other).unwrap();assert!(load(&root,SESSION,&other).is_err());
        assert!(record(&root,"../other",&root,instruction(&guide),200).is_err());cleanup(&root);
    }
    #[test]
    fn loaded_instructions_concurrent_events_keep_all_references(){
        let root=fixture();let mut threads=Vec::new();
        for i in 0..12{let own=root.clone();let guide=root.join(format!("rule-{i}.md"));std::fs::write(&guide,format!("Rule {i}")).unwrap();threads.push(std::thread::spawn(move||record(&own,SESSION,&own,instruction(&guide),100+i).unwrap()));}
        for thread in threads{thread.join().unwrap();}assert_eq!(load(&root,SESSION,&root).unwrap().unwrap().guides.len(),12);cleanup(&root);
    }
    #[test]
    fn loaded_instructions_invalid_store_is_preserved(){
        let root=fixture();std::fs::create_dir_all(root.join("capy-guides")).unwrap();let stored=path(&root,SESSION);
        let guide=root.join("CLAUDE.md");std::fs::write(&guide,"rule").unwrap();
        for bytes in [b"".as_slice(),b"{\"futureVersion\":2}".as_slice()]{std::fs::write(&stored,bytes).unwrap();assert!(record(&root,SESSION,&root,instruction(&guide),100).is_err());assert_eq!(std::fs::read(&stored).unwrap(),bytes);}cleanup(&root);
    }
    #[test]
    fn loaded_instructions_keep_utf8_excerpt_and_report_missing_files_and_capacity(){
        let root=fixture();let guide=root.join("CLAUDE.md");std::fs::write(&guide,"🦫".repeat(3000)).unwrap();
        record(&root,SESSION,&root,instruction(&guide),100).unwrap();
        let evidence=load(&root,SESSION,&root).unwrap().unwrap();assert!(evidence.partial);assert!(evidence.guides[0].text.as_deref().unwrap().starts_with("🦫🦫"));
        let missing=root.join("missing.md");record(&root,SESSION,&root,instruction(&missing),101).unwrap();
        for i in 0..64{record(&root,SESSION,&root,instruction(&root.join(format!("missing-{i}.md"))),102+i).unwrap();}
        let evidence=load(&root,SESSION,&root).unwrap().unwrap();assert!(evidence.partial);assert_eq!(evidence.guides.len(),64);assert!(evidence.guides.iter().any(|g|g.instruction.file_path==missing.to_string_lossy()&&g.text.is_none()));cleanup(&root);
    }
}
