import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {randomUUID} from 'node:crypto';
import {mkdir, writeFile, readFile, stat} from 'node:fs/promises';
import {createServer} from 'node:net';
import {dirname, join, resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {setTimeout as delay} from 'node:timers/promises';

const workspace=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const root=join(workspace,'scratch',`capy-visual-${randomUUID()}`);
const exe=join(workspace,'src-tauri','target','release','capy.exe');
await stat(exe);await mkdir(root,{recursive:true});
let exitFixture;
if(process.argv.includes('--exit-review')){
  assert.ok(!process.argv.some(arg=>['--live-chat','--live-transfer','--live-recovery'].includes(arg)),'Exit state fixture must run separately from provider calls');
  exitFixture={id:randomUUID(),title:'Own exit state fixture',target:{kind:'api',profileId:randomUUID(),provider:'OpenAI',account:'Own exit fixture',billing:'api',credentialRevision:randomUUID()},model:'fixture-model',messages:[],revision:0,state:'idle',activeNonce:null,usedNonces:[],lastError:null,cliStarted:false,cliAttempted:false,processPolicy:null,recoveryReview:null,interruptions:[],transferredTo:null};
}
const server=createServer();await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
let port=server.address().port;await new Promise(resolve=>server.close(resolve));
let child=spawn(exe,['--visual-test',root],{
  windowsHide:true,stdio:'ignore',env:{...process.env,
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

let failure;
try{
  const petTarget=await waitFor(async()=>ownTarget(await targets(),'/index.html')??ownTarget(await targets(),'/'),'Mascote nativa/CDP');
  let pet=await connect(petTarget);
  await waitFor(()=>pet.evaluate(`document.querySelector('#pet') instanceof SVGSVGElement && !!window.__TAURI_INTERNALS__`),'SVG carregado');
  const panelTarget=await waitFor(async()=>ownTarget(await targets(),'/panel.html'),'Painel nativo/CDP');
  let panel=await connect(panelTarget);
  await waitFor(()=>panel.evaluate(`!!document.querySelector('#startTask') && !!document.querySelector('#accountList')`),'Controles do painel');
  await pet.invoke('show_panel');
  check('native_panel_no_horizontal_overflow',await panel.evaluate('document.documentElement.scrollWidth<=innerWidth'));
  await panel.screenshot('panel');
  await waitFor(()=>panel.evaluate(`!!document.querySelector('#chatCreateForm') && !!document.querySelector('#chatSendForm')`),'Controles de chat');
  check('native_chat_commands_start_with_empty_own_history',(await panel.invoke('list_chats')).length===0);
  check('native_chat_create_requires_account_verification',await panel.evaluate(`document.querySelector('#chatCreateForm button[type="submit"]').disabled`));
  check('native_chat_key_is_password_without_autocomplete',await panel.evaluate(`(()=>{const key=document.querySelector('#chatApiForm input[name="key"]');return key.type==='password' && key.autocomplete==='off' && key.value==='';})()`));
  await panel.evaluate(`document.querySelector('.chat-workbench > details').open=true;document.querySelector('#chatCreateForm').closest('details').open=true;document.querySelector('.chat-workbench').scrollIntoView({block:'start'})`);
  check('native_chat_no_horizontal_overflow',await panel.evaluate('document.documentElement.scrollWidth<=innerWidth'));
  await panel.screenshot('chat');
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
  if(process.argv.includes('--live-chat')||process.argv.includes('--live-transfer')||process.argv.includes('--live-recovery')){
    await panel.invoke('demo_action',{action:'scenario',id:'',answer:'real'});
    const identity=await panel.invoke('profile_identity',{id:'claude-default',cwd:null});
    check('native_live_chat_pins_subscription_before_any_send',identity.loggedIn&&identity.billing==='subscription'&&!!identity.account);
    const chat=await panel.invoke('create_chat',{request:{title:'Own native chat proof',kind:'claudeCli',profileId:'claude-default',model:'sonnet',expectedAccount:identity.account,expectedBilling:'subscription',credentialRevision:null}});
    const firstMarker=process.argv.includes('--live-recovery')?`CAPY_RECOVERY_${randomUUID()}`:'CAPY_NATIVE_CHAT_PROOF';
    await panel.evaluate(`window.__capyChatProof=window.__TAURI_INTERNALS__.invoke('send_chat',{request:${JSON.stringify({id:chat.id,revision:chat.revision,nonce:randomUUID(),model:'sonnet',text:`Memorize este marcador para a próxima mensagem: ${firstMarker}. Responda apenas esse marcador. Não use ferramentas.`})}}); window.__capyChatProof.then(result=>{window.__capyChatProofResult=result;},error=>{window.__capyChatProofError=String(error);}); true`);
    await waitFor(()=>pet.evaluate(`document.querySelector('#pet').classList.contains('s-working')`),'Chat real mostra trabalho',30_000);
    check('native_live_chat_immediately_drives_working_pet',true);
    await waitFor(()=>pet.evaluate(`document.querySelector('#pet').classList.contains('g-celebrate')`),'Resultado real confirma comemoração',30_000);
    const result=await waitFor(()=>panel.evaluate('window.__capyChatProofResult'),'Resultado do chat real',30_000);
    check('native_live_chat_provider_result_matches_exact_conversation',result.id===chat.id&&result.state==='completed'&&result.messages.at(-1)?.text.includes(firstMarker));
    await panel.invoke('open_source',{id:`chat:${chat.id}`});
    await waitFor(()=>panel.evaluate(`document.querySelector('#chatSelection').value===${JSON.stringify(chat.id)}`),'Card abre a conversa exata');
    check('native_live_chat_source_selects_exact_uuid',true);
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
    if(process.argv.includes('--live-transfer')){
      await panel.evaluate(`document.querySelector('#chatCreateForm input[name="model"]').value='haiku';document.querySelector('#chatVerify').click();`);
      await waitFor(()=>panel.evaluate(`!document.querySelector('#chatPrepareTransfer').disabled`),'Destino verificado para transferência',15_000);
      await panel.evaluate(`document.querySelector('#chatPrepareTransfer').click()`);
      await waitFor(()=>panel.evaluate(`!!document.querySelector('#chatReview form[data-chat-review]')`),'Resumo revisável na interface',15_000);
      const review=await panel.invoke('chat_transfer_review',{sourceId:chat.id});
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
  await pet.invoke('hide_window',{label:'panel'});

  await pet.invoke('demo_action',{action:'scenario',id:'',answer:'working'});
  await waitFor(()=>pet.evaluate(`document.querySelector('#pet').classList.contains('s-working')`),'Postura trabalhando');
  await waitFor(()=>pet.evaluate(`Number(getComputedStyle(document.querySelector('.laptop')).opacity)>.99`),'Teclado visível');
  check('native_working_keyboard_visible',true);
  await pet.screenshot('working');
  const box=await pet.evaluate(`(()=>{const r=document.querySelector('#petToggle').getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height};})()`);
  const x=box.x+box.width*.8,y=box.y+box.height*.6;
  await pet.call('Input.dispatchMouseEvent',{type:'mouseMoved',x,y});
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
  await pet.invoke('hide_window',{label:'summary'});

  await pet.invoke('hide_window',{label:'pet'});
  await waitFor(()=>pet.evaluate(`document.body.classList.contains('pet-paused')`),'Animações pausadas quando oculta');
  check('native_hidden_animations_paused',await pet.evaluate(`document.getAnimations().every(a=>a.playState==='paused')`));
  await pet.invoke('move_pet',{dx:0,dy:0});
  // The tray's show operation is intentionally exercised by the existing native smoke test.
  await pet.invoke('toggle_summary');
  await waitFor(()=>pet.evaluate(`!document.body.classList.contains('pet-paused')`),'Mascote reaparece');
  check('native_shown_animations_resume',true);
  await pet.invoke('hide_window',{label:'summary'});

  await pet.invoke('demo_action',{action:'motion',id:'',answer:'true'});
  await waitFor(()=>pet.evaluate(`document.body.classList.contains('reduce-motion')`),'Movimento reduzido');
  check('native_reduced_motion_removes_spatial_animation',await pet.evaluate(`document.getAnimations().length===0 && getComputedStyle(document.querySelector('.eye-open')).translate==='none'`));
  await pet.screenshot('reduced-motion');
  await pet.invoke('demo_action',{action:'motion',id:'',answer:'false'});
  await pet.invoke('demo_action',{action:'scenario',id:'',answer:'waiting'});
  await waitFor(()=>pet.evaluate(`document.querySelector('#pet').classList.contains('g-wave')`),'Novo pedido acena');
  check('native_new_request_waves_and_badge_visible',await pet.evaluate(`!document.querySelector('#petBadge').hidden && Number(document.querySelector('#petBadge').textContent)>0`));
  await pet.screenshot('waiting');
  await pet.invoke('show_panel');
  await waitFor(()=>pet.evaluate(`!document.querySelector('#pet').classList.contains('g-wave')`),'Painel marca pedidos como vistos');
  check('native_panel_acknowledges_attention',true);
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
}catch(error){failure=error;}
finally{
  for(const connection of connections)connection.close();
  if(!childExited){child.kill();await Promise.race([new Promise(resolve=>child.once('exit',resolve)),delay(5000)]);}
  const report={passed:!failure,checks,error:failure?.stack??null,root};
  await writeFile(join(root,'report.json'),JSON.stringify(report,null,2));
  console.log(JSON.stringify({passed:report.passed,checks:checks.length,root,error:failure?.message??null}));
}
if(failure)process.exitCode=1;
