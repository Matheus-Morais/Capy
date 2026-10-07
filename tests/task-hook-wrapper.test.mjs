import test from 'node:test';
import assert from 'node:assert/strict';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {mkdtemp,readFile,writeFile,rm,access} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,dirname,basename,resolve} from 'node:path';
import {setTimeout as delay} from 'node:timers/promises';

test('task hook wrapper keeps its GUI collector parent alive and preserves UTF-8 input',{skip:process.platform!=='win32'},async()=>{
  const root=await mkdtemp(join(tmpdir(),'capy-task-hook-wrapper-'));
  const completed=join(root,'parent-alive.txt');
  let launched=false;
  try{
    const source=join(root,'collector.rs');const collector=join(root,'collector.exe');
    await writeFile(source,`#![windows_subsystem="windows"]
use std::{io::Read,ffi::c_void};
#[link(name="kernel32")]
unsafe extern "system" { fn OpenProcess(access:u32,inherit:i32,pid:u32)->*mut c_void;fn WaitForSingleObject(handle:*mut c_void,timeout:u32)->u32;fn CloseHandle(handle:*mut c_void)->i32; }
fn main(){
 let mut input=Vec::new();std::io::stdin().read_to_end(&mut input).unwrap();std::thread::sleep(std::time::Duration::from_millis(350));
 let pid=std::env::var("CAPY_HOOK_WRAPPER_PID").unwrap().parse().unwrap();
 let alive=unsafe{let handle=OpenProcess(0x00100000,0,pid);let alive=!handle.is_null()&&WaitForSingleObject(handle,0)==0x102;if !handle.is_null(){CloseHandle(handle);}alive};
 std::fs::write(std::env::var("CAPY_HOOK_INPUT_FILE").unwrap(),input).unwrap();
 std::fs::write(std::env::var("CAPY_HOOK_ALIVE_FILE").unwrap(),if alive{"true"}else{"false"}).unwrap();
}
`);
    await promisify(execFile)('rustc',[source,'--edition=2024','-o',collector],{windowsHide:true,timeout:30_000,maxBuffer:8192});
    const script=join(root,'task-hooks.ps1');
    const production=await readFile(resolve('src-tauri/scripts/task-hooks.ps1'),'utf8');const firstLine=production.indexOf('\n')+1;
    await writeFile(script,production.slice(0,firstLine)+'$env:CAPY_HOOK_WRAPPER_PID=$PID\n'+production.slice(firstLine));
    const payload={session_id:'11111111-1111-1111-1111-111111111111',hook_event_name:'InstructionsLoaded',file_path:'C:/own/ação_日本語_🦫.md'};
    const inputFile=join(root,'input.json');
    const pending=promisify(execFile)('powershell.exe',['-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-File',script,'-CapyExe',collector],{windowsHide:true,timeout:20_000,maxBuffer:8192,env:{...process.env,CAPY_HOOK_INPUT_FILE:inputFile,CAPY_HOOK_ALIVE_FILE:completed}});
    launched=true;pending.child.stdin.end(JSON.stringify(payload)+'\n');await pending;
    assert.equal(await readFile(completed,'utf8'),'true');assert.deepEqual(JSON.parse(await readFile(inputFile,'utf8')),payload);
  }finally{
    if(launched){const started=Date.now();while(Date.now()-started<10_000){try{await access(completed);break;}catch{await delay(50);}}await delay(100);}
    assert.equal(dirname(root),tmpdir());assert.ok(basename(root).startsWith('capy-task-hook-wrapper-'));
    await rm(root,{recursive:true,force:true});
  }
});
