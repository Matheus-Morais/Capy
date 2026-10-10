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
const root = fs.mkdtempSync(path.join(scratch, 'capy-visual-concurrent-'));
const sessions = ['A', 'B'].map(label => ({label, id:randomUUID(), cwd:path.join(root,label), content:`capy-concurrent-${label}-${randomUUID()}`, toolResults:[], hostWrites:[], controlRequests:0}));
for (const session of sessions) fs.mkdirSync(session.cwd);
const marker = path.join(config, 'capy-activity/responses-enabled');
const markerContent = `capy-concurrent-proof:${randomUUID()}\n`;
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
const report = {root, sessions, mode:'concurrent native WebView2 + Claude print', permissionTransport:'hooks with passive stdio host',
  executable:exe, executableSha256:createHash('sha256').update(fs.readFileSync(exe)).digest('hex'),
  startedAt:new Date().toISOString(), decisionOrder:['B deny','A allow'], checks:[], passiveHostRequests:0,
  runtimeExceptions:[], logErrors:[], runtimeObservation:'CDP Runtime/Log events after connection; initial startup before connection is not covered', passed:false};
const children = [], connections = [];
let app, appExited = false, panel, timer, failure, failedReject;
const failed = new Promise((_, reject) => { failedReject = reject; }); failed.catch(() => {});
const race = promise => Promise.race([promise, failed]);
const check = (name, condition) => { assert.ok(condition, name); report.checks.push(name); };
let logBytes = 0;
function append(name, value) {
  logBytes += Buffer.byteLength(value); assert.ok(logBytes <= 8*1024*1024, 'Combined subprocess output exceeded 8 MiB');
  fs.appendFileSync(path.join(root, name), value);
}
function start(command, args, name, env, cwd = root) {
  const child = spawn(command, args, {cwd, env, windowsHide:true, stdio:['pipe','pipe','pipe']});
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
async function ownRequest(session) {
  const snapshot = await panel.invoke('demo_snapshot');
  const request = snapshot.interventions.find(r=>r.sessionId===`claude:${session.id}`&&r.status==='pending'&&r.body.kind==='claudePermission');
  if (!request) return null;
  assert.equal(snapshot.scenario,'real');
  assert.ok(snapshot.sessions.some(s=>s.id===request.sessionId&&s.kind==='claude'&&s.origin===session.cwd&&!s.hidden), `Production discovery sees live session ${session.label}`);
  const selector=`article[data-id="claude:${session.id}"] [data-request="${request.nonce}"]`;
  if (!await panel.evaluate(`!!document.querySelector(${JSON.stringify(selector)})`)) return null;
  return {request,selector};
}
function startSession(session, settings, env) {
  const child=start(claude,['-p','--session-id',session.id,'--settings',settings,'--setting-sources','','--strict-mcp-config','--tools','Write','--permission-mode','manual','--input-format','stream-json','--output-format','stream-json','--verbose','--permission-prompt-tool','stdio','--max-budget-usd','1','--model','haiku','--system-prompt','You are an isolated integration test. Follow the exact tool sequence. No subagents, network, retries, or other tools.'],`claude-${session.label}`,env,session.cwd);
  const tools=new Map(); let buffer='';
  session.exit=new Promise(resolve=>child.once('exit',(code,signal)=>resolve({code,signal})));
  child.stdout.on('data',chunk=>{
    try {
      append(`claude-${session.label}.stdout.jsonl`,chunk);buffer+=chunk;assert.ok(buffer.length<=1024*1024);
      while(buffer.includes('\n')) {
        const at=buffer.indexOf('\n'),line=buffer.slice(0,at);buffer=buffer.slice(at+1);if(!line.trim())continue;
        const value=JSON.parse(line);
        if(value.type==='control_request') {
          assert.equal(value.request?.subtype,'can_use_tool','Unexpected host control request');
          session.controlRequests++;report.passiveHostRequests++;
        }
        if(value.type==='assistant')for(const part of value.message?.content||[]) {
          if(part.type==='tool_use')tools.set(part.id,part.name);
          if(session.decisionClickedAt&&part.type==='text'&&part.text.trim())session.continued=true;
        }
        if(value.type==='user')for(const part of value.message?.content||[])if(part.type==='tool_result')session.toolResults.push({tool:tools.get(part.tool_use_id),part,meta:value.tool_result_meta?.find(m=>m.id===part.tool_use_id)});
        if(value.type==='result') {
          session.result={subtype:value.subtype,summary:value.result,totalCostUsd:value.total_cost_usd};
          child.stdin.end();
          if(!session.decisionClickedAt)failedReject(new Error(`Session ${session.label} finished before UI decision`));
        }
      }
    }catch(error){failedReject(error);}
  });
  session.exit.then(({code})=>{if(code!==0||!session.result)failedReject(new Error(`Session ${session.label} exited without success (${code})`));});
  const message={type:'user',message:{role:'user',content:`Use Write exactly ONCE to create probe.txt in the current directory containing exactly ${session.content} with no newline. Wait for the user's decision. Do not retry if denied. Then respond with the exact phrase "Session ${session.label} continued" and describe whether the write was allowed or denied. Do not use any other tools.`}};
  session.hostWrites.push(message);
  fs.writeFileSync(path.join(root,`claude-${session.label}.stdin.jsonl`),JSON.stringify(message)+'\n');
  child.stdin.write(JSON.stringify(message)+'\n');
  // Do not send control_response: only the real PermissionRequest hook can resolve it.
}
try {
  optIn();timer=setTimeout(()=>failedReject(new Error('Concurrent Claude proof exceeded 180 seconds')),180000);
  const version=spawnSync(claude,['--version'],{windowsHide:true,encoding:'utf8',timeout:10000});
  assert.equal(version.status,0);report.claudeVersion=version.stdout.trim();
  const server=createServer();await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));report.cdpPort=server.address().port;await new Promise(resolve=>server.close(resolve));
  const env={...process.env,CAPY_CLAUDE_INTERVENTION_ROOT:root};
  app=start(exe,['--visual-test',root],'capy',{...env,WEBVIEW2_USER_DATA_FOLDER:path.join(root,'webview'),WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:`--remote-debugging-port=${report.cdpPort} --remote-debugging-address=127.0.0.1`});
  app.on('exit',()=>{appExited=true;});app.stdout.on('data',data=>{try{append('capy.stdout.log',data);}catch(error){failedReject(error);}});
  const pet=await connect(await waitFor(async()=>{const values=await targets();return targetFor(values,'/index.html')||targetFor(values,'/');},'Own native pet/CDP'));
  await waitFor(()=>pet.evaluate(`document.querySelector('#pet') instanceof SVGSVGElement && !!window.__TAURI_INTERNALS__`),'Native pet and bridge ready');
  await waitFor(async()=>{await pet.invoke('show_panel');return true;},'Native panel shown');
  panel=await connect(await waitFor(async()=>targetFor(await targets(),'/panel.html'),'Own native panel/CDP'));
  await waitFor(()=>panel.evaluate(`!!document.querySelector('[data-scenario="real"]')`),'Native real mode control ready');
  await click('[data-scenario="real"]');
  await waitFor(async()=>(await panel.invoke('demo_snapshot')).scenario==='real','Production real monitor mode');
  check('production real mode selected through native UI',true);
  const endpoint=path.join(root,'claude-interventions.json');
  await waitFor(()=>fs.existsSync(endpoint),'Own production endpoint');
  const endpointIdentity=JSON.parse(fs.readFileSync(endpoint,'utf8'));
  report.endpoint={root,addr:endpointIdentity.addr,pid:endpointIdentity.pid,birth:endpointIdentity.birth};
  const settings=path.join(root,'settings.json');
  fs.writeFileSync(settings,JSON.stringify({hooks:{PermissionRequest:[{hooks:[{type:'command',command:`"${exe}" --claude-intervention`,timeout:120}]}]}}));
  for(const session of sessions) startSession(session,settings,env);
  const [a,b]=sessions;
  const pending=await waitFor(async()=>{
    const values=await Promise.all(sessions.map(ownRequest));
    return values.every(Boolean)?values:null;
  },'Both real Claude permissions simultaneously pending',90000);
  assert.notEqual(pending[0].request.nonce,pending[1].request.nonce);
  for(let i=0;i<sessions.length;i++) {
    const session=sessions[i],request=pending[i].request;
    assert.equal(request.body.toolName,'Write');
    assert.equal(path.resolve(request.body.toolInput.file_path),path.join(session.cwd,'probe.txt'));
    assert.equal(request.body.toolInput.content,session.content);
    assert.equal(fs.existsSync(path.join(session.cwd,'probe.txt')),false);
    session.request=request;
  }
  const simultaneous=await panel.invoke('demo_snapshot');
  check('both distinct sessions pending in one production snapshot',sessions.every(session=>simultaneous.interventions.some(r=>r.sessionId===`claude:${session.id}`&&r.nonce===session.request.nonce&&r.status==='pending')));
  fs.writeFileSync(path.join(root,'simultaneous-pending.json'),JSON.stringify(simultaneous,null,2));
  await panel.screenshot('simultaneous-pending');
  b.decisionClickedAt=new Date().toISOString();
  await click(`${pending[1].selector} [data-decision="deny"]`);
  check('session B denied first through its native permission card',true);
  const aStillPending=await waitFor(()=>ownRequest(a),'Session A remains pending after B denial');
  assert.equal(aStillPending.request.nonce,a.request.nonce);
  assert.equal((await race(b.exit)).code,0);
  check('B denial completes while A is still pending',!!await ownRequest(a));
  check('neither file exists before A approval',sessions.every(session=>!fs.existsSync(path.join(session.cwd,'probe.txt'))));
  fs.writeFileSync(path.join(root,'after-b-deny.json'),JSON.stringify(await panel.invoke('demo_snapshot'),null,2));
  await panel.screenshot('after-b-deny-a-pending');
  a.decisionClickedAt=new Date().toISOString();
  await click(`${aStillPending.selector} [data-decision="allow"]`);
  check('session A allowed second through its native permission card',true);
  assert.equal((await race(a.exit)).code,0);
  for(const session of sessions) {
    check(`${session.label} finished successfully and continued after UI decision`,session.result?.subtype==='success'&&session.continued&&session.result.summary.includes(`Session ${session.label} continued`));
    assert.equal(session.toolResults.length,1,`Session ${session.label} must execute exactly one tool`);
    const write=session.toolResults[0];assert.equal(write.tool,'Write');
    check(`${session.label} decision attributed to real hook`,write.meta?.permission_decision?.source==='hook'&&write.meta.permission_decision.decision===(session===a?'accept':'reject'));
    assert.equal(write.part.is_error===true,session===b);
    check(`${session.label} received no host control_response`,session.hostWrites.length===1&&session.hostWrites.every(message=>message.type==='user'));
    delete session.exit;
  }
  check('only A wrote its exact unique content',fs.readFileSync(path.join(a.cwd,'probe.txt'),'utf8')===a.content);
  check('B denied file absent',!fs.existsSync(path.join(b.cwd,'probe.txt')));
  fs.writeFileSync(path.join(root,'completed-snapshot.json'),JSON.stringify(await panel.invoke('demo_snapshot'),null,2));
  await panel.screenshot('completed');await delay(250);
  check('no native runtime exceptions observed after CDP connection',report.runtimeExceptions.length===0);
  check('no native log errors observed after CDP connection',report.logErrors.length===0);
  report.passed=true;
}catch(error){failure=error;report.error=error.stack||String(error);process.exitCode=1;}
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
  for(const session of sessions)delete session.exit;
  report.finishedAt=new Date().toISOString();fs.writeFileSync(path.join(root,'report.json'),JSON.stringify(report,null,2));
  console.log(JSON.stringify({passed:report.passed,checks:report.checks.length,root,error:failure?.message||null}));
}
