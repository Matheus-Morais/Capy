import {randomUUID,createHash} from 'node:crypto';
import {readFile,writeFile} from 'node:fs/promises';
import {join} from 'node:path';

export async function verifyLiveTaskModel({panel,task,profile,root,check,waitFor,taskHistory}){
  const settingsPath=join(profile.configDir,'settings.json');
  const settings=async()=>{try{return await readFile(settingsPath);}catch(error){if(error.code==='ENOENT')return null;throw error;}};
  const before=await settings();
  const screen=()=>panel.evaluate(`Array.from(document.querySelector('.xterm-rows').children).map(row=>row.textContent).join(String.fromCharCode(10))`);
  await panel.evaluate(`document.querySelector('#terminalModel').click();true`);
  await waitFor(async()=>/Select.*model/i.test(await screen()),'Seletor oficial para troca efetiva');
  let selected=false;
  for(let step=0;step<10;step++){
    const text=await screen();await writeFile(join(root,`model-picker-${step}.txt`),text);
    if(/❯\s*(?:\d+[.)]\s*)?Sonnet\b/i.test(text)){selected=true;break;}
    await panel.invoke('terminal_input',{id:task.id,data:'\u001b[A'});
    await new Promise(resolve=>setTimeout(resolve,300));
  }
  check('native_model_picker_observes_sonnet_selected_before_commit',selected);
  await panel.screenshot('model-sonnet-selected');
  await panel.invoke('terminal_input',{id:task.id,data:'s'});
  await waitFor(async()=>!/Select.*model/i.test(await screen()),'Escolha oficial somente nesta sessão');
  const marker=`PROBE_${randomUUID().replaceAll('-','')}`;
  const prompt=`Responda literalmente a palavra ${marker}, sem qualquer outra palavra. Não use ferramentas.`;
  await panel.invoke('terminal_input',{id:task.id,data:prompt});
  await waitFor(async()=> (await screen()).includes(marker.slice(0,20)),'Novo prompt na mesma sessão');
  await panel.invoke('terminal_input',{id:task.id,data:'\r'});
  const receipt=await waitFor(async()=>{
    const history=await taskHistory(task,profile);
    const response=history?.rows.find(row=>row.type==='assistant'&&row.sessionId===task.id&&row.message?.model?.toLowerCase().includes('sonnet')&&row.message?.content?.some(part=>part.type==='text'&&part.text.includes(marker)));
    return response?{history,response}:null;
  },'Resposta Sonnet confirmada no UUID exato',60_000);
  check('native_model_switch_provider_receipt_confirms_same_uuid_and_sonnet',receipt.response.sessionId===task.id&&receipt.response.message.model.toLowerCase().includes('sonnet'));
  check('native_model_switch_does_not_execute_tools',!receipt.history.rows.some(row=>row.type==='assistant'&&row.message?.content?.some(part=>part.type==='tool_use')));
  const after=await settings();
  check('native_model_switch_preserves_user_default_settings_bytes',before===null?after===null:after!==null&&before.equals(after));
  check('native_model_switch_labels_launch_model_without_claiming_current_model',await panel.evaluate(`document.querySelector('#terminalTitle').textContent.includes('modelo inicial: haiku') && document.querySelector('#managedTasks').textContent.includes('modelo inicial: haiku')`));
  await writeFile(join(root,'model-switch-receipt.json'),JSON.stringify({taskId:task.id,cwd:task.cwd,historyPath:receipt.history.path,response:receipt.response,settingsUnchanged:true,settingsHash:before&&createHash('sha256').update(before).digest('hex')},null,2));
  await panel.screenshot('model-switch-result');
}
