import assert from 'node:assert/strict';
import {spawn, spawnSync} from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {randomUUID, createHash} from 'node:crypto';
import {createServer} from 'node:net';
import {fileURLToPath} from 'node:url';
import {setTimeout as delay} from 'node:timers/promises';

// Production service/discovery/UI proof. No probe, fixture presence, SDK answers,
// credential copies, or changes to the user's Claude settings are used.
const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const exe = process.env.CAPY_TEST_CAPY_EXE || path.join(repo, 'src-tauri/target/release/capy.exe');
const claude = process.env.CAPY_TEST_CLAUDE_EXE || path.join(os.homedir(), '.local/bin/claude.exe');
const config = process.env.CLAUDE_CONFIG_DIR || path.join(os.homedir(), '.claude');
const allowWrite = process.argv.includes('--allow-write');
function regular(file, directory = false) {
  const absolute = path.resolve(file);
  for (let current = absolute;; current = path.dirname(current)) {
    assert.equal(fs.lstatSync(current).isSymbolicLink(), false, `Symlink/reparse refused: ${current}`);
    if (current === path.dirname(current)) break;
  }
  const stat = fs.lstatSync(absolute);
  assert.ok(directory ? stat.isDirectory() : stat.isFile(), `Unexpected file type: ${absolute}`);
}
regular(exe); regular(claude); regular(config, true);
const scratch = path.join(repo, 'scratch'); fs.mkdirSync(scratch, {recursive:true}); regular(scratch, true);
const root = fs.mkdtempSync(path.join(scratch, 'capy-visual-claude-'));
const sessionId = randomUUID();
const marker = path.join(config, 'capy-activity/responses-enabled');
const markerContent = `capy-native-proof:${randomUUID()}\n`;
let ownedMarker;
function optIn() {
  assert.equal(fs.existsSync(path.join(config, 'capy-activity/disabled')), false, 'Claude activity is disabled at origin');
  if (fs.existsSync(marker)) { regular(marker); return; }
  assert.ok(process.argv.includes('--temporary-optin'), 'Enable responses or pass --temporary-optin');
  const parent = path.dirname(marker); if (!fs.existsSync(parent)) fs.mkdirSync(parent);
  regular(parent, true);
  try { fs.writeFileSync(marker, markerContent, {flag:'wx'}); ownedMarker = fs.lstatSync(marker); }
  catch (error) { if (error.code !== 'EEXIST') throw error; regular(marker); }
}
const report = {root, sessionId, mode:'native WebView2 + Claude print', permissionTransport:'hooks with passive stdio host',
  executable:exe, executableSha256:createHash('sha256').update(fs.readFileSync(exe)).digest('hex'),
  startedAt:new Date().toISOString(), permissionDecision:allowWrite?'allow':'deny', checks:[], passiveHostRequests:0,
  runtimeExceptions:[], logErrors:[], runtimeObservation:'CDP Runtime/Log events after connection; initial startup before connection is not covered', passed:false};
