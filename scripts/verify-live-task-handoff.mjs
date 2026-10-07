import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
import {readFile,writeFile} from 'node:fs/promises';
import {join} from 'node:path';

export function taskHandoffProof({panel,root,task,profile,check,waitFor,taskHistory,taskProcess,sameFolder}){
  async function stableBoundary(){
    const boundaryPath=join(profile.configDir,'capy-activity',`${task.id}.boundary.json`);
    let atMs,lastChanged=0;
    return waitFor(async()=>{
      const value=await readFile(boundaryPath,'utf8').then(JSON.parse).catch(()=>null);
      if(!value||value.session_id!==task.id||!['StopFailure','idle_prompt'].includes(value.event))return null;
      if(value.at_ms!==atMs){atMs=value.at_ms;lastChanged=Date.now();return null;}
      return Date.now()-lastChanged>=1_500?value:null;
    },'Boundary Claude estável antes da revisão',120_000);
  }
  async function verify(){
    await panel.invoke('demo_action',{action:'scenario',id:'',answer:'real'});
    const existing=await panel.invoke('list_tasks');assert.equal(existing.length,1);assert.equal(existing[0].id,task.id);
    await stableBoundary();
    let review=await waitFor(()=>panel.invoke('prepare_handoff',{sourceId:task.id,destinationId:profile.id,model:'haiku'}).catch(()=>null),'Revisão de handoff após boundary de turno Claude',30_000);
    const marker=`CAPY_HANDOFF_${randomUUID()}`;
    check('native_handoff_review_pins_source_destination_and_billing',review.sourceTaskId===task.id&&review.destinationProfileId===profile.id&&review.destinationAccount===task.account&&review.sourceBilling==='subscription'&&review.destinationBilling==='subscription'&&review.model==='haiku');
    check('native_handoff_review_contains_actual_source_instruction',review.summary.objective.includes(task.prompt)||review.summary.decisions.includes(task.prompt));
    let destination,staleReviewObserved=false;
    for(let attempt=0;attempt<3&&!destination;attempt++){
      const form=`form[data-handoff="${review.nonce}"]`;
      await waitFor(()=>panel.evaluate(`!!document.querySelector(${JSON.stringify(form)})`),'Formulário revisável de handoff');
      const revised={
        objective:`Prova de transferência: a tarefa anterior respondeu literalmente ${task.prompt.match(/CAPY_TASK_[A-Fa-f0-9-]+/)?.[0]??'o marcador da origem'}. Esse turno está concluído.`,
        decisions:`A instrução da origem se aplicava ao turno anterior. A instrução vigente desta continuação é responder somente com o marcador definido em Próximos passos.`,
        state:'O resumo revisado é uma prova curta de que a nova sessão recebeu e seguiu a instrução aprovada.',
        files:'Nenhum arquivo deve ser aberto ou alterado nesta prova.',
        tests:'A tarefa da origem já respondeu com seu marcador literal. Agora verifique o marcador da continuação.',
        nextSteps:`Responda somente com o texto literal: ${marker}. Não use ferramentas.`,
        guides:'Nenhuma leitura de arquivo é necessária; a resposta deve conter somente o marcador de Próximos passos.',
      };
      await panel.evaluate(`(()=>{const f=document.querySelector(${JSON.stringify(form)});for(const [name,value] of Object.entries(${JSON.stringify(revised)})){const field=f.elements.namedItem(name);field.value=value;field.dispatchEvent(new Event('input',{bubbles:true}));}f.elements.namedItem('mode').value='embedded';f.querySelector('textarea[name="nextSteps"]').scrollIntoView({block:'center'});return true;})()`);
      if(attempt===0){
        check('native_handoff_review_editable_and_uses_integrated_destination',await panel.evaluate(`(()=>{const f=document.querySelector(${JSON.stringify(form)});return Object.entries(${JSON.stringify(revised)}).every(([name,value])=>f.elements.namedItem(name).value===value)&&f.elements.namedItem('mode').value==='embedded'&&!f.querySelector('[name="billingConfirmed"]');})()`));
        await writeFile(join(root,'task-handoff-review.json'),JSON.stringify({review,revised,marker},null,2));await panel.screenshot('task-handoff-review');
      }
      await panel.evaluate(`document.querySelector(${JSON.stringify(form)}).requestSubmit();true`);
      const deadline=Date.now()+15_000;let stale=false;
      while(Date.now()<deadline&&!destination){
        destination=(await panel.invoke('list_tasks')).find(item=>item.id!==task.id&&sameFolder(item.cwd,task.cwd));
        if(destination)break;
        const error=await panel.evaluate(`document.querySelector('#error')?.textContent??''`);
        if(error.includes('sessão voltou a trabalhar ou mudou')){stale=true;break;}
        await new Promise(resolve=>setTimeout(resolve,100));
      }
      if(destination)break;
      if(!stale)throw new Error('A aprovação não criou a continuação nem informou revisão obsoleta.');
      check('native_handoff_stale_boundary_rejects_without_creating_destination',!destination&&(await panel.invoke('list_tasks')).length===1);
      staleReviewObserved=true;
      await panel.invoke('cancel_handoff',{nonce:review.nonce});
      await stableBoundary();
      review=await waitFor(()=>panel.invoke('prepare_handoff',{sourceId:task.id,destinationId:profile.id,model:'haiku'}).catch(()=>null),'Nova revisão após boundary tardio',30_000);
    }
    if(!destination)throw new Error('A revisão de handoff não estabilizou após três aprovações próprias.');
    if(!staleReviewObserved)check('native_handoff_stale_boundary_revalidation_not_needed',true);
    check('native_handoff_approval_creates_one_new_task_with_exact_identity',destination.profileId===profile.id&&destination.account===task.account&&destination.billing==='subscription'&&destination.model==='haiku'&&destination.mode==='embedded'&&sameFolder(destination.cwd,task.cwd));
    check('native_handoff_review_nonce_consumed_once',!(await panel.invoke('list_handoffs')).some(item=>item.nonce===review.nonce));
    const replayRejected=await panel.invoke('approve_handoff',{nonce:review.nonce,summary:review.summary,billingConfirmed:false,mode:'embedded'}).then(()=>false,()=>true);
    check('native_handoff_replay_rejected_without_third_task',replayRejected&&(await panel.invoke('list_tasks')).length===2);
    await waitFor(()=>panel.evaluate(`!!document.querySelector('[data-task-action="view"][data-task-id="${destination.id}"]')`),'Destino integrado visível na lista',15_000);
    await panel.evaluate(`document.querySelector('[data-task-action="view"][data-task-id="${destination.id}"]').click();true`);
    await waitFor(()=>panel.evaluate(`document.querySelector('#terminalIdentity').textContent.includes(${JSON.stringify(destination.id)})`),'Terminal de destino anexado após aprovação',15_000);
    const destinationProcess=await waitFor(()=>taskProcess(destination,profile),'Processo Claude da sessão de destino',30_000);
    const destinationReplay=await panel.invoke('terminal_replay',{id:destination.id});
    await writeFile(join(root,'task-handoff-destination-process.json'),JSON.stringify(destinationProcess,null,2));
    await writeFile(join(root,'task-handoff-destination-replay.json'),JSON.stringify(destinationReplay,null,2));
    const destinationScreen=()=>panel.evaluate(`document.querySelector('.xterm-rows')?.textContent??''`);
    let destinationTrusted=false;
    const history=await waitFor(async()=>{
      const text=await destinationScreen();await writeFile(join(root,'task-handoff-destination-screen.txt'),text);
      if(!destinationTrusted&&/Yes,\s*I\s*trust\s*this\s*folder/i.test(text)){
        if(/❯\s*(?:\d+[.)]\s*)?No,?\s*exit/i.test(text)){await panel.invoke('terminal_input',{id:destination.id,data:'\u001b[B'});return null;}
        if(/❯\s*(?:\d+[.)]\s*)?Yes,?\s*I\s*trust\s*this\s*folder/i.test(text)){await panel.invoke('terminal_input',{id:destination.id,data:'\r'});destinationTrusted=true;}
      }
      const value=await taskHistory(destination,profile);if(!value)return null;
      const user=value.rows.find(row=>row.type==='user'&&row.sessionId===destination.id&&row.isSidechain!==true);
      const response=value.rows.find(row=>row.type==='assistant'&&row.sessionId===destination.id&&row.message?.content?.some(part=>part.type==='text'&&part.text.trim()===marker));
      return user&&response?{...value,user,response}:null;
    },'Resposta do destino no UUID próprio com resumo revisado',100_000);
    check('native_handoff_provider_receipt_matches_destination_and_reviewed_prompt',sameFolder(history.user.cwd,destination.cwd)&&JSON.stringify(history.user.message.content).includes(marker)&&history.response.message.model.toLowerCase().includes('haiku'));
    check('native_handoff_destination_does_not_execute_tools',!history.rows.some(row=>row.type==='assistant'&&row.message?.content?.some(part=>part.type==='tool_use')));
    const sourceHistory=await taskHistory(task,profile);
    check('native_handoff_keeps_source_history_without_resending',sourceHistory.rows.filter(row=>row.type==='user'&&row.sessionId===task.id).length===1);
    await writeFile(join(root,'task-handoff-receipt.json'),JSON.stringify({source:task,destination,historyPath:history.path,response:history.response},null,2));
    await panel.evaluate(`document.querySelector('#terminalSection').scrollIntoView({block:'start'});true`);await panel.screenshot('task-handoff-destination');
    return destination;
  }
  return {verify};
}
