use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use std::{collections::{HashMap,VecDeque}, io::{Read,Write}, sync::{Arc,Mutex, atomic::{AtomicBool,Ordering}}};

const BUFFER_LIMIT:usize=524_288;
#[derive(Clone, Serialize)]
#[serde(rename_all="camelCase")]
pub struct Chunk { pub task_id:String, pub sequence:u64, pub data:Vec<u8> }
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct Replay { pub chunks:Vec<Chunk>, pub exited:bool }
#[derive(Default)]
struct History { chunks:VecDeque<Chunk>, bytes:usize, sequence:u64 }
impl History {
    fn append(&mut self,id:&str,data:Vec<u8>)->Chunk{
        self.sequence+=1;
        let chunk=Chunk{task_id:id.into(),sequence:self.sequence,data};
        self.bytes+=chunk.data.len();self.chunks.push_back(chunk.clone());
        while self.bytes>BUFFER_LIMIT {if let Some(old)=self.chunks.pop_front(){self.bytes-=old.data.len();}else{break;}}
        chunk
    }
}
struct Entry {
    master:Mutex<Option<Box<dyn MasterPty+Send>>>,
    writer:Mutex<Option<Box<dyn Write+Send>>>,
    history:Mutex<History>,
    exited:AtomicBool,
}
#[derive(Default)]
pub struct Service { entries:Mutex<HashMap<String,Arc<Entry>>> }
impl Service {
    pub fn start(&self,id:String,command:CommandBuilder,on_output:impl Fn(Chunk)+Send+'static,on_exit:impl Fn()+Send+'static)->Result<(),String>{
        let mut entries=self.entries.lock().map_err(|_|"Terminais indisponíveis.")?;
        if entries.contains_key(&id){return Err("Este terminal já foi iniciado.".into());}
        if entries.len()>=32{return Err("Limite de 32 terminais nesta execução.").map_err(str::to_owned);}
        let pair=native_pty_system().openpty(PtySize{rows:24,cols:80,pixel_width:0,pixel_height:0}).map_err(|_|"Não foi possível criar o terminal nativo Windows.")?;
        let reader=pair.master.try_clone_reader().map_err(|_|"Saída do terminal indisponível.")?;
        let writer=pair.master.take_writer().map_err(|_|"Entrada do terminal indisponível.")?;
        let mut child=pair.slave.spawn_command(command).map_err(|_|"O CLI não pôde iniciar no terminal integrado.")?;
        drop(pair.slave);
        let entry=Arc::new(Entry{master:Mutex::new(Some(pair.master)),writer:Mutex::new(Some(writer)),history:Mutex::new(History::default()),exited:AtomicBool::new(false)});
        entries.insert(id.clone(),entry.clone());drop(entries);
        let output_entry=entry.clone();
        std::thread::spawn(move||{
            let mut reader=reader;let mut buffer=[0u8;16_384];
            while let Ok(count)=reader.read(&mut buffer){
                if count==0{break;}
                if let Ok(mut history)=output_entry.history.lock(){let chunk=history.append(&id,buffer[..count].to_vec());drop(history);on_output(chunk);}
            }
        });
        std::thread::spawn(move||{let _=child.wait();entry.exited.store(true,Ordering::Release);on_exit();});
        Ok(())
    }
    fn entry(&self,id:&str)->Result<Arc<Entry>,String>{self.entries.lock().map_err(|_|"Terminais indisponíveis.")?.get(id).cloned().ok_or("Terminal não encontrado nesta execução.".into())}
    pub fn input(&self,id:&str,data:&str)->Result<(),String>{
        if data.len()>16_384{return Err("Entrada excede 16 KiB. Divida o texto em blocos menores.".into());}
        let entry=self.entry(id)?;
        if entry.exited.load(Ordering::Acquire){return Err("Este CLI já encerrou.".into());}
        let mut writer=entry.writer.lock().map_err(|_|"Entrada do terminal indisponível.")?;
        let writer=writer.as_mut().ok_or("Este terminal foi fechado.")?;
        writer.write_all(data.as_bytes()).and_then(|_|writer.flush()).map_err(|_|"Não foi possível enviar ao terminal.".into())
    }
    pub fn resize(&self,id:&str,cols:u16,rows:u16)->Result<(),String>{
        if !(10..=400).contains(&cols)||!(2..=200).contains(&rows){return Err("Dimensão de terminal inválida.".into());}
        let entry=self.entry(id)?;
        let master=entry.master.lock().map_err(|_|"Terminal indisponível.")?;
        master.as_ref().ok_or("Este terminal foi fechado.")?.resize(PtySize{rows,cols,pixel_width:0,pixel_height:0}).map_err(|_|"Não foi possível redimensionar o terminal.".into())
    }
    pub fn replay(&self,id:&str)->Result<Replay,String>{
        let entry=self.entry(id)?;
        let chunks=entry.history.lock().map_err(|_|"Histórico indisponível.")?.chunks.iter().cloned().collect();
        Ok(Replay{chunks,exited:entry.exited.load(Ordering::Acquire)})
    }
    pub fn active_ids(&self)->Result<Vec<String>,String>{
        let entries=self.entries.lock().map_err(|_|"Terminais indisponíveis; a saída não pôde ser conferida.")?;
        let mut ids:Vec<_>=entries.iter().filter(|(_,entry)|!entry.exited.load(Ordering::Acquire)).map(|(id,_)|id.clone()).collect();ids.sort();Ok(ids)
    }
    pub fn close_all(&self)->Result<(),String>{
        let entries:Vec<_>=self.entries.lock().map_err(|_|"Terminais indisponíveis.")?.values().cloned().collect();
        for entry in entries {
            let writer=entry.writer.lock().map_err(|_|"A entrada de um terminal não pôde ser fechada.")?.take();drop(writer);
            let master=entry.master.lock().map_err(|_|"Um terminal não pôde ser fechado.")?.take();drop(master);
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn terminal_native_close_releases_only_owned_pseudoconsole(){
        let service=Service::default();let id=uuid::Uuid::new_v4().to_string();
        let mut command=CommandBuilder::new("powershell.exe");command.args(["-NoProfile","-Command","Write-Output 'CAPY_PTY_CLOSE_PROOF'; Start-Sleep -Seconds 30"]);
        let (sender,receiver)=std::sync::mpsc::channel();service.start(id.clone(),command,move|chunk|{let _=sender.send(chunk);},||{}).unwrap();
        let started=std::time::Instant::now();let mut bytes=Vec::new();let mut cursor_answered=false;
        while started.elapsed()<std::time::Duration::from_secs(10){
            if let Ok(chunk)=receiver.recv_timeout(std::time::Duration::from_millis(100)){assert_eq!(chunk.task_id,id);bytes.extend(chunk.data);}
            if !cursor_answered&&bytes.windows(4).any(|window|window==b"\x1b[6n"){service.input(&id,"\x1b[1;1R").unwrap();cursor_answered=true;}
            if String::from_utf8_lossy(&bytes).contains("CAPY_PTY_CLOSE_PROOF"){break;}
        }
        let observed=String::from_utf8_lossy(&bytes).contains("CAPY_PTY_CLOSE_PROOF");
        let active=service.active_ids().unwrap();
        let mut unrelated=std::process::Command::new("powershell.exe").args(["-NoProfile","-Command","Start-Sleep -Seconds 30"]).spawn().unwrap();
        let closed=service.close_all();let started=std::time::Instant::now();
        while !service.active_ids().unwrap().is_empty()&&started.elapsed()<std::time::Duration::from_secs(10){std::thread::sleep(std::time::Duration::from_millis(50));}
        let own_closed=service.active_ids().unwrap().is_empty();let unrelated_alive=unrelated.try_wait().unwrap().is_none();
        let _=unrelated.kill();let _=unrelated.wait();
        assert!(observed);assert_eq!(active,vec![id.clone()]);assert!(closed.is_ok());assert!(own_closed);assert!(unrelated_alive);
        assert!(service.input(&id,"test").is_err());assert!(service.resize(&id,80,24).is_err());assert!(service.close_all().is_ok());
    }
    #[test]
    fn terminal_history_is_bounded_and_sequences_are_monotonic(){
        let mut history=History::default();
        for _ in 0..100 {history.append("one",vec![b'x';16_384]);}
        assert!(history.bytes<=BUFFER_LIMIT);
        assert_eq!(history.chunks.back().unwrap().sequence,100);
        assert_eq!(history.chunks.len(),32);
        assert!(history.chunks.iter().all(|c|c.task_id=="one"));
        let service=Service::default();assert!(service.input("other","test").is_err());assert!(service.resize("other",80,24).is_err());
    }
    #[test]
    fn terminal_native_roundtrip_has_exact_identity(){
        let service=Service::default();
        let mut command=CommandBuilder::new("powershell.exe");command.args(["-NoProfile","-Command","Write-Output 'CAPY_PTY_PROOF'"]);
        let (sender,receiver)=std::sync::mpsc::channel();
        service.start("proof".into(),command,move|chunk|{let _=sender.send(chunk);},||{}).unwrap();
        let start=std::time::Instant::now();let mut bytes=Vec::new();let mut answered_cursor_query=false;
        while start.elapsed()<std::time::Duration::from_secs(15){
            if let Ok(chunk)=receiver.recv_timeout(std::time::Duration::from_millis(100)){assert_eq!(chunk.task_id,"proof");bytes.extend(chunk.data);}
            if !answered_cursor_query && bytes.windows(4).any(|w|w==b"\x1b[6n") {
                service.input("proof","\x1b[1;1R").unwrap();answered_cursor_query=true;
            }
            if String::from_utf8_lossy(&bytes).contains("CAPY_PTY_PROOF"){return;}
        }
        panic!("PTY did not return its own output: {:?}",String::from_utf8_lossy(&bytes));
    }
}