const children = [], connections = [];
let app, appExited = false, panel, timer, failure, failedReject, result, answered = false, continued = false;
const failed = new Promise((_, reject) => { failedReject = reject; }); failed.catch(() => {});
const race = promise => Promise.race([promise, failed]);
const check = (name, condition) => { assert.ok(condition, name); report.checks.push(name); };
let logBytes = 0;
function append(name, value) {
  logBytes += Buffer.byteLength(value); assert.ok(logBytes <= 8*1024*1024, 'Combined subprocess output exceeded 8 MiB');
  fs.appendFileSync(path.join(root, name), value);
}
function start(command, args, name, env) {
  const child = spawn(command, args, {cwd:root, env, windowsHide:true, stdio:['pipe','pipe','pipe']});
  children.push(child); child.on('error', failedReject); child.stdin.on('error', failedReject);
  child.stderr.on('data', data => { try { append(`${name}.stderr.log`, data); } catch(error) { failedReject(error); } });
  return child;
}
async function waitFor(fn, description, timeout = 15000) {
  const until = Date.now()+timeout; let last;
  while(Date.now()<until) {
    if(appExited) throw new Error(`Own Capy exited during ${description}; close the other instance before proof`);
    try { const value=await race(Promise.resolve().then(fn)); if(value) return value; } catch(error) { last=error; }
    await race(delay(100));
  }
  throw new Error(`${description}: ${last?.message || 'timed out'}`);
}
async function targets() {
  return (await fetch(`http://127.0.0.1:${report.cdpPort}/json/list`, {signal:AbortSignal.timeout(1000)})).json();
}
const targetFor = (targets, pathname) => targets.find(t=>t.type==='page' && /^https?:\/\/tauri\.localhost\//.test(t.url) && new URL(t.url).pathname===pathname);
async function connect(target) {
  const socket = new WebSocket(target.webSocketDebuggerUrl); let sequence=0; const pending=new Map();
  await race(new Promise((resolve,reject)=>{socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true});}));
  socket.addEventListener('message',event=>{
    const message=JSON.parse(event.data);
    if(message.method==='Runtime.exceptionThrown')report.runtimeExceptions.push({surface:target.url,details:message.params.exceptionDetails});
    if(message.method==='Log.entryAdded'&&message.params.entry.level==='error')report.logErrors.push({surface:target.url,entry:message.params.entry});
    const request=pending.get(message.id); if(!request)return;
    pending.delete(message.id); clearTimeout(request.timer);
    if(message.error)request.reject(new Error(message.error.message));else request.resolve(message.result);
  });
  socket.addEventListener('close',()=>{for(const request of pending.values()){clearTimeout(request.timer);request.reject(new Error('CDP closed'));}pending.clear();});
  const call=(method,params={})=>race(new Promise((resolve,reject)=>{
    const id=++sequence;const timer=setTimeout(()=>{pending.delete(id);reject(new Error(`CDP ${method} timed out`));},10000);
    pending.set(id,{resolve,reject,timer});socket.send(JSON.stringify({id,method,params}));
  }));
  const evaluate=async expression=>{
    const value=await call('Runtime.evaluate',{expression,awaitPromise:true,returnByValue:true});
    if(value.exceptionDetails)throw new Error(`CDP evaluation failed: ${JSON.stringify(value.exceptionDetails)}`);
    return value.result.value;
  };
  const invoke=(command,args={})=>evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)})`);
  const screenshot=async name=>{const value=await call('Page.captureScreenshot',{format:'png',captureBeyondViewport:false});fs.writeFileSync(path.join(root,`${name}.png`),Buffer.from(value.data,'base64'));};
  const connection={call,evaluate,invoke,screenshot,close:()=>socket.close()}; connections.push(connection);
  await call('Runtime.enable');await call('Log.enable');return connection;
}
async function click(selector) {
  const box=await panel.evaluate(`(()=>{const e=document.querySelector(${JSON.stringify(selector)});if(!e||e.disabled)throw Error('Missing enabled control');e.scrollIntoView({block:'center'});const r=e.getBoundingClientRect();return {x:r.x+r.width/2,y:r.y+r.height/2};})()`);
  await panel.call('Input.dispatchMouseEvent',{type:'mousePressed',...box,button:'left',clickCount:1});
  await panel.call('Input.dispatchMouseEvent',{type:'mouseReleased',...box,button:'left',clickCount:1});
}
const ownRequest = async kind => {
  const snapshot = await panel.invoke('demo_snapshot');
  const request = snapshot.interventions.find(r=>r.sessionId===`claude:${sessionId}`&&r.status==='pending'&&r.body.kind===kind);
  if (!request) return null;
  assert.equal(snapshot.scenario,'real');
  assert.ok(snapshot.sessions.some(s=>s.id===request.sessionId&&s.kind==='claude'&&s.origin===root&&!s.hidden), 'Production discovery sees own live Claude session');
  const selector=`article[data-id="claude:${sessionId}"] [data-request="${request.nonce}"]`;
  if(!await panel.evaluate(`!!document.querySelector(${JSON.stringify(selector)})`))return null;
  return {request,selector};
};
try {
  optIn(); timer=setTimeout(()=>failedReject(new Error('Native Claude proof exceeded 180 seconds')),180000);
  const version=spawnSync(claude,['--version'],{windowsHide:true,encoding:'utf8',timeout:10000});
  assert.equal(version.status,0);report.claudeVersion=version.stdout.trim();
  const server=createServer();await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));report.cdpPort=server.address().port;await new Promise(resolve=>server.close(resolve));
  const env={...process.env,CAPY_CLAUDE_INTERVENTION_ROOT:root};
  app=start(exe,['--visual-test',root],'capy',{...env,WEBVIEW2_USER_DATA_FOLDER:path.join(root,'webview'),WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:`--remote-debugging-port=${report.cdpPort} --remote-debugging-address=127.0.0.1`});
  app.on('exit',()=>{appExited=true;});app.stdout.on('data',data=>{try{append('capy.stdout.log',data);}catch(error){failedReject(error);}});
  const pet=await connect(await waitFor(async()=>{const values=await targets();return targetFor(values,'/index.html')||targetFor(values,'/');},'Own native pet/CDP'));
  await waitFor(()=>pet.evaluate(`document.querySelector('#pet') instanceof SVGSVGElement && !!window.__TAURI_INTERNALS__`),'Native pet SVG and bridge ready');
  let panelShown=false,lastPanelError;
  for(let attempt=0;attempt<30&&!panelShown;attempt++) {
    try { await pet.invoke('show_panel'); panelShown=true; }
    catch(error) { lastPanelError=error; await race(delay(100)); }
  }
  assert.ok(panelShown,`Native panel did not become available: ${lastPanelError?.message || 'no response'}`);
  panel=await connect(await waitFor(async()=>targetFor(await targets(),'/panel.html'),'Own native panel/CDP'));
  await waitFor(()=>panel.evaluate(`!!document.querySelector('#startTask') && !!document.querySelector('#accountList') && !!document.querySelector('[data-scenario="real"]')`),'Native panel controls ready');
  report.logoLoads=await panel.evaluate(`(async()=>{
    const host=document.createElement('div');host.setAttribute('aria-hidden','true');host.style.position='fixed';host.style.left='-10000px';document.body.append(host);
    try{return await Promise.all(['claude','codex','agy'].map(async provider=>{
      const account=document.createElement('div');account.className='quota-account';account.dataset.provider=provider;
      const title=document.createElement('h3');title.textContent=provider;account.append(title);host.append(account);
      const background=getComputedStyle(title,'::before').backgroundImage;
      if(!background.startsWith('url('))throw Error(provider+' production CSS logo missing');
      let uri=background.slice(4,-1);if(uri.startsWith('"'))uri=JSON.parse(uri);
      if(!uri.startsWith('data:image/svg+xml,'))throw Error(provider+' production CSS logo URI unexpected');
      const image=new Image();
      await new Promise((resolve,reject)=>{
        const timer=setTimeout(()=>reject(Error(provider+' production CSS logo load timed out')),3000);
        image.onload=()=>{clearTimeout(timer);resolve();};image.onerror=()=>{clearTimeout(timer);reject(Error(provider+' production CSS logo blocked or invalid'));};image.src=uri;
      });
      return {provider,loaded:image.complete&&image.naturalWidth>0&&image.naturalHeight>0};
    }));}finally{host.remove();}
  })()`);
  check('all three production CSS provider logos load under native CSP',report.logoLoads.length===3&&report.logoLoads.every(logo=>logo.loaded));
  await click('[data-scenario="real"]');
  await waitFor(async()=>(await panel.invoke('demo_snapshot')).scenario==='real','Production real monitor mode');
  check('production real mode selected through native UI',true);
  await waitFor(()=>fs.existsSync(path.join(root,'claude-interventions.json')),'Own production endpoint');
  const settings=path.join(root,'settings.json');
  const hook={type:'command',command:`"${exe}" --claude-intervention`,timeout:120};
  fs.writeFileSync(settings,JSON.stringify({hooks:{PermissionRequest:[{hooks:[hook]}],PreToolUse:[{matcher:'^AskUserQuestion$',hooks:[hook]}]}}));
  const child=start(claude,['-p','--session-id',sessionId,'--settings',settings,'--setting-sources','','--strict-mcp-config','--tools','Write,AskUserQuestion','--permission-mode','manual','--input-format','stream-json','--output-format','stream-json','--verbose','--permission-prompt-tool','stdio','--max-budget-usd','1','--model','haiku','--system-prompt','You are an isolated integration test. Follow the exact tool sequence. No subagents, network, retries, or other tools.'],'claude',env);
  const childExit=new Promise(resolve=>child.once('exit',(code,signal)=>resolve({code,signal})));
  const tools=new Map(), toolResults=[];let buffer='';
  child.stdout.on('data',chunk=>{
    try {
      append('claude.stdout.jsonl',chunk);buffer+=chunk;assert.ok(buffer.length<=1024*1024);
      while(buffer.includes('\n')) {
        const at=buffer.indexOf('\n'),line=buffer.slice(0,at);buffer=buffer.slice(at+1);if(!line.trim())continue;
        const value=JSON.parse(line);
        if(value.type==='control_request') { assert.equal(value.request?.subtype,'can_use_tool','Unexpected control request');report.passiveHostRequests++; }
        if(value.type==='assistant')for(const part of value.message?.content||[]) {
          if(part.type==='tool_use')tools.set(part.id,part.name);
          if(answered&&part.type==='text'&&part.text.trim())continued=true;
        }
        if(value.type==='user')for(const part of value.message?.content||[])if(part.type==='tool_result')toolResults.push({tool:tools.get(part.tool_use_id),part,meta:value.tool_result_meta?.find(m=>m.id===part.tool_use_id)});
        if(value.type==='result'){result=value;child.stdin.end();if(!answered)failedReject(new Error('Claude finished before native question answer'));}
      }
    } catch(error){failedReject(error);}
  });
  childExit.then(({code})=>{if(code!==0||!result)failedReject(new Error(`Claude exited without success (${code})`));});
  child.stdin.write(JSON.stringify({type:'user',message:{role:'user',content:`Use Write exactly ONCE to create probe.txt in the current directory containing exactly capy-proof with no newline. The user will ${allowWrite?'allow':'deny'} it; do not retry. Then use AskUserQuestion exactly ONCE with one question "Which proof option?", header "Proof", multiSelect false, and two options: label "Alpha" description "First option", label "Beta" description "Second option". Wait for the answer and then thank the user, mentioning the selected option. Do not use any other tools.`}})+'\n');
  const permission=await waitFor(()=>ownRequest('claudePermission'),'Own Claude native permission card',65000);
  assert.equal(permission.request.body.toolName,'Write');assert.equal(path.resolve(permission.request.body.toolInput.file_path),path.join(root,'probe.txt'));
  fs.writeFileSync(path.join(root,'permission.json'),JSON.stringify(permission.request,null,2));await panel.screenshot('permission');
  await click(`${permission.selector} [data-decision="${allowWrite?'allow':'deny'}"]`);check(`native permission ${allowWrite?'allow':'deny'} clicked`,true);
  const question=await waitFor(()=>ownRequest('question'),'Own Claude native question card',65000);
  assert.equal(question.request.body.questions.length,1);assert.equal(question.request.body.questions[0].question,'Which proof option?');
  fs.writeFileSync(path.join(root,'question.json'),JSON.stringify(question.request,null,2));
  await click(`${question.selector} input[data-question-answer][value="Alpha"]`);
  await panel.screenshot('question-alpha');answered=true;
  await click(`${question.selector} [data-action="submit-intervention"]`);check('native Alpha question answer submitted',true);
  assert.equal((await race(childExit)).code,0);assert.equal(result?.subtype,'success');
  check('assistant continued after native answer with Alpha',continued&&/Alpha/i.test(result.result||''));
  const writeResult=toolResults.find(r=>r.tool==='Write'), questionResult=toolResults.find(r=>r.tool==='AskUserQuestion');
  if(allowWrite)check('Write allowed by hook',!!writeResult&&writeResult.part.is_error!==true&&writeResult.meta?.permission_decision?.source==='hook'&&writeResult.meta.permission_decision.decision==='accept');
  else check('Write denied by hook',writeResult?.part.is_error===true&&writeResult.meta?.permission_decision?.source==='hook'&&writeResult.meta.permission_decision.decision==='reject');
  check('AskUserQuestion answered by hook',questionResult?.meta?.permission_decision?.source==='hook'&&questionResult.meta.permission_decision.decision==='accept'&&JSON.stringify(questionResult.part.content).includes('Alpha'));
  if(allowWrite)check('allowed file contains exact requested content',fs.readFileSync(path.join(root,'probe.txt'),'utf8')==='capy-proof');
  else check('denied file absent',!fs.existsSync(path.join(root,'probe.txt')));
  report.result={subtype:result.subtype,summary:result.result,totalCostUsd:result.total_cost_usd};report.toolResults=toolResults;
  await delay(250);
  check('no native runtime exceptions observed after CDP connection',report.runtimeExceptions.length===0);
  check('no native log errors observed after CDP connection',report.logErrors.length===0);
  report.passed=true;
} catch(error) { failure=error;report.error=error.stack||String(error);process.exitCode=1; }
finally {
  clearTimeout(timer);
  if(failure&&panel)try{await panel.screenshot('failure');fs.writeFileSync(path.join(root,'failure-snapshot.json'),JSON.stringify(await panel.invoke('demo_snapshot'),null,2));}catch(error){report.captureError=String(error);}
  for(const connection of connections)connection.close();
  for(const child of children)if(child.pid&&child.exitCode===null&&child.signalCode===null)spawnSync('taskkill',['/PID',String(child.pid),'/T','/F'],{windowsHide:true,stdio:'ignore',timeout:10000});
  if(ownedMarker)try {
    regular(marker);const current=fs.lstatSync(marker);
    if(current.dev===ownedMarker.dev&&current.ino===ownedMarker.ino&&current.birthtimeMs===ownedMarker.birthtimeMs&&current.mtimeMs===ownedMarker.mtimeMs&&fs.readFileSync(marker,'utf8')===markerContent)fs.unlinkSync(marker);
    else report.markerCleanup='Marker changed externally; preserved';
  }catch(error){report.markerCleanup=`Marker preserved: ${error.message}`;}
  report.finishedAt=new Date().toISOString();fs.writeFileSync(path.join(root,'report.json'),JSON.stringify(report,null,2));
  console.log(JSON.stringify({passed:report.passed,checks:report.checks.length,root,error:failure?.message||null}));
}
