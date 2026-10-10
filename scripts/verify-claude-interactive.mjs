import assert from 'node:assert/strict';
import {spawn, spawnSync} from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {randomUUID, createHash} from 'node:crypto';
import {createServer} from 'node:net';
import {fileURLToPath} from 'node:url';
import {setTimeout as delay} from 'node:timers/promises';

// Native interactive proof helper. Claude is launched separately by the orchestrator
// in exec_command tty:true. This helper never launches Claude or uses print/SDK mode.
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
const root = fs.mkdtempSync(path.join(scratch, 'capy-visual-interactive-'));
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
const report = {root, sessionId, mode:'native WebView2 + externally launched interactive Claude TTY', permissionTransport:'native hook raw stdio forwarding',
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
  optIn(); timer=setTimeout(()=>failedReject(new Error('Native Claude proof exceeded 300 seconds')),300000);
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
  const wrapper=path.join(root,'hook-capture.mjs');
  fs.writeFileSync(wrapper,`import fs from 'node:fs';
import {spawn} from 'node:child_process';
import {randomUUID} from 'node:crypto';
import path from 'node:path';
const root=${JSON.stringify(root)}, exe=${JSON.stringify(exe)};
const id=randomUUID(), base=path.join(root,'hook-'+id);
const chunks=[]; for await(const chunk of process.stdin)chunks.push(chunk);
const input=Buffer.concat(chunks); fs.writeFileSync(base+'.stdin.raw',input);
const child=spawn(exe,['--claude-intervention'],{env:process.env,stdio:['pipe','pipe','pipe'],windowsHide:true});
fs.writeFileSync(base+'.provenance.json',JSON.stringify({wrapperPid:process.pid,nativeHookPid:child.pid,exe,argv:['--claude-intervention'],startedAt:new Date().toISOString()}));
child.stdout.on('data',chunk=>{fs.appendFileSync(base+'.stdout.raw',chunk);process.stdout.write(chunk);});
child.stderr.on('data',chunk=>{fs.appendFileSync(base+'.stderr.raw',chunk);process.stderr.write(chunk);});
child.on('error',error=>{fs.writeFileSync(base+'.error.txt',String(error));process.exitCode=1;});
child.on('close',code=>{fs.writeFileSync(base+'.exit.json',JSON.stringify({code,finishedAt:new Date().toISOString()}));process.exitCode=code??1;});
child.stdin.end(input);
`);
  const hook={type:'command',command:`"${process.execPath}" "${wrapper}"`,timeout:120};
  fs.writeFileSync(settings,JSON.stringify({hooks:{PermissionRequest:[{hooks:[hook]}]}}));
  const prompt=`Use Write exactly ONCE to create interactive-proof.txt in the current directory containing exactly capy-interactive-proof with no newline. The user will ${allowWrite?'allow':'deny'} it using the external permission hook. Do not retry or use any other tool. After the tool result, state whether the write succeeded or was denied, then say INTERACTIVE_FINISHED. Stay in this interactive session.`;
  const argv=['--session-id',sessionId,'--settings',settings,'--setting-sources=','--strict-mcp-config','--tools','Write','--permission-mode','manual','--model','haiku','--system-prompt','You are an isolated integration test. Follow the exact tool sequence. No subagents, network, retries, or other tools.',prompt];
  const psquote=value=>"'"+value.replaceAll("'","''")+"'";
  const ttyProbe="const proof={stdin:process.stdin.isTTY===true,stdout:process.stdout.isTTY===true,stderr:process.stderr.isTTY===true,pid:process.pid,recordedAt:new Date().toISOString()};require('node:fs').writeFileSync(process.argv[1],JSON.stringify(proof));console.log(JSON.stringify({terminalTtyProof:proof}));if(!proof.stdin||!proof.stdout||!proof.stderr)process.exit(2);";
  const launch=path.join(root,'launch-interactive.ps1');
  fs.writeFileSync(launch,`$ErrorActionPreference = 'Stop'\nSet-Location -LiteralPath ${psquote(root)}\n$env:CAPY_CLAUDE_INTERVENTION_ROOT = ${psquote(root)}\n& ${psquote(process.execPath)} '-e' ${psquote(ttyProbe)} ${psquote(path.join(root,'tty-probe.json'))}\nif ($LASTEXITCODE -ne 0) { throw 'Claude requires a real TTY' }\n& ${psquote(claude)} ${argv.map(psquote).join(' ')}\nexit $LASTEXITCODE\n`);
  report.claudeArgv=argv;
  report.terminalEvidence='Awaiting orchestrator real-TTY capture and observed assistant continuation; helper does not infer continuation from hooks alone';
  fs.writeFileSync(path.join(root,'interactive-ready.json'),JSON.stringify({root,sessionId,settings,launch,argv,decision:report.permissionDecision},null,2));
  console.log(JSON.stringify({interactiveReady:true,root,sessionId,launch,command:`powershell -NoProfile -ExecutionPolicy Bypass -File "${launch}"`,requiresTty:true}));
  const permission=await waitFor(()=>ownRequest('claudePermission'),'Own interactive Claude native Write permission card',180000);
  assert.equal(permission.request.body.toolName,'Write');
  assert.equal(path.resolve(permission.request.body.toolInput.file_path),path.join(root,'interactive-proof.txt'));
  fs.writeFileSync(path.join(root,'permission.json'),JSON.stringify(permission.request,null,2));
  fs.writeFileSync(path.join(root,'pending-snapshot.json'),JSON.stringify(await panel.invoke('demo_snapshot'),null,2));
  await panel.screenshot('permission');
  check('real session Write visible in native app',true);
  await click(`${permission.selector} [data-decision="${allowWrite?'allow':'deny'}"]`);
  check(`native interactive permission ${allowWrite?'allow':'deny'} clicked`,true);
  const capture=await waitFor(()=>{
    for(const file of fs.readdirSync(root).filter(name=>name.endsWith('.stdin.raw'))) {
      const base=path.join(root,file.slice(0,-'.stdin.raw'.length));
      const input=JSON.parse(fs.readFileSync(base+'.stdin.raw','utf8'));
      if(input.session_id!==sessionId||input.hook_event_name!=='PermissionRequest')continue;
      if(!fs.existsSync(base+'.exit.json'))continue;
      return {base,input,output:JSON.parse(fs.readFileSync(base+'.stdout.raw','utf8')),exit:JSON.parse(fs.readFileSync(base+'.exit.json','utf8'))};
    }
    return null;
  },'Raw native hook response and exit',20000);
  assert.equal(capture.exit.code,0);
  assert.equal(capture.output.hookSpecificOutput.hookEventName,'PermissionRequest');
  assert.equal(capture.output.hookSpecificOutput.decision.behavior,allowWrite?'allow':'deny');
  report.hookCapture=capture;
  check('actual native hook stdout carries selected decision',true);
  if(allowWrite)await waitFor(()=>fs.existsSync(path.join(root,'interactive-proof.txt')),'Allowed interactive file',20000);
  check('interactive file matches native decision',allowWrite?fs.readFileSync(path.join(root,'interactive-proof.txt'),'utf8')==='capy-interactive-proof':!fs.existsSync(path.join(root,'interactive-proof.txt')));
  const sessionTrace=await waitFor(()=>{
    const projects=path.join(config,'projects');
    if(!fs.existsSync(projects))return null;
    for(const entry of fs.readdirSync(projects,{withFileTypes:true})) {
      if(!entry.isDirectory()||entry.isSymbolicLink())continue;
      const trace=path.join(projects,entry.name,`${sessionId}.jsonl`);
      if(!fs.existsSync(trace))continue;
      regular(trace);
      const raw=fs.readFileSync(trace,'utf8');
      const rows=raw.split('\n').filter(Boolean).flatMap(line=>{try{return [JSON.parse(line)];}catch{return [];}});
      let writeId, resultIndex=-1;
      rows.forEach((row,index)=>{
        for(const part of Array.isArray(row.message?.content)?row.message.content:[]) {
          if(part.type==='tool_use'&&part.name==='Write')writeId=part.id;
          if(writeId&&part.type==='tool_result'&&part.tool_use_id===writeId)resultIndex=index;
        }
      });
      if(resultIndex<0)return null;
      const continuation=rows.slice(resultIndex+1).find(row=>row.type==='assistant'&&row.message?.content?.some(part=>part.type==='text'&&part.text.includes('INTERACTIVE_FINISHED')));
      if(!continuation)return null;
      return {source:trace,raw,resultIndex,continuation};
    }
    return null;
  },'Own persisted interactive tool result and assistant continuation',30000);
  fs.writeFileSync(path.join(root,'claude-session.jsonl'),sessionTrace.raw);
  report.sessionTrace={source:sessionTrace.source,captured:path.join(root,'claude-session.jsonl'),resultIndex:sessionTrace.resultIndex,continuation:sessionTrace.continuation};
  check('own session transcript contains assistant completion after Write tool result',true);
  console.log(JSON.stringify({nativeResponseComplete:true,root,waitingFor:'terminal-proof.json with tty:true, assistantContinued:true, transcriptFile absolute path'}));
  const terminal=await waitFor(()=>fs.existsSync(path.join(root,'terminal-proof.json'))&&JSON.parse(fs.readFileSync(path.join(root,'terminal-proof.json'),'utf8')),'Orchestrator terminal evidence',90000);
  assert.equal(terminal.tty,true);assert.equal(terminal.assistantContinued,true);
  assert.ok(path.isAbsolute(terminal.transcriptFile));regular(terminal.transcriptFile);
  assert.ok(fs.statSync(terminal.transcriptFile).size>0);
  report.terminalProof=terminal;
  report.terminalEvidence='Real TTY and post-decision assistant continuation observed by orchestrator; transcript preserved at terminalProof.transcriptFile';
  check('orchestrator observed real interactive TTY and assistant continuation',true);
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
