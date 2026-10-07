import {randomUUID} from 'node:crypto';
import {writeFile} from 'node:fs/promises';
import {join} from 'node:path';

export async function verifyLiveTaskControls({panel,task,profile,process:originalProcess,root,check,waitFor,taskHistory,processInfo}){
  await panel.call('Page.enable');
  const screen=()=>panel.evaluate(`Array.from(document.querySelector('.xterm-rows').children).map(row=>row.textContent).join(String.fromCharCode(10))`);
  const outputTokens=text=>{
    const match=text.match(/…\s*\([^\n)]*↓\s*([\d.,]+)(k?)\s+tokens\)/i);
    return match?Number(match[1].replace(',','.'))*(match[2]?1000:1):null;
  };
  const active=text=>outputTokens(text)>0;
  const sameProcess=async()=>{
    const current=await processInfo(originalProcess.pid);
    return current?.started===originalProcess.started&&current?.path===originalProcess.path;
  };
  async function generating(label){
    const marker=`CAPY_${randomUUID().replaceAll('-','').slice(0,12)}`;
    const prompt=`Sem ferramentas. Gere uma lista de mil linhas numeradas, sem abreviar nem usar código. Cada linha começa com o marcador ${marker}, seguido de espaço e do número crescente. Comece agora e continue até o fim.`;
    await panel.invoke('terminal_input',{id:task.id,data:prompt});
    await waitFor(async()=> (await screen()).includes(marker),'Instrução própria recebida pelo campo do CLI');
    await panel.invoke('terminal_input',{id:task.id,data:'\r'});
    let first;
    const receipt=await waitFor(async()=>{
      const text=await screen();await writeFile(join(root,`${label}-screen.txt`),text);
      const tokens=outputTokens(text);if(!(tokens>0))return null;
      const now=Date.now();
      if(!first){first={at:now,tokens,text};return null;}
      if(now-first.at<1000||tokens<=first.tokens)return null;
      const history=await taskHistory(task,profile);
      const user=history?.rows.find(row=>row.type==='user'&&row.sessionId===task.id&&JSON.stringify(row.message?.content).includes(prompt));
      return user&&await sameProcess()?{marker,prompt,first,last:{at:now,tokens,text},historyPath:history.path,user}:null;
    },`Tokens de saída crescentes durante geração (${label})`,30_000);
    check(`native_${label}_official_output_token_count_increases_during_own_turn`,receipt.last.tokens>receipt.first.tokens&&receipt.last.at>receipt.first.at);
    await writeFile(join(root,`${label}-receipt.json`),JSON.stringify(receipt,null,2));
    await panel.screenshot(label);
    return receipt;
  }
  const interruptionReceipt=await generating('interrupt-active');
  check('native_interrupt_backend_rejects_missing_confirmation',await panel.evaluate(`window.__TAURI_INTERNALS__.invoke('terminal_interrupt',{id:${JSON.stringify(task.id)},confirmed:false}).then(()=>false,()=>true)`));
  await panel.evaluate(`setTimeout(()=>document.querySelector('#interruptTerminal').click(),0);true`);
  const cancel=await waitFor(()=>panel.takeDialog(),'Diálogo real de interrupção');
  check('native_interrupt_dialog_names_exact_session_and_folder',cancel.type==='confirm'&&cancel.message.includes(task.id)&&cancel.message.includes(task.cwd));
  await panel.call('Page.handleJavaScriptDialog',{accept:false});
  check('native_interrupt_cancel_preserves_owned_process',await sameProcess());
  const afterCancel=await waitFor(async()=>{
    const text=await screen();return outputTokens(text)>interruptionReceipt.last.tokens?text:null;
  },'Tokens continuam aumentando após cancelar interrupção');
  check('native_interrupt_cancel_preserves_generation_progress',outputTokens(afterCancel)>interruptionReceipt.last.tokens);
  await panel.evaluate(`setTimeout(()=>document.querySelector('#interruptTerminal').click(),0);true`);
  const approve=await waitFor(()=>panel.takeDialog(),'Nova confirmação de interrupção');
  check('native_interrupt_fresh_confirmation_names_same_session',approve.type==='confirm'&&approve.message.includes(task.id));
  await panel.call('Page.handleJavaScriptDialog',{accept:true});
  const interrupted=await waitFor(async()=>{
    const text=await screen();return /Interrupted|interrompid/i.test(text)&&!active(text)?text:null;
  },'CLI confirma interrupção do turno',15_000);
  check('native_interrupt_official_cli_reports_interrupted_and_keeps_process',await sameProcess());
  const partial=[...interrupted.matchAll(new RegExp(interruptionReceipt.marker+'\\s+(\\d+)','g'))].map(match=>Number(match[1]));
  check('native_interrupt_keeps_partial_response_from_exact_own_prompt',partial.length>=2&&partial.some((value,i)=>i>0&&value>partial[i-1]));
  await writeFile(join(root,'interrupt-result.txt'),interrupted);await panel.screenshot('interrupt-result');
  const exitReceipt=await generating('exit-active');
  await panel.evaluate(`window.__TAURI_INTERNALS__.invoke('request_exit').catch(error=>{window.__capyExitError=String(error);});true`);
  const exitCancel=await waitFor(()=>panel.takeDialog(),'Cancelamento de saída durante geração');
  check('native_exit_during_generation_cancel_dialog_names_exact_session',exitCancel.type==='confirm'&&exitCancel.message.includes(task.id));
  await panel.call('Page.handleJavaScriptDialog',{accept:false});
  check('native_exit_during_generation_cancel_preserves_owned_process',await sameProcess());
  const beforeApproval=await waitFor(async()=>{
    const text=await screen();return outputTokens(text)>exitReceipt.last.tokens?text:null;
  },'Tokens continuam aumentando após cancelar saída');
  check('native_exit_cancel_preserves_generation_progress',outputTokens(beforeApproval)>exitReceipt.last.tokens);
  const history=await taskHistory(task,profile);
  check('native_control_proof_does_not_execute_tools',!history.rows.some(row=>row.type==='assistant'&&row.message?.content?.some(part=>part.type==='tool_use')));
  await writeFile(join(root,'exit-before-approval.txt'),beforeApproval);
}
