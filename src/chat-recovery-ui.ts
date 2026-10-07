import {prepareChatRecovery,approveChatRecovery,type ChatConversation} from './bridge';
import {chatRecoveryForm} from './presentation';

export function initializeChatRecovery(host:HTMLElement,context:()=>ChatConversation|undefined,onUpdate:(chat:ChatConversation)=>void){
  const view=host.querySelector<HTMLElement>('#chatRecovery')!,notice=host.querySelector<HTMLElement>('#chatNotice')!;
  let signature='',busy=false;
  const error=(value:unknown)=>{notice.textContent=String(value);};
  view.addEventListener('click',event=>{
    const button=(event.target as Element).closest<HTMLButtonElement>('[data-prepare-chat-recovery]'),chat=context();
    if(!button||!chat||chat.state!=='unknown'||busy)return;
    busy=true;button.disabled=true;notice.textContent='Conferindo a conta e o histórico da sessão…';
    void prepareChatRecovery(chat.id,chat.revision).then(updated=>{
      onUpdate(updated);
      if(context()?.id===chat.id){view.scrollIntoView({block:'start'});view.querySelector<HTMLInputElement>('input')?.focus();}
    }).catch(error).finally(()=>{busy=false;button.disabled=false;});
  });
  view.addEventListener('submit',event=>{
    event.preventDefault();const form=event.target as HTMLFormElement,chat=context(),review=chat?.recoveryReview;
    if(!chat||!review||busy||form.dataset.chatRecovery!==review.nonce||!form.reportValidity())return;
    busy=true;const button=form.querySelector<HTMLButtonElement>('button')!;button.disabled=true;
    notice.textContent='Confirmando esta revisão; nenhuma mensagem será enviada…';
    void approveChatRecovery(chat.id,chat.revision,review.nonce,new FormData(form).get('reviewed')==='on')
      .then(onUpdate).catch(error).finally(()=>{busy=false;button.disabled=false;});
  });
  return {refresh(){
    const chat=context(),next=chat?`${chat.id}:${chat.revision}`:'';
    if(next===signature)return;signature=next;
    view.hidden=chat?.state!=='unknown';view.innerHTML=chat?chatRecoveryForm(chat):'';
  }};
}
