import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { mkdir, writeFile, readFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { homedir } from 'node:os';
import { randomUUID } from 'node:crypto';

const dir = join(resolve(import.meta.dirname, '..'), 'scratch', `claude-question-${randomUUID()}`);
await mkdir(dir,{recursive:true});
await writeFile(join(dir,'CLAUDE.md'),'Isolated integration test. Follow the prompt exactly. Only AskUserQuestion is permitted. No files, network, skills or subagents.\n');
const marker = join(dir,'held.json');
const wrapper = join(dir,'question.cjs');
await writeFile(wrapper, `
const fs=require('node:fs');let input='';
process.stdin.on('data',c=>input+=c);process.stdin.on('end',()=>{
 const h=JSON.parse(input);
 if(h.hook_event_name!=='PreToolUse'||h.tool_name!=='AskUserQuestion')process.exit(2);
 const questions=h.tool_input?.questions;
 if(!Array.isArray(questions)||questions.length!==1||!questions[0].options.some(o=>o.label==='Stop'))process.exit(2);
 fs.writeFileSync(${JSON.stringify(marker)},JSON.stringify({pid:process.pid,at:Date.now(),toolUseId:h.tool_use_id,sessionId:h.session_id,questionCount:questions.length}));
 setTimeout(()=>process.stdout.write(JSON.stringify({hookSpecificOutput:{hookEventName:'PreToolUse',permissionDecision:'allow',updatedInput:{...h.tool_input,questions,answers:{[questions[0].question]:'Stop'}}}})),35000);
});
`);
const settings = join(dir,'settings.json');
await writeFile(settings,JSON.stringify({hooks:{PreToolUse:[{matcher:'AskUserQuestion',hooks:[{type:'command',command:`node "${wrapper}"`,timeout:60}]}]}}));
const id = randomUUID();
const claude = process.env.CAPY_TEST_CLAUDE_EXE || join(homedir(),'.local/bin/claude.exe');
let child;
let timer;
let heldProven = false;
try {
  child = spawn(claude,['-p','--session-id',id,'--settings',settings,'--setting-sources','','--strict-mcp-config','--tools','AskUserQuestion','--permission-mode','manual','--permission-prompt-tool','stdio','--input-format','stream-json','--output-format','stream-json','--verbose','--max-budget-usd','1','--system-prompt','You are an isolated integration test. Use AskUserQuestion exactly once, with question Which test path?, header Path, options Continue and Stop, and multiSelect false. After its answer print CAPY_RESPONSE_CONFIRMED: followed by the chosen answer. Do nothing else.'],{cwd:dir,windowsHide:true,stdio:['pipe','pipe','pipe']});
  console.log('Own Claude question test started; hook will hold for 35 seconds.');
  const done = new Promise((resolve,reject)=>{
    let buffer='',gotResult=false;
    timer = setTimeout(()=>reject(new Error('Question hook probe timed out')),120000);
    child.on('error',reject);
    child.stdout.on('data',chunk=>{
      buffer+=chunk;
      while(buffer.includes('\n')) {
        const at=buffer.indexOf('\n'),line=buffer.slice(0,at);buffer=buffer.slice(at+1);
        let v;try{v=JSON.parse(line);}catch{continue;}
        if(v.type==='control_request'&&v.request?.subtype==='can_use_tool') {
          child.stdin.write(JSON.stringify({type:'control_response',response:{subtype:'success',request_id:v.request_id,response:{behavior:'deny',message:'Only the question hook may answer this test.'}}})+'\n');
        }
        if(v.type==='result') {
          try {
            gotResult=true;
            assert.equal(v.subtype,'success');
            assert.match(v.result,/CAPY_RESPONSE_CONFIRMED:\s*Stop/);
            assert.ok(heldProven,'Response arrived before the held-hook proof');
            child.stdin.end();
          } catch(error) {reject(error);}
        }
      }
    });
    child.stderr.on('data',()=>{});
    child.on('exit',code=>{clearTimeout(timer);try{assert.equal(code,0);assert.ok(gotResult);resolve();}catch(error){reject(error);}});
  });
  child.stdin.write(JSON.stringify({type:'user',message:{role:'user',content:'Ask the exact test question using AskUserQuestion, then report the answer exactly as specified. No other tools.'}})+'\n');
  const observe = async () => {
    const deadline=Date.now()+90000;
    while(Date.now()<deadline) {
      let h;try{h=JSON.parse(await readFile(marker,'utf8'));}catch{}
      if(h&&Date.now()-h.at>31000) {
        assert.equal(h.sessionId,id);
        assert.equal(h.questionCount,1);
        assert.equal(typeof h.toolUseId,'string');
        process.kill(h.pid,0);
        assert.equal(child.exitCode,null);
        heldProven=true;
        console.log('PASS: exact question/session callback held alive beyond 30 seconds.');
        return;
      }
      if(child.exitCode!==null) throw new Error('Claude exited before held-hook observation');
      await new Promise(resolve=>setTimeout(resolve,250));
    }
    throw new Error('Held hook observation timed out');
  };
  await Promise.all([done,observe()]);
  await writeFile(join(dir,'proof.json'),JSON.stringify({heldBeyond30Seconds:true,exactAnswerConsumed:true},null,2));
  console.log('PASS: Claude consumed the exact hook answer and completed. No Capy response feature claimed.');
} finally {
  clearTimeout(timer);
  if(child&&child.exitCode===null) spawnSync('taskkill',['/PID',String(child.pid),'/T','/F'],{windowsHide:true,stdio:'ignore'});
}
