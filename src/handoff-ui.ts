import { invoke } from '@tauri-apps/api/core';
import { desktopCommand, showError, type Snapshot, type HandoffReview, type HandoffSummary } from './bridge';
import { billingLabel,escape } from './presentation';

let signature='\0';let latest:HandoffReview[]=[];
export function renderHandoffs(data:Snapshot):void{
  latest=data.handoffs??[];
  let host=document.getElementById('handoffReviews');
  if(!host){
    host=document.createElement('section');host.id='handoffReviews';host.className='handoff-reviews';
    document.getElementById('sessions')!.before(host);
    host.addEventListener('click',event=>{
      const button=(event.target as Element).closest<HTMLButtonElement>('[data-handoff-action]');if(!button||button.disabled)return;
      if(button.dataset.handoffAction==='panel'){void desktopCommand('show_panel').catch(showError);return;}
      const nonce=button.closest<HTMLElement>('[data-handoff]')!.dataset.handoff!;
      if(button.dataset.handoffAction==='cancel'){button.disabled=true;void invoke('cancel_handoff',{nonce}).catch(showError).finally(()=>button.disabled=false);}
    });
    host.addEventListener('submit',event=>{
      event.preventDefault();const form=event.target as HTMLFormElement;if(!form.reportValidity())return;
      const review=latest.find(r=>r.nonce===form.dataset.handoff);if(!review)return;
      const values=new FormData(form);
      const summary=Object.fromEntries(['objective','decisions','state','files','tests','nextSteps','guides'].map(key=>[key,String(values.get(key)).trim()])) as unknown as HandoffSummary;
      const button=form.querySelector<HTMLButtonElement>('[type="submit"]')!;button.disabled=true;
      void invoke('approve_handoff',{nonce:review.nonce,summary,billingConfirmed:values.get('billingConfirmed')==='on',mode:String(values.get('mode'))}).then(()=>{document.getElementById('notice')!.textContent='Continuação iniciada na conta e modelo aprovados nesta revisão.';}).catch(showError).finally(()=>button.disabled=false);
    });
  }
  const next=latest.map(r=>r.nonce).join(',');
  if(next===signature)return;signature=next;
  host.hidden=!latest.length;
  if(document.body.dataset.surface!=='panel'){
    host.innerHTML=latest.map(r=>`<div data-handoff="${r.nonce}"><h2>Continuação pronta para revisão</h2><p>${escape(r.destinationAccount??'Conta de destino')} · ${escape(r.model)} · ${escape(billingLabel(r.destinationBilling))}</p><button data-handoff-action="panel">Revisar resumo no painel</button></div>`).join('');return;
  }
  const labels:Record<keyof HandoffSummary,string>={objective:'Objetivo',decisions:'Decisões e regras',state:'Estado atual',files:'Arquivos alterados',tests:'Testes e resultados',nextSteps:'Próximos passos',guides:'Planos e arquivos .md usados'};
  host.innerHTML=latest.map(r=>`<form data-handoff="${r.nonce}"><h2>${r.automatic?'Percentual de troca atingido':'Continuar em outra conta ou modelo'}</h2><p>Revise o contexto antes de enviar. Origem: sessão ${escape(r.sourceTaskId)}.</p><p><strong>Destino: ${escape(r.destinationAccount??'Conta do perfil')} · ${escape(r.model)} · ${escape(billingLabel(r.destinationBilling))}</strong></p>${Object.entries(labels).map(([key,label])=>`<label>${label}<textarea name="${key}" rows="${key==='decisions'||key==='guides'?5:3}" required maxlength="65536">${escape(r.summary[key as keyof HandoffSummary])}</textarea></label>`).join('')}<label>Abrir continuação em<select name="mode"><option value="external">CLI externo · padrão</option><option value="embedded">Terminal integrado da Capy</option></select></label>${r.sourceBilling!==r.destinationBilling?`<label class="billing-confirm"><input type="checkbox" name="billingConfirmed" required>Confirmo a mudança de ${escape(billingLabel(r.sourceBilling))} para ${escape(billingLabel(r.destinationBilling))}, somente nesta transferência.</label>`:''}<div class="actions"><button type="submit" class="primary">Aprovar resumo e iniciar continuação</button><button type="button" data-handoff-action="cancel">Cancelar transferência</button></div></form>`).join('');
}
