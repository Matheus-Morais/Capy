import { spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const exe = path.join(root, 'src-tauri/target/release/capy.exe');
const claude = process.env.CAPY_TEST_CLAUDE_EXE || path.join(os.homedir(), '.local/bin/claude.exe');
const config = process.env.CLAUDE_CONFIG_DIR || path.join(os.homedir(), '.claude');
const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'capy-hooks-test-'));
const id = randomUUID();
const metadata = path.join(config, 'capy-activity', `${id}.json`);
const log = path.join(dir, 'proof.jsonl');
const events = ['SessionStart','UserPromptSubmit','PreToolUse','PermissionRequest','PermissionDenied','PostToolUse','PostToolUseFailure','PostToolBatch','Notification','Stop','StopFailure','SessionEnd'];
const wrapper = path.join(dir, 'observer.cjs');
fs.writeFileSync(wrapper, `
const fs=require('node:fs'),{spawnSync}=require('node:child_process');
let input='';process.stdin.on('data',c=>input+=c);process.stdin.on('end',()=>{
 const x=JSON.parse(input);
 const result=spawnSync(${JSON.stringify(exe)},['--claude-hook'],{input,windowsHide:true,timeout:5000});
 if(result.status!==0 || result.stdout.length || result.stderr.length) process.exit(3);
 let state=null;try{state=JSON.parse(fs.readFileSync(${JSON.stringify(metadata)},'utf8')).state;}catch{}
 fs.appendFileSync(${JSON.stringify(log)},JSON.stringify({event:x.hook_event_name,state,stopActive:x.stop_hook_active})+'\\n');
 if(x.hook_event_name==='Stop'&&!fs.existsSync(${JSON.stringify(path.join(dir,'continued'))})) {
   fs.writeFileSync(${JSON.stringify(path.join(dir,'continued'))},'1');
   process.stdout.write(JSON.stringify({decision:'block',reason:'Isolated test: reply CONTINUED now, with no tools.'}));
 }
});`);
const settings = path.join(dir, 'settings.json');
fs.writeFileSync(settings, JSON.stringify({hooks:Object.fromEntries(events.map(event=>[event,[{hooks:[{type:'command',command:`node "${wrapper}"`,timeout:10}]}]]))}));
let child, timer;
function report() {
  const output=path.join(dir,'report.json');
  const r=spawnSync(exe,['--activity-report',output],{windowsHide:true,timeout:15000});
  assert.equal(r.status,0,'activity report failed');
  return JSON.parse(fs.readFileSync(output,'utf8')).sessions.find(s=>s.id===`claude:${id}`);
}
async function waitForWaiting() {
  const until=Date.now()+10000;
  while(Date.now()<until) {
    try {if(JSON.parse(fs.readFileSync(metadata,'utf8')).state==='waiting')return;}catch{}
    await new Promise(resolve=>setTimeout(resolve,100));
  }
  throw new Error('PermissionRequest observer did not record waiting');
}
function killOwnTree() {
  if(child&&child.exitCode===null) spawnSync('taskkill',['/PID',String(child.pid),'/T','/F'],{windowsHide:true,stdio:'ignore'});
}
try {
  child=spawn(claude,['-p','--session-id',id,'--settings',settings,'--setting-sources','','--strict-mcp-config','--tools','Write','--permission-mode','manual','--input-format','stream-json','--output-format','stream-json','--verbose','--permission-prompt-tool','stdio','--max-budget-usd','1','--system-prompt','You are an isolated integration test. Follow instructions exactly. No subagents or network.'],{cwd:dir,windowsHide:true,stdio:['pipe','pipe','pipe']});
  console.log('Own Claude session started; no existing session resumed.');
  const finished=new Promise((resolve,reject)=>{
    let buffer='',permissionCount=0,gotResult=false;
    timer=setTimeout(()=>{child.kill();reject(new Error('Claude hook proof timed out'));},120000);
    child.on('error',reject);
    child.stdout.on('data',async chunk=>{
      buffer+=chunk;
      while(buffer.includes('\n')) {
        const at=buffer.indexOf('\n'),line=buffer.slice(0,at);buffer=buffer.slice(at+1);
        let x;try{x=JSON.parse(line);}catch{continue;}
        if(x.type==='control_request'&&x.request?.subtype==='can_use_tool') {
          try {
            permissionCount++;
            await waitForWaiting();
            const session=report();assert.ok(session,'live test session not discovered');
            assert.equal(session.state,'waiting');assert.equal(session.request,null);assert.equal(session.command,null);
            const m=JSON.parse(fs.readFileSync(metadata,'utf8'));
            assert.equal(m.pid,child.pid);assert.equal(m.session_id,id);assert.equal(m.cwd,dir);
            assert.deepEqual(Object.keys(m).sort(),['version','session_id','cwd','pid','proc_start','at_ms','state'].sort());
            console.log('PASS: live waiting, matching Claude PID/session/project, metadata only, no real response controls');
            child.stdin.write(JSON.stringify({type:'control_response',response:{subtype:'success',request_id:x.request_id,response:{behavior:'deny',message:'Own test session: denied; do not retry.'}}})+'\n');
          } catch(error) {console.error('Proof failed:',error.message);if(fs.existsSync(log))console.error(fs.readFileSync(log,'utf8'));killOwnTree();reject(error);}
        }
        if(x.type==='result') {
          try {gotResult=true;assert.equal(x.subtype,'success');child.stdin.end();}
          catch(error) {child.kill();reject(error);}
        }
      }
    });
    child.stderr.on('data',()=>{});
    child.on('exit',code=>{clearTimeout(timer);try{assert.equal(code,0);assert.equal(permissionCount,1);assert.ok(gotResult);resolve();}catch(e){reject(e);}});
  });
  child.stdin.write(JSON.stringify({type:'user',message:{role:'user',content:'Use Write exactly once to create probe.txt in the current directory with content hook-test. If denied, do not retry. Then answer OK.'}})+'\n');
  await finished;
  const proof=fs.readFileSync(log,'utf8').trim().split('\n').map(line=>JSON.parse(line));
  assert.ok(proof.some(x=>x.event==='UserPromptSubmit'&&x.state==='working'));
  assert.ok(proof.some(x=>x.event==='PermissionRequest'&&x.state==='waiting'));
  assert.ok(proof.some(x=>x.event==='PostToolBatch'&&x.state==='working'));
  assert.ok(proof.some(x=>x.event==='Stop'&&x.state==='unknown'&&x.stopActive===false));
  assert.ok(proof.some(x=>x.event==='Stop'&&x.state==='unknown'&&x.stopActive===true));
  assert.ok(proof.some(x=>x.event==='SessionEnd'&&x.state==='unknown'));
  assert.equal(fs.existsSync(path.join(dir,'probe.txt')),false);
  assert.equal(report(),undefined,'ended test session remained visible');
  console.log('PASS: working -> waiting -> working -> unknown, Stop continuation, ended session removed; observer silent');
} finally {
  clearTimeout(timer);
  if(child&&child.exitCode===null) {
    await new Promise(resolve=>{const stop=setTimeout(resolve,5000);child.once('close',()=>{clearTimeout(stop);resolve();});killOwnTree();});
  }
  fs.rmSync(metadata,{force:true});
  assert.ok(dir.startsWith(path.join(os.tmpdir(),'capy-hooks-test-')));
  try {fs.rmSync(dir,{recursive:true,force:true,maxRetries:10,retryDelay:200});}
  catch {console.error('Temporary proof directory still locked; cleanup deferred.');}
}
