import { listen } from '@tauri-apps/api/event';
import { native,listProfiles,profileIdentity,listApiAccounts,addApiAccount,listChats,createChat,sendChat,
  type ApiAccount,type ChatConversation,type CreateChat,type Snapshot } from './bridge';
import { escape,billingLabel } from './presentation';
import { initializeChatTransfers } from './chat-transfer-ui';
import { initializeChatRecovery } from './chat-recovery-ui';

export async function initializeChat():Promise<void>{
  if(!native)return;
  const host=document.createElement('section');host.className='workbench chat-workbench';
  host.innerHTML=`<details><summary>Conversar na Capy</summary>
    <details><summary>Adicionar conta API</summary><form id="chatApiForm">
      <label>Nome da chave<input name="label" required maxlength="80"></label>
      <label>Provedor<select name="provider"><option>OpenAI</option><option>Anthropic</option><option>Gemini</option></select></label>
      <label>Chave API<input name="key" type="password" required minlength="8" maxlength="2048" autocomplete="off" spellcheck="false"></label>
      <p>A chave fica no cofre do Windows. API gera cobrança por uso na conta do provedor.</p><button>Salvar chave</button></form></details>
    <details><summary>Conta e modelo de destino</summary><form id="chatCreateForm">
      <label>Nome para nova conversa<input name="title" required maxlength="80"></label><label>Conta<select name="target" required></select></label>
      <label>Modelo<input name="model" required maxlength="128" placeholder="Nome ou identificador do modelo"></label>
      <p id="chatBilling" role="status">Verifique a conta antes de criar a conversa.</p>
      <div class="actions"><button type="button" id="chatVerify">Verificar conta e cobrança</button><button type="submit" disabled>Criar conversa</button><button type="button" id="chatPrepareTransfer" disabled>Preparar transferência da conversa selecionada</button></div></form></details>
    <label>Conversa<select id="chatSelection"></select></label><section id="chatBody" hidden aria-label="Conversa selecionada">
      <p id="chatIdentity"></p><div id="chatMessages" class="chat-messages"></div><section id="chatRecovery" hidden></section><form id="chatSendForm">
      <label>Modelo para a próxima mensagem<input name="model" required maxlength="128"></label>
      <label>Mensagem<textarea name="text" required rows="4" maxlength="32768"></textarea></label><button>Enviar mensagem</button></form></section>
    <section id="chatReview" class="handoff-reviews" hidden></section><p id="chatNotice" role="status" aria-live="polite"></p></details>`;
  document.getElementById('sessions')!.before(host);
  const find=<T extends HTMLElement>(id:string)=>host.querySelector<T>(`#${id}`)!;
  const field=<T extends HTMLElement>(form:HTMLFormElement,name:string)=>form.elements.namedItem(name) as T;
  const apiForm=find<HTMLFormElement>('chatApiForm'),newForm=find<HTMLFormElement>('chatCreateForm'),sendForm=find<HTMLFormElement>('chatSendForm');
  const target=field<HTMLSelectElement>(newForm,'target'),selection=find<HTMLSelectElement>('chatSelection'),notice=find('chatNotice');
  const createButton=newForm.querySelector<HTMLButtonElement>('[type="submit"]')!,verify=find<HTMLButtonElement>('chatVerify');
  let apis:ApiAccount[]=[],chats:ChatConversation[]=[],selected='',sending=false,verifiedTarget='',openEpoch=0;
  let verified:Pick<CreateChat,'expectedAccount'|'expectedBilling'|'credentialRevision'>|null=null;
  const drafts=new Map<string,{text:string;model:string}>();
  const recovery=initializeChatRecovery(host,()=>chats.find(c=>c.id===selection.value),update);
  const transfers=initializeChatTransfers(host,()=>{
    const chat=chats.find(c=>c.id===selection.value),split=target.value.indexOf(':');
    return {chat,request:verified&&verifiedTarget===target.value?{title:chat?.title??'',kind:target.value.slice(0,split) as CreateChat['kind'],profileId:target.value.slice(split+1),model:field<HTMLInputElement>(newForm,'model').value.trim(),...verified}:null};
  },destination=>{update(destination);selection.value=destination.id;render();});
  const fail=(error:unknown)=>{notice.textContent=String(error);};
  const sourceWarning=document.createElement('p');sourceWarning.setAttribute('role','status');host.append(sourceWarning);
  const reset=()=>{verified=null;createButton.disabled=true;find('chatBilling').textContent='Verifique a conta antes de criar a conversa.';transfers.refresh();};
  async function choices(){
    const previous=target.value;
    const [profileResult,apiResult]=await Promise.allSettled([listProfiles(),listApiAccounts()]);
    const profiles=profileResult.status==='fulfilled'?profileResult.value:[];
    apis=apiResult.status==='fulfilled'?apiResult.value:[];
    sourceWarning.textContent=[profileResult,apiResult].filter(result=>result.status==='rejected').map(result=>String(result.reason)).join(' ');
    target.innerHTML=profiles.map(p=>`<option value="claudeCli:${escape(p.id)}">CLI Claude · ${escape(p.label)}</option>`).join('')
      +apis.filter(a=>a.configured).map(a=>`<option value="api:${escape(a.account.id)}">API ${escape(a.account.provider)} · ${escape(a.account.label)}</option>`).join('');
    if([...target.options].some(o=>o.value===previous))target.value=previous;reset();
  }
  function render(){
    const id=selection.value;
    selection.innerHTML='<option value="">Escolha uma conversa</option>'+chats.map(c=>`<option value="${escape(c.id)}">${escape(c.title)} · ${escape(c.target.provider)}</option>`).join('');
    if(chats.some(c=>c.id===id))selection.value=id;
    const chat=chats.find(c=>c.id===selection.value);find('chatBody').hidden=!chat;
    if(!chat){selected='';recovery.refresh();transfers.refresh();return;}
    if(selected!==chat.id){const draft=drafts.get(chat.id);field<HTMLInputElement>(sendForm,'model').value=draft?.model??chat.model;field<HTMLTextAreaElement>(sendForm,'text').value=draft?.text??'';selected=chat.id;}
    find('chatIdentity').textContent=`${chat.target.provider} · ${chat.target.account} · ${billingLabel(chat.target.billing)}${chat.target.kind==='api'?' · nome local da chave':''}`;
    const messages=find('chatMessages');messages.replaceChildren();
    if(!chat.messages.length){const p=document.createElement('p');p.textContent='Conversa criada. Envie a primeira mensagem quando estiver pronto.';messages.append(p);}
    for(const [index,message] of chat.messages.entries()){
      const article=document.createElement('article');article.className=`chat-message ${message.role}`;
      const h=document.createElement('h3');h.textContent=message.role==='user'?'Você':chat.target.provider;
      const content=document.createElement('div');content.textContent=message.text;article.append(h,content);
      if(chat.interruptions.some(entry=>entry.messageIndex===index)){
        const warning=document.createElement('p');warning.textContent='Resposta deste envio indisponível. Revisado sem reenvio; o consumo pode ter ocorrido.';article.append(warning);
      }
      messages.append(article);
    }
    sendForm.querySelector<HTMLButtonElement>('button')!.disabled=sending||['working','unknown','transferred'].includes(chat.state)||(chat.target.kind==='claudeCli'&&chat.cliAttempted&&!chat.cliStarted);
    notice.textContent=chat.state==='working'?'Aguardando resposta…':chat.lastError??(chat.state==='completed'?'Resposta concluída.':'');
    recovery.refresh();transfers.refresh();
  }
  function update(chat:ChatConversation){const index=chats.findIndex(c=>c.id===chat.id);if(index>=0&&chats[index].revision>chat.revision)return;if(index>=0)chats[index]=chat;else chats.push(chat);render();}
  target.addEventListener('change',reset);selection.addEventListener('change',render);
  sendForm.addEventListener('input',()=>{if(selection.value)drafts.set(selection.value,{text:field<HTMLTextAreaElement>(sendForm,'text').value,model:field<HTMLInputElement>(sendForm,'model').value});});
  window.addEventListener('capy-profiles',()=>{void choices().catch(fail);});
  verify.addEventListener('click',()=>{
    const value=target.value;verify.disabled=true;reset();
    void(async()=>{
      const split=value.indexOf(':'),kind=value.slice(0,split),id=value.slice(split+1);
      if(kind==='claudeCli'){
        const identity=await profileIdentity(id);if(!identity.loggedIn||!identity.account||!['subscription','api'].includes(identity.billing))throw new Error('Faça login no perfil Claude e confirme sua identidade.');
        if(target.value!==value)return;verified={expectedAccount:identity.account,expectedBilling:identity.billing as 'subscription'|'api',credentialRevision:null};
      }else{
        apis=await listApiAccounts();const api=apis.find(a=>a.account.id===id&&a.configured);if(!api)throw new Error('Chave não encontrada no cofre do Windows.');
        if(target.value!==value)return;verified={expectedAccount:api.account.label,expectedBilling:'api',credentialRevision:api.account.revision};
      }
      verifiedTarget=value;createButton.disabled=false;find('chatBilling').textContent=`${verified.expectedAccount} · ${billingLabel(verified.expectedBilling)}${kind==='api'?' · chave cadastrada; autenticação pelo provedor no envio':''}`;
      transfers.refresh();
    })().catch(fail).finally(()=>{verify.disabled=false;});
  });
  apiForm.addEventListener('submit',event=>{
    event.preventDefault();if(!apiForm.reportValidity())return;const button=apiForm.querySelector<HTMLButtonElement>('button')!;button.disabled=true;
    const keyField=field<HTMLInputElement>(apiForm,'key'),key=keyField.value;keyField.value='';
    void addApiAccount(field<HTMLInputElement>(apiForm,'label').value.trim(),field<HTMLSelectElement>(apiForm,'provider').value as ApiAccount['account']['provider'],key)
      .then(async()=>{await choices();notice.textContent='Chave salva no Windows. Selecione a conta para criar uma conversa.';}).catch(fail).finally(()=>{button.disabled=false;});
  });
  newForm.addEventListener('submit',event=>{
    event.preventDefault();if(!newForm.reportValidity()||!verified||verifiedTarget!==target.value)return;createButton.disabled=true;const split=target.value.indexOf(':');
    void createChat({title:field<HTMLInputElement>(newForm,'title').value.trim(),model:field<HTMLInputElement>(newForm,'model').value.trim(),kind:target.value.slice(0,split) as CreateChat['kind'],profileId:target.value.slice(split+1),...verified})
      .then(chat=>{update(chat);selection.value=chat.id;render();field<HTMLTextAreaElement>(sendForm,'text').focus();}).catch(fail).finally(()=>{createButton.disabled=!verified;});
  });
  sendForm.addEventListener('submit',event=>{
    event.preventDefault();const chat=chats.find(c=>c.id===selection.value);
    if(!chat||sending||['working','unknown','transferred'].includes(chat.state)||(chat.target.kind==='claudeCli'&&chat.cliAttempted&&!chat.cliStarted)||!sendForm.reportValidity())return;
    const text=field<HTMLTextAreaElement>(sendForm,'text'),submitted=text.value;sending=true;render();notice.textContent='Verificando a conta e enviando…';
    void sendChat({id:chat.id,revision:chat.revision,nonce:crypto.randomUUID(),model:field<HTMLInputElement>(sendForm,'model').value.trim(),text:submitted})
      .then(result=>{const draft=drafts.get(chat.id);if(!draft||draft.text===submitted)drafts.delete(chat.id);if(selection.value===chat.id&&text.value===submitted)text.value='';update(result);})
      .catch(async error=>{try{chats=await listChats();}catch{}render();fail(error);})
      .finally(()=>{sending=false;const message=notice.textContent;render();notice.textContent=message;});
  });
  await listen<ChatConversation>('chat-updated',event=>update(event.payload));
  await listen<Snapshot>('demo-updated',event=>{if(event.payload.scenario==='real')transfers.sync(event.payload.chatTransfers??[]);});
  await listen<string>('chat-selected',event=>{
    const epoch=++openEpoch;
    void listChats().then(values=>{
      if(epoch!==openEpoch)return;
      chats=values;render();
      if(!chats.some(chat=>chat.id===event.payload)){fail('Conversa não encontrada. Atualize o painel.');return;}
      selection.value=event.payload;render();
      host.querySelector<HTMLDetailsElement>(':scope > details')!.open=true;
      host.scrollIntoView({block:'start'});field<HTMLTextAreaElement>(sendForm,'text').focus();
    }).catch(fail);
  });
  try { await choices();for(const chat of await listChats())update(chat);render(); }
  catch(error) { fail(error); }
}
