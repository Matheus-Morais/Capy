import assert from 'node:assert/strict';
import {spawn,execFile} from 'node:child_process';
import {promisify,isDeepStrictEqual} from 'node:util';
import {randomUUID} from 'node:crypto';
import {mkdir, writeFile, readFile, stat,readdir} from 'node:fs/promises';
import {createServer} from 'node:net';
import {dirname, join, resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {setTimeout as delay} from 'node:timers/promises';
import {verifyLiveTaskControls} from './verify-live-task-controls.mjs';
import {verifyLiveTaskModel} from './verify-live-task-model.mjs';
import {externalTaskProof} from './verify-live-task-external.mjs';
import {instructionProof} from './verify-live-task-guides.mjs';
import {taskHandoffProof} from './verify-live-task-handoff.mjs';

const workspace=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const root=join(workspace,'scratch',`capy-visual-${randomUUID()}`);
const exe=join(workspace,'src-tauri','target','release','capy.exe');
await stat(exe);await mkdir(root,{recursive:true});
let exitFixture;
const automaticChatApiReview=process.argv.includes('--automatic-chat-api-review');
const agyAutomatic=process.argv.includes('--agy-auto');
const quotaDashboard=process.argv.includes('--quota-dashboard');
assert.ok(!(quotaDashboard&&agyAutomatic)&&(!(quotaDashboard||agyAutomatic)||process.argv.length===3),'Quota dashboard/automatic agy proof runs separately, with read-only quota sources');
const automaticChatReview=process.argv.includes('--automatic-chat-review')||automaticChatApiReview;
assert.ok(!automaticChatReview||!process.argv.some(arg=>arg.startsWith('--live-')||['--exit-review','--profiles-corrupt','--quota-settings'].includes(arg)),'Automatic chat UI proof uses only synthetic own records, without provider calls');
let automaticFixture;
if(automaticChatReview){
  const target={kind:'claudeCli',profileId:'own-quota-fixture',provider:'Claude',account:'Own synthetic source',billing:'subscription',credentialRevision:null};
  const sendNonce=randomUUID();
  automaticFixture={id:randomUUID(),title:'Own synthetic quota chat',target,model:'haiku',messages:[{role:'user',text:'Synthetic UI fixture; no provider dispatched.'},{role:'assistant',text:'Synthetic stored result for UI proof.'}],revision:2,state:'completed',activeNonce:null,usedNonces:[sendNonce],lastError:null,cliStarted:false,cliAttempted:false,processPolicy:null,recoveryReview:null,interruptions:[],transferredTo:null};
  await mkdir(join(root,'chat'));await writeFile(join(root,'chat','index.json'),JSON.stringify([automaticFixture.id]));
  await writeFile(join(root,'chat',`${automaticFixture.id}.json`),JSON.stringify(automaticFixture));
}
const liveTaskControls=process.argv.includes('--live-task-controls');
const liveTaskModel=process.argv.includes('--live-task-model');
const liveTaskGuides=process.argv.includes('--live-task-guides');
const liveTaskHandoff=process.argv.includes('--live-task-handoff');
assert.ok([liveTaskControls,liveTaskModel,liveTaskGuides,liveTaskHandoff].filter(Boolean).length<=1,'Model, guides, handoff and interruption proofs run separately');
const liveChatGuides=process.argv.includes('--live-chat-guides');
assert.ok(!liveChatGuides||!process.argv.some(arg=>['--live-chat','--live-transfer','--live-recovery','--live-task','--live-task-controls','--live-task-model','--live-task-guides','--live-task-handoff','--live-task-external'].includes(arg)),'Chat instruction capture runs separately with one short subscription call');
const liveTaskMode=process.argv.includes('--live-task')||liveTaskControls||liveTaskModel||liveTaskGuides||liveTaskHandoff;
const liveTaskExternal=process.argv.includes('--live-task-external');
assert.ok(!(liveTaskExternal&&(liveTaskMode||process.argv.some(arg=>['--live-chat','--live-transfer','--live-recovery','--exit-review','--profiles-corrupt'].includes(arg)))),'External task proof runs separately');
let externalProof;
const corruptProfiles=process.argv.includes('--profiles-corrupt');
const quotaSettings=process.argv.includes('--quota-settings');
assert.ok(!(quotaSettings&&(liveTaskMode||liveTaskExternal||corruptProfiles||process.argv.some(arg=>['--live-chat','--live-transfer','--live-recovery','--exit-review'].includes(arg)))),'Quota settings proof runs without provider calls');
let quotaProfile,quotaOriginal;
if(quotaSettings){
  const configDir=join(root,'own-claude-config');await mkdir(configDir);
  quotaProfile={id:'own-quota-settings',label:'Own quota settings fixture',provider:'Claude',configDir,billing:'subscription'};
  quotaOriginal={model:'haiku',statusLine:{type:'command',command:'echo capy_ação_日本語_🦫',padding:2,refreshInterval:10},hooks:{Stop:[]},ownFutureValue:{preserve:'ação_日本語'}};
  await writeFile(join(configDir,'settings.json'),JSON.stringify(quotaOriginal));
  await writeFile(join(root,'profiles.json'),JSON.stringify([quotaProfile]));
}
const incompatibleProfiles='[{"futureProfileVersion":2,"opaque":"preserve exact bytes"}]\r\n';
if(corruptProfiles){
  assert.ok(!liveTaskMode&&!process.argv.some(arg=>['--live-chat','--live-transfer','--live-recovery','--exit-review'].includes(arg)),'Incompatible profiles proof must run without provider calls');
  await writeFile(join(root,'profiles.json'),incompatibleProfiles);
}
let liveTask,liveTaskProfile,liveTaskProcess,liveTaskPanel,liveTaskHandoffTask,liveTaskHandoffProcess;
if(liveTaskMode)assert.ok(!process.argv.some(arg=>['--live-chat','--live-transfer','--live-recovery','--exit-review'].includes(arg)),'Interactive task proof must run separately');
if(process.argv.includes('--exit-review')){
  assert.ok(!process.argv.some(arg=>['--live-chat','--live-transfer','--live-recovery'].includes(arg)),'Exit state fixture must run separately from provider calls');
  exitFixture={id:randomUUID(),title:'Own exit state fixture',target:{kind:'api',profileId:randomUUID(),provider:'OpenAI',account:'Own exit fixture',billing:'api',credentialRevision:randomUUID()},model:'fixture-model',messages:[],revision:0,state:'idle',activeNonce:null,usedNonces:[],lastError:null,cliStarted:false,cliAttempted:false,processPolicy:null,recoveryReview:null,interruptions:[],transferredTo:null};
}
const server=createServer();await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
let port=server.address().port;await new Promise(resolve=>server.close(resolve));
let child=spawn(exe,['--visual-test',root],{
  windowsHide:true,stdio:'ignore',env:{...process.env,
    ...((liveTaskMode||liveTaskExternal)?{CAPY_VISUAL_TEST_NO_TOOLS:'1'}:{}),
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:`--remote-debugging-port=${port} --remote-debugging-address=127.0.0.1`,
    WEBVIEW2_USER_DATA_FOLDER:join(root,'webview'),
  },
});
let childExited=false;let launchError;
child.on('exit',()=>childExited=true);child.on('error',error=>{launchError=error;childExited=true;});
const connections=[];const checks=[];
const check=(name,condition)=>{checks.push({name,passed:!!condition});assert.ok(condition,name);};

async function targets(){
  if(childExited)throw launchError??new Error('A Capy de teste encerrou; feche a outra instância antes desta prova.');
  const response=await fetch(`http://127.0.0.1:${port}/json/list`,{signal:AbortSignal.timeout(1000)});
  return response.json();
}
async function waitFor(fn,description,timeout=10_000){
  const started=Date.now();let last;
  while(Date.now()-started<timeout){
    try{const value=await fn();if(value)return value;}catch(error){last=error;}
    if(childExited)throw launchError??new Error(`A Capy encerrou durante: ${description}`);
    await delay(100);
  }
  throw new Error(`${description}: ${last?.message??'tempo excedido'}`);
}
async function connect(target){
  const socket=new WebSocket(target.webSocketDebuggerUrl);let sequence=0;const pending=new Map();const dialogs=[];
  await new Promise((resolve,reject)=>{socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true});});
  socket.addEventListener('message',event=>{
    const message=JSON.parse(event.data);
    if(message.method==='Page.javascriptDialogOpening')dialogs.push(message.params);
    const request=pending.get(message.id);if(!request)return;
    pending.delete(message.id);clearTimeout(request.timer);
    if(message.error)request.reject(new Error(message.error.message));else request.resolve(message.result);
  });
  socket.addEventListener('close',()=>{for(const request of pending.values()){clearTimeout(request.timer);request.reject(new Error('CDP encerrou'));}pending.clear();});
  const call=(method,params={})=>new Promise((resolve,reject)=>{
    const id=++sequence;const timer=setTimeout(()=>{pending.delete(id);reject(new Error(`CDP ${method}: tempo excedido`));},10_000);
    pending.set(id,{resolve,reject,timer});socket.send(JSON.stringify({id,method,params}));
  });
  const evaluate=async expression=>{
    const result=await call('Runtime.evaluate',{expression,awaitPromise:true,returnByValue:true});
    if(result.exceptionDetails)throw new Error(result.exceptionDetails.text+' '+(result.exceptionDetails.exception?.description??result.exceptionDetails.exception?.value??''));
    return result.result.value;
  };
  const invoke=(command,args={})=>evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)})`);
  const screenshot=async name=>{
    const result=await call('Page.captureScreenshot',{format:'png',captureBeyondViewport:false});
    await writeFile(join(root,`${name}.png`),Buffer.from(result.data,'base64'));
  };
  const connection={call,evaluate,invoke,screenshot,takeDialog:()=>dialogs.shift(),close:()=>socket.close()};connections.push(connection);return connection;
}
const ownTarget=(values,path)=>values.find(target=>target.type==='page'
  && /^https?:\/\/tauri\.localhost\//.test(target.url)
  && new URL(target.url).pathname===path);
const sameFolder=(a,b)=>a.replace(/^\\\\\?\\/,'').replaceAll('\\','/').toLowerCase()===b.replace(/^\\\\\?\\/,'').replaceAll('\\','/').toLowerCase();
async function taskHistory(task,profile){
  const projects=join(profile.configDir,'projects');const dirs=await readdir(projects,{withFileTypes:true});assert.ok(dirs.length<=2048);
  let found;
  for(const dir of dirs.filter(dir=>dir.isDirectory())){
    const path=join(projects,dir.name,`${task.id}.jsonl`);let info;
    try{info=await stat(path);}catch(error){if(error.code==='ENOENT')continue;throw error;}
    assert.ok(info.size<=2_097_152);assert.ok(!found,'Own UUID appears in only one project');
    const rows=(await readFile(path,'utf8')).split('\n').filter(Boolean).map(line=>{try{return JSON.parse(line);}catch{return null;}}).filter(Boolean);
    found={path,rows};
  }
  return found;
}
async function processInfo(pid){
  assert.ok(Number.isInteger(pid)&&pid>0&&pid<=0xffffffff);
  const {stdout}=await promisify(execFile)('powershell.exe',['-NoProfile','-Command',`$ErrorActionPreference='Stop'; for($attempt=0;$attempt -lt 4;$attempt++){ $p=Get-Process -Id ${pid} -ErrorAction SilentlyContinue; if(-not $p -or $p.HasExited){'null';exit}; $started=$p.StartTime; $path=$p.Path; if($started -and $path){[pscustomobject]@{pid=$p.Id;started=$started.ToUniversalTime().ToString('o');path=$path}|ConvertTo-Json -Compress;exit}; Start-Sleep -Milliseconds 25 }; throw 'Identidade do processo indisponível.'`],{windowsHide:true,timeout:10_000,maxBuffer:4096});
  return JSON.parse(stdout.trim());
}
async function taskProcess(task,profile){
  const folder=join(profile.configDir,'sessions');const entries=await readdir(folder);assert.ok(entries.length<=4096);
  for(const name of entries.filter(name=>name.endsWith('.json'))){
    const path=join(folder,name);let value;
    try{assert.ok((await stat(path)).size<=65_536);value=JSON.parse(await readFile(path,'utf8'));}catch(error){if(error.code==='ENOENT')continue;throw error;}
    if(value.sessionId===task.id&&sameFolder(value.cwd,task.cwd))return processInfo(value.pid);
  }
  return null;
}

let failure;
try{
  const petTarget=await waitFor(async()=>ownTarget(await targets(),'/index.html')??ownTarget(await targets(),'/'),'Mascote nativa/CDP');
  let pet=await connect(petTarget);
  await pet.call('Emulation.setEmulatedMedia',{features:[{name:'prefers-reduced-motion',value:'no-preference'}]});
  await waitFor(()=>pet.evaluate(`document.querySelector('#pet') instanceof SVGSVGElement && !!window.__TAURI_INTERNALS__`),'SVG carregado');
  let panelShown=false,lastPanelError;
  for(let attempt=0;attempt<30&&!panelShown;attempt++){
    try{await pet.invoke('show_panel');panelShown=true;}
    catch(error){lastPanelError=error;await delay(100);}
  }
  assert.ok(panelShown,`Painel nativo não ficou disponível: ${lastPanelError?.message??'sem resposta'}`);
  const panelTarget=await waitFor(async()=>ownTarget(await targets(),'/panel.html'),'Painel nativo/CDP');
  let panel=await connect(panelTarget);
  await waitFor(()=>panel.evaluate(`!!document.querySelector('#startTask') && !!document.querySelector('#accountList')`),'Controles do painel');
  check('native_panel_no_horizontal_overflow',await panel.evaluate('document.documentElement.scrollWidth<=innerWidth'));
  await panel.screenshot('panel');
  check('native_quota_board_precedes_session_grid',await panel.evaluate(`document.querySelector('.quotas').compareDocumentPosition(document.querySelector('.panel-grid')) & Node.DOCUMENT_POSITION_FOLLOWING`));
  check('native_quota_board_groups_demo_accounts',await panel.evaluate(`document.querySelectorAll('.quota-account').length===3 && document.querySelectorAll('.quota-dial[role="meter"]').length===6 && document.querySelector('.simulation').textContent.includes('simulad')`));
  check('native_quota_board_shows_window_countdowns',await panel.evaluate(`document.querySelector('#quotaRows').textContent.includes('Semanal') && document.querySelector('#quotaRows').textContent.includes('Renova em')`));
  await panel.evaluate(`document.querySelector('[data-quota-section="accounts"]').click()`);
  check('native_quota_accounts_shortcut_focuses_account_controls',await panel.evaluate(`document.querySelector('#accounts').contains(document.activeElement)`));
  await panel.evaluate(`document.querySelector('[data-quota-section="quotaPreferences"]').click()`);
  check('native_quota_alerts_shortcut_focuses_preferences',await panel.evaluate(`document.querySelector('#quotaPreferences')===document.activeElement || document.querySelector('#quotaPreferences').contains(document.activeElement)`));
  await panel.evaluate(`document.querySelector('[data-quota-section="chatCreateForm"]').click()`);
  check('native_quota_switch_shortcut_opens_and_focuses_destination',await panel.evaluate(`document.querySelector('#chatCreateForm').closest('details').open && document.querySelector('#chatCreateForm').contains(document.activeElement)`));
  await panel.evaluate(`window.scrollTo(0,0)`);
  if(quotaDashboard||agyAutomatic){
    const realStarted=Date.now();
    await panel.invoke('demo_action',{action:'scenario',id:'',answer:'real'});
    let data=await waitFor(async()=>{const data=await panel.invoke('demo_snapshot');return data.quotas?.some(q=>q.provider==='Antigravity')?data:null;},'Fontes reais de cota aparecem',30_000);
    if(agyAutomatic){
      const started=Date.now();await panel.evaluate(`document.querySelector('[data-quota-section="accounts"]').click()`);
      check('native_agy_query_keeps_account_navigation_responsive',Date.now()-started<5000&&await panel.evaluate(`document.querySelector('#accounts').contains(document.activeElement)`));
      const observed=async(minimum)=>{
        const value=await panel.invoke('demo_snapshot');const rows=value.quotas?.filter(q=>q.provider==='Antigravity')??[];
        return rows.length===4&&rows.every(q=>q.account&&q.observedAt>minimum)&&rows.some(q=>q.state==='fresh'&&q.window&&q.message.includes('Consulta automática oficial agy /usage'))?value:null;
      };
      data=await waitFor(()=>observed(realStarted-1),'Consulta automática inicial agy identificada',60_000);
      const first=data.quotas.find(q=>q.provider==='Antigravity').observedAt;
      check('native_agy_initial_query_refreshes_identified_windows_without_manual_terminal',true);
      data=await waitFor(()=>observed(first),'Segunda consulta periódica agy sem botão nem terminal',90_000);
      check('native_agy_periodic_query_renews_observation_without_manual_action',true);
      await waitFor(()=>panel.evaluate(`document.querySelector('#quotaRows').textContent.includes('Antigravity')&&[...document.querySelectorAll('#quotaRows [role="meter"]')].some(m=>m.getAttribute('aria-label').startsWith('Antigravity'))`),'Medidores reais agy visíveis');
      await panel.evaluate(`window.scrollTo(0,0)`);
    }
    await waitFor(()=>panel.evaluate(`document.querySelector('#refreshQuotas').hidden===false`),'Atualização de fontes reais visível');
    check('native_quota_real_board_never_keeps_demo_accounts',await panel.evaluate(`!document.querySelector('#quotaRows').textContent.includes('Conta pessoal')&&!document.querySelector('#quotaRows').textContent.includes('Conta trabalho')`));
    check('native_quota_real_board_includes_antigravity_source',await panel.evaluate(`document.querySelector('#quotaRows').textContent.includes('Antigravity')`));
    const fresh=data.quotas.filter(q=>q.state==='fresh'&&q.account&&q.window&&q.observedAt!==null&&Date.now()-q.observedAt<=120000&&q.window.resetsAt*1000>Date.now());
    check('native_quota_real_meters_match_observed_backend_balances',await panel.evaluate(`(()=>{const meters=[...document.querySelectorAll('#quotaRows [role="meter"]')];const rows=${JSON.stringify(fresh)};return meters.length===rows.length&&rows.every(row=>meters.some(meter=>meter.getAttribute('aria-label').includes(row.account)&&Number(meter.getAttribute('aria-valuenow'))===Number((100-row.window.usedPercent).toFixed(1))));})()`));
    await writeFile(join(root,'quota-snapshot.json'),JSON.stringify(data.quotas,null,2));
    await panel.screenshot('quota-real');
    await panel.call('Emulation.setDeviceMetricsOverride',{width:540,height:680,deviceScaleFactor:1,mobile:false});
    check('native_quota_real_board_at_minimum_panel_width_has_no_overflow',await panel.evaluate('document.documentElement.scrollWidth<=innerWidth'));
    await panel.screenshot('quota-real-compact');await panel.call('Emulation.clearDeviceMetricsOverride');
    await panel.invoke('refresh_quotas');
    check('native_quota_refresh_limits_repeated_requests',await panel.invoke('refresh_quotas').then(()=>false,error=>String(error).includes('15 segundos')));
    await panel.invoke('demo_action',{action:'scenario',id:'',answer:'waiting'});
    await waitFor(()=>panel.evaluate(`document.querySelectorAll('.quota-account').length===3`),'Volta à demonstração explicitamente identificada');
    check('native_quota_real_alerts_do_not_leak_into_simulation',await panel.evaluate(`document.querySelector('#quotaAlerts').hidden && !document.querySelector('#quotaAlerts').textContent`));
  }
  await waitFor(()=>panel.evaluate(`!!document.querySelector('#chatCreateForm') && !!document.querySelector('#chatSendForm')`),'Controles de chat');
  if(automaticFixture){const rows=await panel.invoke('list_chats');check('native_automatic_chat_own_synthetic_record_loaded',rows.length===1&&rows[0].id===automaticFixture.id&&rows[0].revision===2);}
  else check('native_chat_commands_start_with_empty_own_history',(await panel.invoke('list_chats')).length===0);
  check('native_chat_create_requires_account_verification',await panel.evaluate(`document.querySelector('#chatCreateForm button[type="submit"]').disabled`));
  check('native_chat_key_is_password_without_autocomplete',await panel.evaluate(`(()=>{const key=document.querySelector('#chatApiForm input[name="key"]');return key.type==='password' && key.autocomplete==='off' && key.value==='';})()`));
  await panel.evaluate(`document.querySelector('.chat-workbench > details').open=true;document.querySelector('#chatCreateForm').closest('details').open=true;document.querySelector('.chat-workbench').scrollIntoView({block:'start'})`);
  check('native_chat_no_horizontal_overflow',await panel.evaluate('document.documentElement.scrollWidth<=innerWidth'));
  await panel.screenshot('chat');
  if(automaticFixture){
    await panel.invoke('demo_action',{action:'scenario',id:'',answer:'real'});
    await waitFor(async()=>(await panel.invoke('demo_snapshot')).sessions.some(session=>session.id===`chat:${automaticFixture.id}`),'Monitor registra a origem sintética própria');
    await panel.invoke('open_source',{id:`chat:${automaticFixture.id}`});
    await waitFor(()=>panel.evaluate(`document.querySelector('#chatSelection').value===${JSON.stringify(automaticFixture.id)} && !document.querySelector('#chatReview form')`),'Seleção exata antes da revisão automática');
    const summary={objective:'Own synthetic quota objective',decisions:'Review this local UI fixture.',state:'Synthetic source record. No provider was called.',files:'No files changed by a provider.',tests:'Synthetic UI proof only.',nextSteps:'Cancel without sending.',guides:'No provider instructions were loaded in this fixture.'};
    const destination=automaticChatApiReview?{kind:'api',profileId:randomUUID(),provider:'OpenAI',account:'Own synthetic destination',billing:'api',credentialRevision:randomUUID()}:{...automaticFixture.target,profileId:'own-destination-fixture',account:'Own synthetic destination'};
    const review={nonce:randomUUID(),sourceId:automaticFixture.id,sourceRevision:automaticFixture.revision,sourceTarget:automaticFixture.target,destination,model:automaticChatApiReview?'own-model':'sonnet',summary,uncertainMessages:[],automatic:true,dismissed:false};
    const reviewPath=join(root,'chat',`${automaticFixture.id}.review.json`);await writeFile(reviewPath,JSON.stringify(review));
    await waitFor(()=>panel.evaluate(`document.querySelector('#chatReview form')?.dataset.chatReview===${JSON.stringify(review.nonce)}`),'Revisão adicionada pelo snapshot sem mudança do histórico',15_000);
    check('native_automatic_chat_review_visible_without_history_revision_change',(await panel.invoke('list_chats'))[0].revision===2&&await panel.evaluate(`document.querySelector('#chatReview h2').textContent==='Percentual de troca atingido'`));
    check('native_automatic_chat_review_names_destination_and_requires_fresh_consent',await panel.evaluate(`(()=>{const view=document.querySelector('#chatReview'),consent=view.querySelector('[name="reviewed"]');return view.textContent.includes('Own synthetic destination')&&view.textContent.includes(${JSON.stringify(review.model)})&&consent.required&&!consent.checked;})()`));
    if(automaticChatApiReview)check('native_automatic_chat_api_review_requires_explicit_billing_change',await panel.evaluate(`(()=>{const view=document.querySelector('#chatReview'),consent=view.querySelector('[name="billingConfirmed"]');return view.textContent.includes('OpenAI')&&view.textContent.includes('cobrança por uso')&&consent.required&&!consent.checked;})()`));
    const draft=`OWN_EDITED_QUOTA_SUMMARY_${randomUUID()}`;await panel.evaluate(`document.querySelector('#chatReview textarea[name="objective"]').value=${JSON.stringify(draft)}`);
    await panel.invoke('save_preferences',{value:{sounds:true,reduceMotion:false,quotaRules:[]}});
    await waitFor(()=>panel.evaluate(`document.querySelector('#petSounds').checked`),'Snapshot repetido processado pela interface');
    check('native_automatic_chat_same_review_preserves_edited_summary',await panel.evaluate(`document.querySelector('#chatReview textarea[name="objective"]').value===${JSON.stringify(draft)}`));
    await panel.invoke('save_preferences',{value:{sounds:false,reduceMotion:false,quotaRules:[]}});
    await panel.evaluate(`document.querySelector('#chatReview').scrollIntoView({block:'start'})`);
    await panel.screenshot('automatic-chat-review');
    await panel.evaluate(`document.querySelector('[data-cancel-chat-transfer]').click()`);
    await waitFor(()=>panel.evaluate(`document.querySelector('#chatReview').hidden && document.querySelector('#chatNotice').textContent.includes('cancelada')`),'Cancelamento nativo da revisão');
    const canceled=JSON.parse(await readFile(reviewPath,'utf8'));const rows=await panel.invoke('list_chats');
    check('native_automatic_chat_cancel_persists_tombstone_and_never_creates_destination',canceled.dismissed===true&&rows.length===1&&rows[0].revision===2&&rows[0].messages.length===2&&rows[0].activeNonce===null);
    check('native_automatic_chat_canceled_review_returns_none',await panel.invoke('chat_transfer_review',{sourceId:automaticFixture.id})===null);
    check('native_automatic_chat_canceled_approval_never_dispatches',await panel.evaluate(`window.__TAURI_INTERNALS__.invoke('approve_chat_transfer',${JSON.stringify({sourceId:automaticFixture.id,nonce:review.nonce,summary,reviewed:true,billingConfirmed:false})}).then(()=>false,error=>String(error).includes('expirou'))`));
  }
  if(corruptProfiles){
    await waitFor(()=>panel.evaluate(`document.querySelector('#accountList').textContent.includes('preservado')`),'Erro de perfis visível');
    check('native_profiles_incompatible_bytes_preserved',await readFile(join(root,'profiles.json'),'utf8')===incompatibleProfiles);
    check('native_profiles_add_controls_disabled',await panel.evaluate(`Array.from(document.querySelectorAll('#addAccount input,#addAccount button,#addAccount select')).every(control=>control.disabled)`));
    const rejected=await panel.evaluate(`window.__TAURI_INTERNALS__.invoke('add_profile',{label:'Own blocked fixture',existing:null,billing:'subscription'}).then(()=>false,error=>String(error).includes('preservado'))`);
    check('native_profiles_backend_rejects_add_without_overwrite',rejected&&await readFile(join(root,'profiles.json'),'utf8')===incompatibleProfiles);
    check('native_profiles_failed_add_creates_no_login_directory',!(await readdir(root)).includes('accounts'));
    check('native_profiles_api_controls_remain_available',await panel.evaluate(`!document.querySelector('#chatApiForm button').disabled && !document.querySelector('#chatApiForm input[name="key"]').disabled`));
    check('native_profiles_chat_source_warning_visible',await panel.evaluate(`Array.from(document.querySelectorAll('.chat-workbench [role="status"]')).some(node=>node.textContent.includes('preservado'))`));
    check('native_profiles_tasks_and_history_remain_readable',(await panel.invoke('list_tasks')).length===0&&(await panel.invoke('list_chats')).length===0);
    check('native_profiles_exit_review_remains_available',(await panel.invoke('prepare_exit_review')).resources.length===0);
    await panel.screenshot('profiles-corrupt');
  }
  if(liveTaskMode){
    liveTaskPanel=panel;
    const project=join(root,'own-task-project');await mkdir(project);
    const marker=`CAPY_TASK_${randomUUID()}`;const instruction=`Responda somente com o texto literal: ${marker}.`;
    const profiles=await panel.invoke('list_profiles');liveTaskProfile=profiles.find(profile=>profile.id==='claude-default');assert.ok(liveTaskProfile);
    const guideProof=liveTaskGuides?instructionProof({panel,root,project,profile:liveTaskProfile,check,waitFor,sameFolder,taskHistory}):null;
    await guideProof?.prepare();
    const identity=await panel.invoke('profile_identity',{id:liveTaskProfile.id,cwd:project});
    check('native_task_pins_subscription_before_launch',identity.loggedIn&&identity.billing==='subscription'&&!!identity.account);
    await panel.evaluate(`(()=>{document.querySelector('#newTask').open=true;const form=document.querySelector('#startTask');form.elements.namedItem('profile').value='claude-default';form.elements.namedItem('cwd').value=${JSON.stringify(project)};form.elements.namedItem('cwd').dispatchEvent(new Event('input',{bubbles:true}));form.elements.namedItem('model').value='haiku';form.elements.namedItem('mode').value='embedded';form.elements.namedItem('prompt').value=${JSON.stringify(instruction)};document.querySelector('#verifyTaskAccount').click();})()`);
    await waitFor(()=>panel.evaluate(`!document.querySelector('#startTask button[type="submit"]').disabled && document.querySelector('#taskBilling').textContent.includes('Assinatura Claude')`),'Conta do formulário de tarefa verificada',20_000);
    await panel.evaluate(`document.querySelector('#startTask').requestSubmit();true`);
    liveTask=await waitFor(async()=>{const tasks=await panel.invoke('list_tasks');return tasks.find(task=>sameFolder(task.cwd,project));},'Tarefa própria registrada',25_000);
    check('native_task_form_keeps_exact_folder_account_model_prompt_and_mode',liveTask.profileId===liveTaskProfile.id&&liveTask.account===identity.account&&liveTask.billing==='subscription'&&liveTask.model==='haiku'&&liveTask.prompt===instruction&&liveTask.mode==='embedded');
    await waitFor(()=>panel.evaluate(`!document.querySelector('#terminalSection').hidden && document.querySelector('#terminalIdentity').textContent.includes(${JSON.stringify(liveTask.id)})`),'Terminal anexado ao UUID próprio');
    check('native_task_terminal_identity_matches_created_task',await panel.evaluate(`document.querySelector('#terminalIdentity').textContent.includes(${JSON.stringify(identity.account)}) && document.querySelector('#terminalTitle').textContent.includes('haiku')`));
    await writeFile(join(root,'task-initial-geometry.json'),JSON.stringify(await panel.evaluate(`({section:document.querySelector('#terminalSection').getBoundingClientRect().toJSON(),viewport:document.querySelector('#terminalViewport').getBoundingClientRect().toJSON(),height:innerHeight})`),null,2));
    check('native_task_terminal_is_visible_without_manual_scrolling',await panel.evaluate(`(()=>{const section=document.querySelector('#terminalSection').getBoundingClientRect();const viewport=document.querySelector('#terminalViewport').getBoundingClientRect();return section.top>=0&&section.top<50&&viewport.height>0&&viewport.bottom<=innerHeight;})()`));
    const screen=()=>panel.evaluate(`document.querySelector('.xterm-rows')?.textContent??''`);let trusted=false;
    const history=await waitFor(async()=>{
      const text=await screen();await writeFile(join(root,'task-terminal-screen.txt'),text);
      if(!trusted&&/Yes,\s*I\s*trust\s*this\s*folder/i.test(text)){
        if(/❯\s*(?:\d+[.)]\s*)?No,?\s*exit/i.test(text)){await panel.invoke('terminal_input',{id:liveTask.id,data:'\u001b[B'});return null;}
        if(/❯\s*(?:\d+[.)]\s*)?Yes,\s*I\s*trust\s*this\s*folder/i.test(text)){await panel.invoke('terminal_input',{id:liveTask.id,data:'\r'});trusted=true;}
      }
      const history=await taskHistory(liveTask,liveTaskProfile);
      const response=history?.rows.find(row=>row.type==='assistant'&&row.sessionId===liveTask.id&&row.message?.content?.some(part=>part.type==='text'&&part.text.trim()===marker));
      return response?{...history,response}:null;
    },'Resposta real no histórico do UUID exato',60_000);
    const user=history.rows.find(row=>row.type==='user'&&row.sessionId===liveTask.id&&row.isSidechain!==true);
    check('native_task_provider_receipt_confirms_exact_uuid_folder_instruction_and_haiku',sameFolder(user.cwd,liveTask.cwd)&&JSON.stringify(user.message.content).includes(instruction)&&history.response.message.model.toLowerCase().includes('haiku'));
    check('native_task_short_proof_does_not_execute_tools',!history.rows.some(row=>row.type==='assistant'&&row.message?.content?.some(part=>part.type==='tool_use')));
    liveTaskProcess=await waitFor(()=>taskProcess(liveTask,liveTaskProfile),'Processo oficial da sessão própria');
    check('native_task_has_live_official_claude_process',liveTaskProcess.path.toLowerCase().endsWith('claude.exe'));
    await panel.evaluate(`document.querySelector('#terminalSection').scrollIntoView({block:'start'});true`);await panel.screenshot('task-terminal');
    check('native_task_terminal_no_horizontal_overflow',await panel.evaluate('document.documentElement.scrollWidth<=innerWidth'));
    await guideProof?.verify(liveTask);
    if(liveTaskHandoff){
      const handoffProof=taskHandoffProof({panel,root,task:liveTask,profile:liveTaskProfile,check,waitFor,taskHistory,taskProcess,sameFolder});
      liveTaskHandoffTask=await handoffProof.verify();
      liveTaskHandoffProcess=await waitFor(()=>taskProcess(liveTaskHandoffTask,liveTaskProfile),'Processo oficial da continuação integrada');
      check('native_handoff_destination_has_live_official_claude_process',liveTaskHandoffProcess.path.toLowerCase().endsWith('claude.exe'));
    }
    await panel.evaluate(`document.querySelector('#terminalSection').scrollIntoView({block:'start'});true`);
    const modelControlTask=liveTaskHandoffTask??liveTask;
    await panel.evaluate(`document.querySelector('#terminalModel').click();true`);
    await waitFor(async()=>/Select.*model|Selecion.*modelo/i.test(await screen()),'Seletor oficial de modelo aberto');
    check('native_task_model_control_opens_official_picker',true);await panel.screenshot('task-model-picker');
    await panel.invoke('terminal_input',{id:modelControlTask.id,data:'\u001b'});
    await waitFor(async()=>!/Select.*model|Selecion.*modelo/i.test(await screen()),'Seletor fecha sem novo envio');
    await writeFile(join(root,'task-receipt.json'),JSON.stringify({task:liveTask,process:liveTaskProcess,historyPath:history.path,response:history.response},null,2));
    if(liveTaskModel)await verifyLiveTaskModel({panel,task:liveTask,profile:liveTaskProfile,root,check,waitFor,taskHistory});
  }
  if(exitFixture){
    for(const connection of connections)connection.close();
    child.kill();await new Promise(resolve=>child.once('exit',resolve));
    const chatRoot=join(root,'chat');await mkdir(chatRoot,{recursive:true});
    await writeFile(join(chatRoot,'index.json'),JSON.stringify([exitFixture.id]));
    await writeFile(join(chatRoot,`${exitFixture.id}.json`),JSON.stringify(exitFixture));
    const exitServer=createServer();await new Promise(resolve=>exitServer.listen(0,'127.0.0.1',resolve));
    port=exitServer.address().port;await new Promise(resolve=>exitServer.close(resolve));
    childExited=false;launchError=undefined;
    child=spawn(exe,['--visual-test',root],{windowsHide:true,stdio:'ignore',env:{...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:`--remote-debugging-port=${port} --remote-debugging-address=127.0.0.1`,WEBVIEW2_USER_DATA_FOLDER:join(root,'webview-exit')}});
    child.on('exit',()=>childExited=true);child.on('error',error=>{launchError=error;childExited=true;});
    pet=await connect(await waitFor(async()=>ownTarget(await targets(),'/index.html')??ownTarget(await targets(),'/'),'Mascote com histórico próprio de saída'));
    panel=await connect(await waitFor(async()=>ownTarget(await targets(),'/panel.html'),'Painel com histórico próprio de saída'));
    await waitFor(()=>panel.evaluate(`!!document.querySelector('#chatCreateForm') && !!document.querySelector('#startTask')`),'Controles após carregar fixture de saída');
    await panel.invoke('show_panel');
    const sendNonce=randomUUID();const path=join(root,'chat',`${exitFixture.id}.json`);
    Object.assign(exitFixture,{state:'working',revision:1,activeNonce:sendNonce,usedNonces:[sendNonce],messages:[{role:'user',text:'Own persisted state fixture; no provider dispatched.'}]});
    await writeFile(path,JSON.stringify(exitFixture));
    await panel.call('Page.enable');
    const review=await panel.invoke('prepare_exit_review');
    check('native_exit_review_pins_exact_chat_send_and_billing',review.resources.length===1&&review.resources[0].id===exitFixture.id&&review.resources[0].revision===1&&review.resources[0].sendNonce===sendNonce&&review.resources[0].billing==='api');
    check('native_exit_review_rejects_missing_consent',await panel.evaluate(`window.__TAURI_INTERNALS__.invoke('confirm_exit',{nonce:${JSON.stringify(review.nonce)},confirmed:false}).then(()=>false,()=>true)`));
    await panel.evaluate(`window.__TAURI_INTERNALS__.invoke('request_exit').catch(error=>{window.__capyExitError=String(error);});true`);
    const dialog=await waitFor(()=>panel.takeDialog(),'Confirmação de saída real do WebView');const prompt=dialog.message;
    await writeFile(join(root,'exit-confirmation.json'),JSON.stringify({resources:review.resources,type:dialog.type,message:prompt},null,2));
    check('native_exit_prompt_names_exact_chat_and_uncertain_consumption',dialog.type==='confirm'&&prompt.includes(exitFixture.id)&&prompt.includes('Own exit fixture')&&prompt.includes('API · cobrança por uso')&&prompt.includes('consumo pode ter ocorrido')&&prompt.includes('não reenviará'));
    await panel.call('Page.handleJavaScriptDialog',{accept:false});
    await delay(100);
    check('native_exit_cancel_keeps_chat_and_application_unchanged',(await panel.invoke('list_chats'))[0].state==='working'&&!childExited);
    const canceled=await panel.invoke('prepare_exit_review');await panel.invoke('cancel_exit_review',{nonce:canceled.nonce});
    check('native_exit_canceled_nonce_cannot_close_application',await panel.evaluate(`window.__TAURI_INTERNALS__.invoke('confirm_exit',{nonce:${JSON.stringify(canceled.nonce)},confirmed:true}).then(()=>false,()=>true)`));
    for(const [command,args] of [
      ['terminal_input',{id:randomUUID(),data:'Own nonexistent terminal fixture'}],
      ['terminal_model_picker',{id:randomUUID()}],
      ['terminal_interrupt',{id:randomUUID(),confirmed:true}],
    ]){
      const inputReview=await panel.invoke('prepare_exit_review');
      check(`native_exit_${command}_rejects_unowned_terminal`,await panel.evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)}).then(()=>false,error=>String(error).includes('Terminal não encontrado'))`));
      check(`native_exit_${command}_invalidates_prior_approval_before_write`,await panel.evaluate(`window.__TAURI_INTERNALS__.invoke('confirm_exit',{nonce:${JSON.stringify(inputReview.nonce)},confirmed:true}).then(()=>false,error=>String(error).includes('Solicite a saída novamente'))`));
      check(`native_exit_${command}_failure_keeps_own_chat_and_application`,!childExited&&(await panel.invoke('list_chats'))[0].activeNonce===sendNonce);
    }
    const stale=await panel.invoke('prepare_exit_review');
    Object.assign(exitFixture,{revision:2,activeNonce:randomUUID()});exitFixture.usedNonces.push(exitFixture.activeNonce);
    await writeFile(path,JSON.stringify(exitFixture));
    check('native_exit_review_rejects_changed_send_identity',await panel.evaluate(`window.__TAURI_INTERNALS__.invoke('confirm_exit',{nonce:${JSON.stringify(stale.nonce)},confirmed:true}).then(()=>false,()=>true)`));
    check('native_exit_stale_approval_does_not_close_application',!childExited&&(await panel.invoke('list_chats'))[0].activeNonce===exitFixture.activeNonce);
    const invalid={...exitFixture,futureField:'preserve me'};await writeFile(path,JSON.stringify(invalid));
    const unreadable=await panel.evaluate(`window.__TAURI_INTERNALS__.invoke('request_exit').then(()=>false,()=>true)`);
    await waitFor(()=>panel.evaluate(`!document.querySelector('#error')?.hidden&&document.querySelector('#error')?.textContent.includes('Histórico incompatível')`),'Erro de conferência de saída visível');
    check('native_exit_unreadable_history_blocks_exit_and_shows_reason',unreadable&&!childExited&&JSON.parse(await readFile(path,'utf8')).futureField==='preserve me');
    Object.assign(exitFixture,{state:'failed',activeNonce:null,lastError:'Own state fixture ended; no provider call occurred.'});await writeFile(path,JSON.stringify(exitFixture));
  }
  if(process.argv.includes('--live-chat')||process.argv.includes('--live-transfer')||process.argv.includes('--live-recovery')||liveChatGuides){
    await panel.invoke('demo_action',{action:'scenario',id:'',answer:'real'});
    const identity=await panel.invoke('profile_identity',{id:'claude-default',cwd:null});
    check('native_live_chat_pins_subscription_before_any_send',identity.loggedIn&&identity.billing==='subscription'&&!!identity.account);
    const sourceSwitchProof=process.argv.includes('--live-chat')&&!process.argv.includes('--live-transfer')&&!process.argv.includes('--live-recovery');
    const alternateChat=sourceSwitchProof?await panel.invoke('create_chat',{request:{title:'Own alternate chat fixture',kind:'claudeCli',profileId:'claude-default',model:'haiku',expectedAccount:identity.account,expectedBilling:'subscription',credentialRevision:null}}):null;
    const chat=await panel.invoke('create_chat',{request:{title:'Own native chat proof',kind:'claudeCli',profileId:'claude-default',model:'sonnet',expectedAccount:identity.account,expectedBilling:'subscription',credentialRevision:null}});
    if(alternateChat){
      await panel.invoke('open_source',{id:`chat:${alternateChat.id}`});
      await waitFor(()=>panel.evaluate(`document.querySelector('#chatSelection').value===${JSON.stringify(alternateChat.id)}`),'Conversa ociosa própria exata selecionada');
      check('native_live_chat_opens_alternate_owned_uuid_before_send',alternateChat.id!==chat.id&&(await panel.invoke('list_chats')).find(value=>value.id===alternateChat.id)?.messages.length===0);
    }
    const firstMarker=process.argv.includes('--live-recovery')?`CAPY_RECOVERY_${randomUUID()}`:liveChatGuides?`CAPY_CHAT_GUIDE_${randomUUID()}`:'CAPY_NATIVE_CHAT_PROOF';
    const chatWorkspace=join(root,'chat','workspaces',chat.id);
    if(liveChatGuides){await mkdir(chatWorkspace,{recursive:true});await writeFile(join(chatWorkspace,'CLAUDE.md'),`When asked for the marker from this workspace instruction, reply only: ${firstMarker}`);}
    const firstPrompt=liveChatGuides?`What exact marker does this workspace instruction give you? Reply with only the marker. Do not use tools.`:`Memorize este marcador para a próxima mensagem: ${firstMarker}. Responda apenas esse marcador. Não use ferramentas.`;
    await panel.evaluate(`window.__capyChatProof=window.__TAURI_INTERNALS__.invoke('send_chat',{request:${JSON.stringify({id:chat.id,revision:chat.revision,nonce:randomUUID(),model:'sonnet',text:firstPrompt})}}); window.__capyChatProof.then(result=>{window.__capyChatProofResult=result;},error=>{window.__capyChatProofError=String(error);}); true`);
    await waitFor(()=>pet.evaluate(`document.querySelector('#pet').classList.contains('s-working')`),'Chat real mostra trabalho',30_000);
    check('native_live_chat_immediately_drives_working_pet',true);
    await waitFor(()=>pet.evaluate(`document.querySelector('#pet').classList.contains('g-celebrate')`),'Resultado real confirma comemoração',30_000);
    const result=await waitFor(()=>panel.evaluate('window.__capyChatProofResult'),'Resultado do chat real',30_000);
    check('native_live_chat_provider_result_matches_exact_conversation',result.id===chat.id&&result.state==='completed'&&result.messages.at(-1)?.text.includes(firstMarker));
    await panel.invoke('open_source',{id:`chat:${chat.id}`});
    await waitFor(()=>panel.evaluate(`document.querySelector('#chatSelection').value===${JSON.stringify(chat.id)}`),'Card abre a conversa exata');
    const openedChats=await panel.invoke('list_chats');
    if(alternateChat){
      check('native_live_chat_switches_to_exact_reply_uuid_without_touching_alternate',await panel.evaluate(`document.querySelector('#chatSelection').value===${JSON.stringify(chat.id)}`)&&chat.id!==alternateChat.id&&openedChats.find(value=>value.id===chat.id)?.messages.at(-1)?.text.includes(firstMarker)&&openedChats.find(value=>value.id===alternateChat.id)?.messages.length===0);
      const staleSourceRejected=await panel.invoke('open_source',{id:`chat:${randomUUID()}`}).then(()=>false,()=>true);
      check('native_live_chat_stale_source_cannot_switch_selection',staleSourceRejected&&await panel.evaluate(`document.querySelector('#chatSelection').value===${JSON.stringify(chat.id)}`));
    }else check('native_live_chat_source_selects_exact_uuid',await panel.evaluate(`document.querySelector('#chatSelection').value===${JSON.stringify(chat.id)}`));
    await waitFor(()=>pet.evaluate(`!document.querySelector('#pet').classList.contains('g-celebrate')`),'Comemoração termina no ciclo da mascote',5_000);
    await delay(1000);
    check('native_live_chat_completion_does_not_loop',await pet.evaluate(`!document.querySelector('#pet').classList.contains('g-celebrate')`));
    if(process.argv.includes('--live-recovery')){
      // Simulate loss at the persistence boundary using only this fixture's record.
      // The provider turn above completed; this is not proof of a network crash.
      child.kill();await Promise.race([new Promise(resolve=>child.once('exit',resolve)),delay(5000)]);
      assert.ok(childExited,'Own fixture must stop before editing its record');
      const path=join(root,'chat',`${chat.id}.json`),saved=JSON.parse(await readFile(path,'utf8'));
      assert.equal(saved.id,chat.id);assert.equal(saved.state,'completed');assert.equal(saved.messages.length,2);
      saved.messages.pop();saved.state='working';saved.activeNonce=saved.usedNonces.at(-1);saved.revision++;saved.cliStarted=false;saved.recoveryReview=null;saved.lastError=null;
      await writeFile(path,JSON.stringify(saved));
      for(const connection of connections)connection.close();
      const recoveryServer=createServer();await new Promise(resolve=>recoveryServer.listen(0,'127.0.0.1',resolve));
      port=recoveryServer.address().port;await new Promise(resolve=>recoveryServer.close(resolve));
      childExited=false;launchError=undefined;
      child=spawn(exe,['--visual-test',root],{windowsHide:true,stdio:'ignore',env:{...process.env,
        WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:`--remote-debugging-port=${port} --remote-debugging-address=127.0.0.1`,WEBVIEW2_USER_DATA_FOLDER:join(root,'webview-recovery')}});
      child.on('exit',()=>childExited=true);child.on('error',error=>{launchError=error;childExited=true;});
      pet=await connect(await waitFor(async()=>ownTarget(await targets(),'/index.html')??ownTarget(await targets(),'/'),'Mascote após recuperação'));
      panel=await connect(await waitFor(async()=>ownTarget(await targets(),'/panel.html'),'Painel após recuperação'));
      await waitFor(()=>panel.evaluate(`!!document.querySelector('#chatRecovery')`),'Interface de recuperação');
      const uncertain=(await panel.invoke('list_chats')).find(value=>value.id===chat.id);
      check('native_recovery_restart_never_resends_or_claims_completion',uncertain.state==='unknown'&&uncertain.messages.length===1&&uncertain.activeNonce===null&&uncertain.usedNonces.includes(saved.activeNonce));
      await panel.invoke('demo_action',{action:'scenario',id:'',answer:'real'});
      await waitFor(()=>pet.evaluate(`document.querySelector('#pet').classList.contains('s-waiting') && !document.querySelector('#petBadge').hidden`),'Envio incerto pede revisão');
      check('native_recovery_attention_never_celebrates_lost_result',await pet.evaluate(`!document.querySelector('#pet').classList.contains('g-celebrate')`));
      await panel.invoke('open_source',{id:`chat:${chat.id}`});
      await waitFor(()=>panel.evaluate(`document.querySelector('#chatSelection').value===${JSON.stringify(chat.id)} && !!document.querySelector('[data-prepare-chat-recovery]')`),'Conversa incerta exata');
      await panel.evaluate(`document.querySelector('[data-prepare-chat-recovery]').click()`);
      await waitFor(()=>panel.evaluate(`!!document.querySelector('[data-chat-recovery]')`),'Revisão persistida da interrupção',20_000);
      const prepared=(await panel.invoke('list_chats')).find(value=>value.id===chat.id),review=prepared.recoveryReview;
      check('native_recovery_pins_exact_cli_history_and_fresh_consent',review.id===chat.id&&review.resumeCli&&await panel.evaluate(`(()=>{const input=document.querySelector('#chatRecovery input[name="reviewed"]');return input.required&&!input.checked;})()`));
      await panel.evaluate(`document.querySelector('#chatRecovery').scrollIntoView({block:'start'})`);
      await panel.screenshot('chat-recovery');
      const recoveryLayout=await panel.evaluate(`({scrollWidth:document.documentElement.scrollWidth,width:innerWidth,checkboxWidth:document.querySelector('#chatRecovery input').getBoundingClientRect().width,overflow:[...document.querySelectorAll('body *')].filter(element=>element.getBoundingClientRect().right>innerWidth).map(element=>({tag:element.tagName,id:element.id,className:element.className,right:element.getBoundingClientRect().right})).slice(0,20)})`);
      await writeFile(join(root,'recovery-layout.json'),JSON.stringify(recoveryLayout,null,2));
      check('native_recovery_form_no_horizontal_overflow_and_compact_checkbox',recoveryLayout.scrollWidth<=recoveryLayout.width&&recoveryLayout.checkboxWidth<30);
      const approval={id:chat.id,revision:prepared.revision,nonce:review.nonce,reviewed:false};
      check('native_recovery_backend_requires_explicit_review',await panel.evaluate(`window.__TAURI_INTERNALS__.invoke('approve_chat_recovery',${JSON.stringify(approval)}).then(()=>false,()=>true)`));
      await panel.evaluate(`(()=>{const form=document.querySelector('[data-chat-recovery]');form.elements.namedItem('reviewed').checked=true;form.requestSubmit();})()`);
      const acknowledged=await waitFor(async()=>{const value=(await panel.invoke('list_chats')).find(value=>value.id===chat.id);return value.state==='failed'?value:null;},'Confirmação sem envio',20_000);
      check('native_recovery_approval_preserves_history_and_consumed_nonce_without_send',acknowledged.messages.length===1&&acknowledged.cliStarted&&acknowledged.interruptions[0]?.sendNonce===saved.activeNonce&&acknowledged.usedNonces.includes(review.nonce));
      check('native_recovery_replay_is_rejected',await panel.evaluate(`window.__TAURI_INTERNALS__.invoke('approve_chat_recovery',${JSON.stringify({...approval,reviewed:true})}).then(()=>false,()=>true)`));
      await waitFor(()=>panel.evaluate(`!document.querySelector('#chatSendForm button').disabled`),'Nova mensagem disponível após revisão');
      await panel.evaluate(`(()=>{const form=document.querySelector('#chatSendForm');form.elements.namedItem('model').value='haiku';form.elements.namedItem('text').value='Qual foi o marcador enviado na primeira mensagem? Responda apenas ele, sem ferramentas.';form.requestSubmit();})()`);
      const continued=await waitFor(async()=>{const value=(await panel.invoke('list_chats')).find(value=>value.id===chat.id);if(value.state==='failed'&&value.revision>acknowledged.revision)throw new Error(value.lastError);return value.state==='completed'?value:null;},'Retomada oficial após revisão',40_000);
      check('native_recovery_resumes_original_uuid_and_context_with_new_message',continued.id===chat.id&&continued.model==='haiku'&&continued.messages.length===3&&continued.messages.at(-1).text.includes(firstMarker));
      check('native_recovery_uncertainty_remains_visible_after_new_success',continued.interruptions.length===1&&await panel.evaluate(`document.querySelector('#chatMessages').textContent.includes('Resposta deste envio indisponível')`));
    }
    if(process.argv.includes('--live-transfer')||liveChatGuides){
      await panel.evaluate(`document.querySelector('#chatCreateForm input[name="model"]').value='haiku';document.querySelector('#chatVerify').click();`);
      await waitFor(()=>panel.evaluate(`!document.querySelector('#chatPrepareTransfer').disabled`),'Destino verificado para transferência',15_000);
      await panel.evaluate(`document.querySelector('#chatPrepareTransfer').click()`);
      await waitFor(()=>panel.evaluate(`!!document.querySelector('#chatReview form[data-chat-review]')`),'Resumo revisável na interface',15_000);
      const review=await panel.invoke('chat_transfer_review',{sourceId:chat.id});
      if(liveChatGuides){
        check('native_live_chat_transfer_summary_contains_captured_path_and_text',review.summary.guides.includes(join(chatWorkspace,'CLAUDE.md'))&&review.summary.guides.includes(firstMarker));
        check('native_live_chat_transfer_review_renders_captured_instruction',await panel.evaluate(`document.querySelector('#chatReview').textContent.includes(${JSON.stringify(firstMarker)})`));
      }
      if(process.argv.includes('--live-recovery'))check('native_live_transfer_preserves_uncertainty_in_read_only_origin_warning',
        review.uncertainMessages?.[0]===0&&review.summary.state.includes('Mensagem 1: resultado indisponível')&&await panel.evaluate(`document.querySelector('[data-chat-uncertainty]')?.textContent.includes('mensagens 1 sem resposta confirmada')`));
      check('native_live_transfer_review_pins_source_destination_and_model',review.sourceId===chat.id&&review.destination.billing==='subscription'&&review.model==='haiku');
      check('native_live_transfer_requires_fresh_review_checkbox',await panel.evaluate(`(()=>{const input=document.querySelector('#chatReview input[name="reviewed"]');return input.required&&!input.checked;})()`));
      await waitFor(()=>pet.evaluate(`!document.querySelector('#petBadge').hidden && document.querySelector('#pet').classList.contains('g-wave')`),'Revisão pede atenção à mascote');
      check('native_live_transfer_new_review_waves_and_has_badge',true);
      check('native_live_transfer_review_checkbox_keeps_compact_width',await panel.evaluate(`document.querySelector('#chatReview input[name="reviewed"]').getBoundingClientRect().width<30`));
      check('native_live_transfer_review_no_horizontal_overflow',await panel.evaluate('document.documentElement.scrollWidth<=innerWidth'));
      await panel.screenshot('transfer-review');
      const denied=await panel.evaluate(`window.__TAURI_INTERNALS__.invoke('approve_chat_transfer',${JSON.stringify({sourceId:chat.id,nonce:review.nonce,summary:review.summary,reviewed:false,billingConfirmed:false})}).then(()=>false,()=>true)`);
      check('native_live_transfer_server_rejects_missing_review',denied);
      if(process.argv.includes('--live-transfer')){
      const edited={...review.summary,objective:'Responda apenas CAPY_NATIVE_TRANSFER_PROOF, sem ferramentas.',decisions:'A validação anterior já terminou. Preserve seu contexto como referência; a nova solicitação é responder apenas CAPY_NATIVE_TRANSFER_PROOF.',state:'Última resposta recebida. Prossiga com o objetivo revisado.',nextSteps:'Responda somente CAPY_NATIVE_TRANSFER_PROOF. Não repita solicitações anteriores já concluídas.'};
      await panel.evaluate(`(()=>{const form=document.querySelector('#chatReview form');const values=${JSON.stringify(edited)};for(const [key,value] of Object.entries(values))form.elements.namedItem(key).value=value;form.elements.namedItem('reviewed').checked=true;form.requestSubmit();})()`);
      const destination=await waitFor(async()=>{const values=await panel.invoke('list_chats');const source=values.find(c=>c.id===chat.id);const target=values.find(c=>c.id===source?.transferredTo);if(target?.state==='failed')throw new Error(target.lastError);return target?.state==='completed'?target:null;},'Continuação real aprovada pelo formulário',40_000);
      check('native_live_transfer_creates_exact_new_uuid_with_reviewed_context',destination.id!==chat.id&&destination.model==='haiku'&&destination.messages[0].text.includes('CAPY_NATIVE_TRANSFER_PROOF')&&destination.messages.at(-1)?.text.includes('CAPY_NATIVE_TRANSFER_PROOF'));
      if(process.argv.includes('--live-recovery'))check('native_live_transfer_context_retains_lost_result_and_no_repeat_warning',
        destination.messages[0].text.includes('Mensagem 1: resultado indisponível')&&destination.messages[0].text.includes('Não repetir automaticamente'));
      await waitFor(()=>panel.evaluate(`document.querySelector('#chatSelection').value===${JSON.stringify(destination.id)}`),'Destino selecionado após aprovação');
      const replayDenied=await panel.evaluate(`window.__TAURI_INTERNALS__.invoke('approve_chat_transfer',${JSON.stringify({sourceId:chat.id,nonce:review.nonce,summary:edited,reviewed:true,billingConfirmed:false})}).then(()=>false,()=>true)`);
      check('native_live_transfer_replay_does_not_create_another_destination',replayDenied&&(await panel.invoke('list_chats')).length===2);
      }
    }
  }
  await pet.invoke('hide_window',{label:'panel'});

  await pet.invoke('demo_action',{action:'scenario',id:'',answer:'working'});
  await waitFor(()=>pet.evaluate(`document.querySelector('#pet').classList.contains('s-working')`),'Postura trabalhando');
  await waitFor(()=>pet.evaluate(`Number(getComputedStyle(document.querySelector('.laptop')).opacity)>.99`),'Teclado visível');
  check('native_working_keyboard_visible',true);
  await pet.screenshot('working');
  const box=await pet.evaluate(`(()=>{const r=document.querySelector('#petToggle').getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height};})()`);
  const x=box.x+box.width*.8,y=box.y+box.height*.6;
  await pet.call('Input.dispatchMouseEvent',{type:'mouseMoved',x:0,y:0});
  await pet.call('Input.dispatchMouseEvent',{type:'mouseMoved',x,y});
  await waitFor(()=>pet.evaluate(`document.querySelector('#pet').classList.contains('is-hovered') && document.querySelector('#pet').style.getPropertyValue('--gaze-x')!==''`),'Hover e gaze após evento WebView');
  check('native_hover_gaze',await pet.evaluate(`document.querySelector('#pet').classList.contains('is-hovered') && document.querySelector('#pet').style.getPropertyValue('--gaze-x')!==''`));
  await pet.screenshot('hover');
  await pet.call('Input.dispatchMouseEvent',{type:'mouseMoved',x:0,y:0});
  check('native_no_global_cursor_follow',await pet.evaluate(`!document.querySelector('#pet').classList.contains('is-hovered') && document.querySelector('#pet').style.getPropertyValue('--gaze-x')===''`));
  await pet.call('Input.dispatchMouseEvent',{type:'mouseMoved',x,y});
  await pet.call('Input.dispatchMouseEvent',{type:'mousePressed',x,y,button:'left',clickCount:1});
  await pet.call('Input.dispatchMouseEvent',{type:'mouseReleased',x,y,button:'left',clickCount:1});
  check('native_click_reaction',await pet.evaluate(`document.querySelector('#pet').classList.contains('g-click')`));
  await waitFor(()=>pet.evaluate(`document.querySelector('#petToggle').getAttribute('aria-expanded')==='true'`),'Clique abre resumo');
  check('native_click_opens_summary',true);
  const summaryTarget=await waitFor(async()=>ownTarget(await targets(),'/summary.html'),'Resumo nativo/CDP');
  const summary=await connect(summaryTarget);
  await waitFor(()=>summary.evaluate(`!!document.querySelector('[data-id]')`),'Cards do resumo');
  check('native_summary_no_horizontal_overflow',await summary.evaluate('document.documentElement.scrollWidth<=innerWidth'));
  await summary.screenshot('summary');
  check('native_summary_quota_board_has_all_demo_accounts',await summary.evaluate(`document.querySelectorAll('.quota-account').length===3 && document.querySelectorAll('.quota-dial[role="meter"]').length===6`));
  await pet.invoke('hide_window',{label:'summary'});

  await waitFor(()=>pet.evaluate(`!document.querySelector('#pet').classList.contains('g-celebrate') && !document.querySelector('#pet').classList.contains('g-click')`),'Gesto finito encerra antes da prova de banho');
  await pet.evaluate(`document.querySelector('#pet').classList.add('s-bath');true`);
  await writeFile(join(root,'accessory-animation.json'),JSON.stringify(await pet.evaluate(`({classes:document.querySelector('#pet').getAttribute('class'),body:document.body.className,name:getComputedStyle(document.querySelector('#orange')).animationName,playState:getComputedStyle(document.querySelector('#orange')).animationPlayState})`),null,2));
  check('native_bath_animation_present_before_pause',await pet.evaluate(`getComputedStyle(document.querySelector('#orange')).animationName==='capy-yuzu-float'`));
  await pet.invoke('hide_window',{label:'pet'});
  await waitFor(()=>pet.evaluate(`document.body.classList.contains('pet-paused')`),'Animações pausadas quando oculta');
  check('native_hidden_animations_paused',await pet.evaluate(`document.getAnimations().every(a=>a.playState==='paused')`));
  check('native_hidden_bath_animation_paused',await pet.evaluate(`getComputedStyle(document.querySelector('#orange')).animationPlayState==='paused'`));
  await pet.evaluate(`document.querySelector('#pet').classList.remove('s-bath');document.querySelector('#pet').classList.add('g-orange-trick');true`);
  check('native_hidden_orange_gesture_paused',await pet.evaluate(`getComputedStyle(document.querySelector('#orange')).animationName==='capy-orange-trick-toss' && getComputedStyle(document.querySelector('#orange')).animationPlayState==='paused'`));
  await pet.invoke('move_pet',{dx:0,dy:0});
  // The tray's show operation is intentionally exercised by the existing native smoke test.
  await pet.invoke('toggle_summary');
  await waitFor(()=>pet.evaluate(`!document.body.classList.contains('pet-paused')`),'Mascote reaparece');
  check('native_shown_animations_resume',true);
  await pet.invoke('hide_window',{label:'summary'});

  await pet.invoke('demo_action',{action:'motion',id:'',answer:'true'});
  await waitFor(()=>pet.evaluate(`document.body.classList.contains('reduce-motion')`),'Movimento reduzido');
  check('native_reduced_motion_removes_spatial_animation',await pet.evaluate(`document.getAnimations().length===0 && getComputedStyle(document.querySelector('.eye-open')).translate==='none'`));
  check('native_reduced_motion_removes_orange_gesture',await pet.evaluate(`getComputedStyle(document.querySelector('#orange')).animationName==='none'`));
  await pet.evaluate(`document.querySelector('#pet').classList.remove('g-orange-trick');document.querySelector('#pet').classList.add('s-bath');true`);
  check('native_reduced_motion_removes_bath_animation',await pet.evaluate(`getComputedStyle(document.querySelector('#orange')).animationName==='none'`));
  await pet.screenshot('reduced-motion');
  await pet.invoke('demo_action',{action:'motion',id:'',answer:'false'});
  await waitFor(()=>pet.evaluate(`!document.body.classList.contains('reduce-motion')`),'Preferência local de movimento restaurada');
  await pet.call('Emulation.setEmulatedMedia',{features:[{name:'prefers-reduced-motion',value:'reduce'}]});
  check('native_system_reduced_motion_removes_bath_animation',await pet.evaluate(`getComputedStyle(document.querySelector('#orange')).animationName==='none'`));
  await pet.evaluate(`document.querySelector('#pet').classList.remove('s-bath');document.querySelector('#pet').classList.add('g-orange-trick');true`);
  check('native_system_reduced_motion_removes_orange_gesture',await pet.evaluate(`getComputedStyle(document.querySelector('#orange')).animationName==='none'`));
  await pet.call('Emulation.setEmulatedMedia',{features:[{name:'prefers-reduced-motion',value:'no-preference'}]});
  await pet.evaluate(`document.querySelector('#pet').classList.remove('s-bath','g-orange-trick');true`);
  await pet.invoke('demo_action',{action:'motion',id:'',answer:'false'});
  await pet.invoke('demo_action',{action:'scenario',id:'',answer:'waiting'});
  await waitFor(()=>pet.evaluate(`document.querySelector('#pet').classList.contains('g-wave')`),'Novo pedido acena');
  check('native_new_request_waves_and_badge_visible',await pet.evaluate(`!document.querySelector('#petBadge').hidden && Number(document.querySelector('#petBadge').textContent)>0`));
  await pet.screenshot('waiting');
  await pet.invoke('show_panel');
  await waitFor(()=>pet.evaluate(`!document.querySelector('#pet').classList.contains('g-wave')`),'Painel marca pedidos como vistos');
  check('native_panel_acknowledges_attention',true);
  if(liveTask){
    if(liveTaskControls){
      await panel.evaluate(`document.querySelector('#terminalSection').scrollIntoView({block:'start'});true`);
      await verifyLiveTaskControls({panel,task:liveTask,profile:liveTaskProfile,process:liveTaskProcess,root,check,waitFor,taskHistory,processInfo});
    }
    await panel.call('Page.enable');const review=await panel.invoke('prepare_exit_review');
    const expectedTasks=[liveTask,...(liveTaskHandoffTask?[liveTaskHandoffTask]:[])];
    check(liveTaskHandoffTask?'native_handoff_exit_review_matches_both_owned_terminals':'native_task_exit_review_matches_only_owned_real_terminal',review.resources.length===expectedTasks.length&&expectedTasks.every(task=>review.resources.some(resource=>resource.kind==='terminal'&&resource.id===task.id&&sameFolder(resource.cwd,task.cwd)&&resource.account===task.account)));
    await panel.evaluate(`window.__TAURI_INTERNALS__.invoke('request_exit').catch(error=>{window.__capyExitError=String(error);});true`);
    const dialog=await waitFor(()=>panel.takeDialog(),'Confirmação de saída do terminal Claude real');
    check(liveTaskHandoffTask?'native_handoff_exit_dialog_names_both_exact_sessions':'native_task_exit_dialog_names_exact_session',dialog.type==='confirm'&&expectedTasks.every(task=>dialog.message.includes(task.id))&&dialog.message.includes('fecha estes terminais integrados'));
    if(liveTaskModel)check('native_model_switch_exit_dialog_labels_launch_model',dialog.message.includes('modelo inicial: haiku'));
    await panel.call('Page.handleJavaScriptDialog',{accept:true});
    const started=Date.now();while(!childExited&&Date.now()-started<10_000)await delay(100);check('native_task_approved_exit_closes_own_application',childExited);
    const ownProcesses=[liveTaskProcess,...(liveTaskHandoffProcess?[liveTaskHandoffProcess]:[])];const closed=[];const processDeadline=Date.now()+10_000;
    while(Date.now()<processDeadline){
      for(const process of ownProcesses){if(closed.some(item=>item.pid===process.pid))continue;const current=await processInfo(process.pid);if(!current||current.started!==process.started)closed.push({pid:process.pid,started:process.started});}
      if(closed.length===ownProcesses.length)break;await delay(100);
    }
    await writeFile(join(root,'task-exit.json'),JSON.stringify({originalProcesses:ownProcesses,closed,appClosed:childExited},null,2));
    check(liveTaskHandoffTask?'native_handoff_approved_exit_closes_both_exact_claude_processes':'native_task_approved_exit_closes_exact_claude_process',closed.length===ownProcesses.length);
  }
  if(exitFixture){
    const review=await panel.invoke('prepare_exit_review');
    check('native_exit_completed_fixture_has_no_active_resource',review.resources.length===0);
    Object.assign(exitFixture,{state:'working',revision:3,activeNonce:randomUUID()});exitFixture.usedNonces.push(exitFixture.activeNonce);
    await writeFile(join(root,'chat',`${exitFixture.id}.json`),JSON.stringify(exitFixture));
    await panel.evaluate(`window.__TAURI_INTERNALS__.invoke('request_exit').catch(error=>{window.__capyExitError=String(error);});true`);
    const dialog=await waitFor(()=>panel.takeDialog(),'Aprovação de saída real do WebView');
    check('native_exit_fresh_dialog_still_names_exact_active_chat',dialog.message.includes(exitFixture.id)&&dialog.type==='confirm');
    await panel.call('Page.handleJavaScriptDialog',{accept:true});
    const start=Date.now();while(!childExited&&Date.now()-start<10_000)await delay(100);
    check('native_exit_fresh_approval_closes_own_application',childExited);
  }
  if(corruptProfiles){
    await panel.invoke('request_exit');
    const started=Date.now();while(!childExited&&Date.now()-started<10_000)await delay(100);
    check('native_profiles_exit_event_closes_own_application',childExited);
    check('native_profiles_incompatible_bytes_preserved_after_exit',await readFile(join(root,'profiles.json'),'utf8')===incompatibleProfiles);
  }
  if(liveTaskExternal){
    externalProof=externalTaskProof({panel,root,check,waitFor,taskHistory,sameFolder,appExited:()=>childExited});
    await externalProof.verify();
  }
  if(quotaSettings){
    const preferencesPath=join(root,'preferences.json');
    await waitFor(()=>panel.evaluate(`!!document.querySelector('#petSounds')`),'Controle nativo de sons');
    check('native_sound_preference_defaults_disabled',await panel.evaluate(`!document.querySelector('#petSounds').checked`));
    await panel.evaluate(`document.querySelector('#petSounds').click();true`);
    await waitFor(async()=>JSON.parse(await readFile(preferencesPath,'utf8')).sounds===true,'preferência de som ligada persistida');
    check('native_sound_preference_toggle_persists_enabled',JSON.parse(await readFile(preferencesPath,'utf8')).sounds===true);
    await panel.evaluate(`document.querySelector('#petSounds').click();true`);
    await waitFor(async()=>JSON.parse(await readFile(preferencesPath,'utf8')).sounds===false,'preferência de som desligada persistida');
    check('native_sound_preference_toggle_persists_disabled',JSON.parse(await readFile(preferencesPath,'utf8')).sounds===false);
    const path=join(quotaProfile.configDir,'settings.json');const bridgePath=join(quotaProfile.configDir,'capy-quotas','bridge.json');
    await panel.invoke('connect_claude_quotas',{id:quotaProfile.id,enabled:true});
    const enabled=JSON.parse(await readFile(path,'utf8'));const active=await readFile(path);const backup=await readFile(bridgePath);
    check('native_quota_settings_connect_preserves_other_fields',enabled.model===quotaOriginal.model&&JSON.stringify(enabled.hooks)===JSON.stringify(quotaOriginal.hooks)&&JSON.stringify(enabled.ownFutureValue)===JSON.stringify(quotaOriginal.ownFutureValue)&&enabled.statusLine.command.includes('capy-quotas/statusline.ps1'));
    const wrapper=promisify(execFile)('powershell.exe',['-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-File',join(quotaProfile.configDir,'capy-quotas','statusline.ps1')],{windowsHide:true,timeout:20_000,maxBuffer:8192});
    wrapper.child.stdin.end('{}\n');
    check('native_quota_settings_embedded_wrapper_preserves_original_unicode_output',(await wrapper).stdout.trim()==='capy_ação_日本語_🦫');
    const rejects=()=>panel.invoke('connect_claude_quotas',{id:quotaProfile.id,enabled:false}).then(()=>false,()=>true);
    await writeFile(bridgePath,'{"invalid":true}');
    check('native_quota_settings_corrupt_bridge_blocks_disconnect',await rejects());
    check('native_quota_settings_corrupt_bridge_preserves_exact_settings',(await readFile(path)).equals(active));
    const incomplete=JSON.parse(backup);delete incomplete.original;await writeFile(bridgePath,JSON.stringify(incomplete));
    check('native_quota_settings_incomplete_bridge_blocks_disconnect',await rejects());
    check('native_quota_settings_incomplete_bridge_preserves_exact_settings',(await readFile(path)).equals(active));
    const {unlink}=await import('node:fs/promises');await unlink(bridgePath);
    check('native_quota_settings_missing_bridge_blocks_disconnect',await rejects());
    check('native_quota_settings_missing_bridge_preserves_exact_settings',(await readFile(path)).equals(active));
    await writeFile(bridgePath,backup);
    const changed=JSON.stringify({model:'opus',statusLine:{type:'command',command:'echo changed-at-origin'}});
    await writeFile(path,changed);check('native_quota_settings_origin_change_blocks_restore',await rejects());
    check('native_quota_settings_origin_change_preserves_exact_bytes',(await readFile(path,'utf8'))===changed);
    await writeFile(path,active);await panel.invoke('connect_claude_quotas',{id:quotaProfile.id,enabled:false});
    check('native_quota_settings_restores_original_configuration',isDeepStrictEqual(JSON.parse(await readFile(path,'utf8')),quotaOriginal)&&isDeepStrictEqual(JSON.parse(backup).original,quotaOriginal.statusLine));
    await panel.screenshot('quota-settings');
  }
}catch(error){failure=error;}
finally{
  if(externalProof){
    try{await externalProof.cleanup();}catch(error){failure??=error;await writeFile(join(root,'external-cleanup-error.txt'),String(error));}
  }
  if(liveTask&&!childExited&&liveTaskPanel){
    try{
      if(failure){
        await liveTaskPanel.screenshot('task-failure');
        await writeFile(join(root,'task-replay-failure.json'),JSON.stringify(await liveTaskPanel.invoke('terminal_replay',{id:liveTask.id}),null,2));
        await writeFile(join(root,'task-ui-failure.json'),JSON.stringify(await liveTaskPanel.evaluate(`({error:document.querySelector('#error')?.textContent,notice:document.querySelector('#terminalNotice')?.textContent,viewport:document.querySelector('#terminalViewport')?.innerHTML})`),null,2));
      }
      const review=await liveTaskPanel.invoke('prepare_exit_review');assert.ok(review.resources.every(resource=>resource.kind==='terminal'&&resource.id===liveTask.id));
      await liveTaskPanel.evaluate(`window.__TAURI_INTERNALS__.invoke('confirm_exit',{nonce:${JSON.stringify(review.nonce)},confirmed:true}).catch(()=>{});true`);
      await Promise.race([new Promise(resolve=>child.once('exit',resolve)),delay(5000)]);
    }catch(error){await writeFile(join(root,'task-cleanup-error.txt'),String(error));}
  }
  for(const connection of connections)connection.close();
  if(!childExited){child.kill();await Promise.race([new Promise(resolve=>child.once('exit',resolve)),delay(5000)]);}
  const report={passed:!failure,checks,error:failure?.stack??null,root};
  await writeFile(join(root,'report.json'),JSON.stringify(report,null,2));
  console.log(JSON.stringify({passed:report.passed,checks:checks.length,root,error:failure?.message??null}));
}
if(failure)process.exitCode=1;
