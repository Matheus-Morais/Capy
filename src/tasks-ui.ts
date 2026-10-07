import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';
import { native, listProfiles, profileIdentity, showError, type AccountProfile, type AccountIdentity } from './bridge';
import { billingLabel, escape, exitConfirmation } from './presentation';
import type {ExitReview} from './exit-review';

interface Task { id:string; profileId:string; account:string|null; billing:string; model:string; cwd:string; prompt:string; mode:string; createdAt:number }
interface Chunk { taskId:string; sequence:number; data:number[] }
interface Replay { chunks:Chunk[]; exited:boolean }

export async function initializeTasks():Promise<void>{
  const host=document.createElement('section');host.className='workbench';
  host.innerHTML='<details id="newTask"><summary>Nova tarefa</summary><form id="startTask"><label>Conta<select name="profile" required></select></label><label>Pasta do projeto<input name="cwd" required placeholder="C:\PProjetos\MeuProjeto"></label><label>Modelo<input name="model" required value="sonnet" list="claudeModels" maxlength="128"><datalist id="claudeModels"><option value="sonnet"><option value="opus"><option value="haiku"></datalist></label><label>Abrir em<select name="mode"><option value="external">CLI externo · padrão</option><option value="embedded">Terminal integrado da Capy</option></select></label><label>Instrução<textarea name="prompt" required rows="4" maxlength="32768" placeholder="O que o agente deve fazer neste projeto?"></textarea></label><p id="taskBilling" role="status">Verifique a conta nesta pasta antes de iniciar.</p><div class="actions"><button type="button" id="verifyTaskAccount">Verificar conta e cobrança</button><button type="submit" class="primary" disabled>Iniciar tarefa</button></div></form></details><div id="managedTasks"></div><section id="terminalSection" hidden><div class="terminal-heading"><h2 id="terminalTitle">Terminal integrado</h2><button id="closeTerminal" aria-label="Ocultar terminal integrado">Ocultar</button></div><p id="terminalIdentity"></p><div class="actions"><button id="interruptTerminal">Interromper turno</button><button id="terminalModel">Escolher modelo no Claude</button></div><div id="terminalViewport" aria-label="Terminal interativo da sessão selecionada"></div><p id="terminalNotice" role="status"></p></section>';
  document.getElementById('sessions')!.before(host);
  const form=document.getElementById('startTask') as HTMLFormElement;
  const profileSelect=form.elements.namedItem('profile') as HTMLSelectElement;
  const cwd=form.elements.namedItem('cwd') as HTMLInputElement;
  const startButton=form.querySelector<HTMLButtonElement>('[type="submit"]')!;
  let profiles:AccountProfile[]=[];
  let identity:AccountIdentity|null=null;
  let verifiedKey='';let tasks:Task[]=[];let taskSignature='';
  const updateProfiles=(values:AccountProfile[])=> {
    profiles=values;const previous=profileSelect.value;
    profileSelect.innerHTML=profiles.map(p=>`<option value="${escape(p.id)}">${escape(p.label)} · ${escape(p.provider)}</option>`).join('');
    if(profiles.some(p=>p.id===previous))profileSelect.value=previous;
    if(previous&&profileSelect.value!==previous)resetIdentity();
    host.querySelectorAll<HTMLSelectElement>('[data-prepare-handoff] select[name="destination"]').forEach(select=>{
      const selected=select.value;
      select.innerHTML=profiles.map(p=>`<option value="${escape(p.id)}">${escape(p.label)}</option>`).join('');
      if(profiles.some(p=>p.id===selected))select.value=selected;
    });
  };
  window.addEventListener('capy-profiles',event=>updateProfiles((event as CustomEvent<AccountProfile[]>).detail));
  const resetIdentity=()=>{identity=null;verifiedKey='';startButton.disabled=true;document.getElementById('taskBilling')!.textContent='Verifique a conta nesta pasta antes de iniciar.';};
  cwd.addEventListener('input',resetIdentity);profileSelect.addEventListener('change',resetIdentity);
  document.getElementById('verifyTaskAccount')!.addEventListener('click',()=>{
    if(!cwd.reportValidity())return;
    const button=document.getElementById('verifyTaskAccount') as HTMLButtonElement;button.disabled=true;
    const key=JSON.stringify([profileSelect.value,cwd.value]);
    void profileIdentity(profileSelect.value,cwd.value.trim()).then(result=>{
      if(key!==JSON.stringify([profileSelect.value,cwd.value]))return;
      identity=result;verifiedKey=key;
      document.getElementById('taskBilling')!.textContent=`${result.account??'Conta do perfil selecionado'} · ${billingLabel(result.billing)} · ${result.message}`;
      startButton.disabled=!result.loggedIn;
    }).catch(showError).finally(()=>button.disabled=false);
  });
  form.addEventListener('submit',event=>{
    event.preventDefault();if(!identity||verifiedKey!==JSON.stringify([profileSelect.value,cwd.value])||!form.reportValidity())return;
    const values=new FormData(form);startButton.disabled=true;
    const request={profileId:profileSelect.value,model:String(values.get('model')).trim(),cwd:cwd.value.trim(),prompt:String(values.get('prompt')),mode:String(values.get('mode')),expectedAccount:identity.account,expectedBilling:identity.billing};
    void invoke<Task>('start_task',{request}).then(async task=>{
      document.getElementById('notice')!.textContent=`Tarefa iniciada · ${billingLabel(task.billing)} · ${task.account??'conta do perfil'}.`;
      await refresh();if(task.mode==='embedded')await attach(task);
    }).catch(showError).finally(()=>startButton.disabled=!identity?.loggedIn);
  });
  const term=new Terminal({fontFamily:'Consolas, monospace',fontSize:12,scrollback:2000,cursorBlink:false,theme:{background:'#272b24',foreground:'#f3ecd9',cursor:'#e2b98f',selectionBackground:'#566648',black:'#272b24',brightBlack:'#b9b09b'}});
  const fit=new FitAddon();term.loadAddon(fit);
  let opened=false;let attached:Task|null=null;let lastSequence=0;let replaying=false;let queued:Chunk[]=[];let attachment=0;let terminalExited=false;
  const section=document.getElementById('terminalSection')!;
  const consume=(chunk:Chunk)=>{if(chunk.taskId!==attached?.id||chunk.sequence<=lastSequence)return;lastSequence=chunk.sequence;term.write(new Uint8Array(chunk.data));};
  const resize=()=>{if(!opened||section.hidden||!attached||terminalExited)return;fit.fit();void invoke('terminal_resize',{id:attached.id,cols:Math.max(10,Math.min(400,term.cols)),rows:Math.max(2,Math.min(200,term.rows))}).catch(showError);};
  const attach=async(task:Task)=>{
    const token=++attachment;
    attached=task;lastSequence=0;queued=[];replaying=true;terminalExited=false;
    section.hidden=false;
    section.scrollIntoView({block:'start'});
    document.getElementById('terminalTitle')!.textContent=`Claude · ${task.model}`;
    document.getElementById('terminalIdentity')!.textContent=`${task.cwd} · ${task.account??'conta do perfil'} · ${billingLabel(task.billing)} · sessão ${task.id}`;
    if(!opened){term.open(document.getElementById('terminalViewport')!);opened=true;}else term.reset();
    try{
      const replay=await invoke<Replay>('terminal_replay',{id:task.id});
      if(attachment!==token)return;
      replay.chunks.forEach(consume);queued.sort((a,b)=>a.sequence-b.sequence).forEach(consume);
      terminalExited ||= replay.exited;
      document.getElementById('terminalNotice')!.textContent=terminalExited?'O CLI encerrou. Sua conversa pode ser retomada pelo identificador.':'As teclas e respostas vão somente para esta sessão.';
      (document.getElementById('interruptTerminal') as HTMLButtonElement).disabled=terminalExited;
      (document.getElementById('terminalModel') as HTMLButtonElement).disabled=terminalExited;
      resize();term.focus();
    }finally{if(attachment===token){replaying=false;queued=[];}}
  };
  term.onData(data=>{if(attached&&!terminalExited)void invoke('terminal_input',{id:attached.id,data}).catch(showError);});
  document.getElementById('closeTerminal')!.addEventListener('click',()=>{section.hidden=true;});
  document.getElementById('interruptTerminal')!.addEventListener('click',()=>{
    if(!attached||!window.confirm(`Interromper o turno de Claude na sessão ${attached.id}, em ${attached.cwd}? O trabalho feito até aqui permanece. Em um diálogo de permissão, isso recusa o pedido.`))return;
    void invoke('terminal_interrupt',{id:attached.id,confirmed:true}).then(()=>{document.getElementById('terminalNotice')!.textContent='Interrupção solicitada pelo controle oficial Esc.';}).catch(showError);
  });
  document.getElementById('terminalModel')!.addEventListener('click',()=>{
    if(attached)void invoke('terminal_model_picker',{id:attached.id}).then(()=>term.focus()).catch(showError);
  });
  host.addEventListener('click',event=>{
    const button=(event.target as Element).closest<HTMLButtonElement>('[data-task-action]');if(!button||button.disabled)return;
    const task=tasks.find(t=>t.id===button.dataset.taskId);if(!task)return;
    if(button.dataset.taskAction==='view'){void attach(task).catch(showError);}else{
      button.disabled=true;void invoke('resume_task',{id:task.id}).then(()=>{document.getElementById('notice')!.textContent='Conversa exata solicitada em um novo terminal.';}).catch(showError).finally(()=>button.disabled=false);
    }
  });
  host.addEventListener('submit',event=>{
    const form=(event.target as Element).closest<HTMLFormElement>('[data-prepare-handoff]');if(!form)return;
    event.preventDefault();if(!form.reportValidity())return;
    const button=form.querySelector<HTMLButtonElement>('[type="submit"]')!;button.disabled=true;
    const values=new FormData(form);
    void invoke('prepare_handoff',{sourceId:form.dataset.prepareHandoff,destinationId:String(values.get('destination')),model:String(values.get('model')).trim()}).then(()=>{document.getElementById('notice')!.textContent='Resumo preparado. Revise os campos antes de aprovar a continuação.';}).catch(showError).finally(()=>button.disabled=false);
  });
  const refresh=async()=>{
    tasks=await invoke<Task[]>('list_tasks');const signature=JSON.stringify(tasks.map(t=>t.id));
    if(taskSignature===signature)return;taskSignature=signature;
    document.getElementById('managedTasks')!.innerHTML=tasks.slice(-20).reverse().map(task=>`<div class="managed-task"><h3>${escape(task.cwd.split(/[\\/]/).filter(Boolean).pop()??task.cwd)} · ${escape(task.model)}</h3><p>${escape(task.account??'Conta do perfil')} · ${escape(billingLabel(task.billing))} · ${task.mode==='embedded'?'Terminal integrado':'CLI externo'}</p><p class="session-id">${escape(task.id)}</p><div class="actions">${task.mode==='embedded'?`<button data-task-action="view" data-task-id="${task.id}">Ver terminal integrado</button>`:''}<button data-task-action="resume" data-task-id="${task.id}">Retomar conversa encerrada</button></div><details><summary>Continuar em outra conta ou modelo</summary><form data-prepare-handoff="${task.id}"><label>Conta de destino<select name="destination" required>${profiles.map(p=>`<option value="${escape(p.id)}">${escape(p.label)}</option>`).join('')}</select></label><label>Modelo de destino<input name="model" value="${escape(task.model)}" required maxlength="128"></label><button type="submit">Preparar resumo para revisão</button></form></details></div>`).join('');
    if(!cwd.value&&tasks.length)cwd.value=tasks[tasks.length-1].cwd;
  };
  if(!native){form.querySelectorAll<HTMLInputElement|HTMLButtonElement|HTMLSelectElement|HTMLTextAreaElement>('input,button,select,textarea').forEach(element=>element.disabled=true);return;}
  await listen<Chunk>('terminal-output',event=>{if(event.payload.taskId!==attached?.id)return;if(replaying)queued.push(event.payload);else consume(event.payload);});
  await listen<string>('terminal-exited',event=>{if(event.payload===attached?.id){terminalExited=true;document.getElementById('terminalNotice')!.textContent='O CLI encerrou; isso não confirma conclusão da tarefa.';(document.getElementById('interruptTerminal') as HTMLButtonElement).disabled=true;(document.getElementById('terminalModel') as HTMLButtonElement).disabled=true;}});
  await listen<ExitReview>('confirm-exit',event=>{
    const review=event.payload;
    if(window.confirm(exitConfirmation(review)))void invoke('confirm_exit',{nonce:review.nonce,confirmed:true}).catch(showError);
    else void invoke('cancel_exit_review',{nonce:review.nonce}).catch(showError);
  });
  await listen<string>('exit-review-error',event=>showError(event.payload));
  new ResizeObserver(resize).observe(document.getElementById('terminalViewport')!);
  try{updateProfiles(await listProfiles());}catch(error){showError(error);}
  await refresh();setInterval(()=>void refresh().catch(showError),5000);
}
