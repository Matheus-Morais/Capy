import {chatTransferReview,prepareChatTransfer,approveChatTransfer,cancelChatTransfer,type ChatConversation,type CreateChat,type ChatTransferReview,type HandoffSummary} from './bridge';
import {chatTransferForm,summaryLabels} from './presentation';

export function initializeChatTransfers(host:HTMLElement,context:()=>{chat:ChatConversation|undefined;request:CreateChat|null},onDestination:(chat:ChatConversation)=>void){
  const view=host.querySelector<HTMLElement>('#chatReview')!,prepare=host.querySelector<HTMLButtonElement>('#chatPrepareTransfer')!,notice=host.querySelector<HTMLElement>('#chatNotice')!;
  let review:ChatTransferReview|null=null,signature='',epoch=0,busy=false;
  const error=(value:unknown)=>{notice.textContent=String(value);};
  const render=(value:ChatTransferReview|null)=>{review=value;view.hidden=!value;view.innerHTML=value?chatTransferForm(value):'';};
  prepare.addEventListener('click',()=>{
    const {chat,request}=context();if(!chat||!request||busy)return;
    if(!host.querySelector<HTMLInputElement>('#chatCreateForm input[name="model"]')!.reportValidity())return;
    busy=true;prepare.disabled=true;const current=++epoch;
    void prepareChatTransfer(chat.id,request).then(value=>{
      if(current!==epoch||context().chat?.id!==chat.id)return;
      render(value);view.scrollIntoView({block:'start'});view.querySelector<HTMLTextAreaElement>('textarea')!.focus();
    }).catch(error).finally(()=>{busy=false;prepare.disabled=!context().request||!context().chat||['working','unknown','transferred'].includes(context().chat!.state);});
  });
  view.addEventListener('submit',event=>{
    event.preventDefault();const form=event.target as HTMLFormElement;
    if(!review||busy||!form.reportValidity())return;
    const selected=review,values=new FormData(form);
    const summary=Object.fromEntries(Object.keys(summaryLabels).map(key=>[key,String(values.get(key)??'').trim()])) as unknown as HandoffSummary;
    const button=form.querySelector<HTMLButtonElement>('[type="submit"]')!;busy=true;button.disabled=true;
    notice.textContent='Verificando identidades e preparando o destino aprovado…';
    void approveChatTransfer(selected.sourceId,selected.nonce,summary,values.get('reviewed')==='on',values.get('billingConfirmed')==='on')
      .then(destination=>{render(null);onDestination(destination);if(destination.state==='failed')notice.textContent=destination.lastError??'O destino falhou. Revise o histórico antes de preparar outra transferência.';})
      .catch(error).finally(()=>{busy=false;button.disabled=false;});
  });
  view.addEventListener('click',event=>{
    const button=(event.target as Element).closest<HTMLButtonElement>('[data-cancel-chat-transfer]');if(!button||!review||busy)return;
    busy=true;button.disabled=true;const selected=review;
    void cancelChatTransfer(selected.sourceId,selected.nonce).then(()=>{render(null);notice.textContent='Transferência cancelada; nenhum resumo foi enviado.';}).catch(error).finally(()=>{busy=false;button.disabled=false;});
  });
  return {refresh(){
    const {chat,request}=context();prepare.disabled=busy||!request||!chat||['working','unknown','transferred'].includes(chat.state);
    const next=chat?`${chat.id}:${chat.revision}`:'';if(next===signature)return;signature=next;
    const current=++epoch;render(null);if(!chat||['working','unknown','transferred'].includes(chat.state))return;
    void chatTransferReview(chat.id).then(value=>{if(current===epoch)render(value);}).catch(value=>{if(current===epoch)error(value);});
  }};
}
