import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {mkdir,writeFile} from 'node:fs/promises';
import {join,basename} from 'node:path';

const powershell=promisify(execFile);
export function externalTaskProof({panel,root,check,waitFor,taskHistory,sameFolder,appExited}){
  let task,profile,owned=[];
  async function processes(){
    if(!task)return [];
    const script=`$ErrorActionPreference='Stop'
$found=@(Get-CimInstance Win32_Process -Filter "Name='claude.exe'" | Where-Object { $_.CommandLine -and $_.CommandLine.Contains($env:CAPY_PROOF_UUID) })
if($found.Count -gt 1){throw 'Multiple processes for own UUID'}
$result=@()
foreach($candidate in $found){
  $cli=Get-Process -Id $candidate.ProcessId -ErrorAction Stop
  if(-not $cli.Path.EndsWith('\\claude.exe',[StringComparison]::OrdinalIgnoreCase)){throw 'Wrong CLI executable'}
  $parent=Get-CimInstance Win32_Process -Filter ('ProcessId='+$candidate.ParentProcessId)
  if($parent.Name -ne 'powershell.exe' -or -not $parent.CommandLine.Contains($env:CAPY_PROOF_ROOT) -or -not $parent.CommandLine.Contains('claude-terminal.ps1')){throw 'Wrong launcher owner'}
  $shell=Get-Process -Id $parent.ProcessId -ErrorAction Stop
  $result+=@{kind='cli';pid=$cli.Id;started=$cli.StartTime.ToUniversalTime().ToString('o');startedTicks=$cli.StartTime.ToUniversalTime().Ticks.ToString();path=$cli.Path}
  $result+=@{kind='launcher';pid=$shell.Id;started=$shell.StartTime.ToUniversalTime().ToString('o');startedTicks=$shell.StartTime.ToUniversalTime().Ticks.ToString();path=$shell.Path}
}
ConvertTo-Json -InputObject @($result) -Compress`;
    const {stdout}=await powershell('powershell.exe',['-NoProfile','-NonInteractive','-Command',script],{windowsHide:true,timeout:10_000,maxBuffer:8192,env:{...process.env,CAPY_PROOF_UUID:task.id,CAPY_PROOF_ROOT:basename(root)}});
    return JSON.parse(stdout.trim());
  }
  async function cleanup(){
    if(!owned.length&&task)owned=await processes();
    if(!owned.length)return;
    const script=`$ErrorActionPreference='Stop'
$expected=ConvertFrom-Json $env:CAPY_PROOF_OWNED
foreach($item in $expected){
  $process=Get-Process -Id $item.pid -ErrorAction SilentlyContinue
  if(-not $process){continue}
  if($process.Path -ne $item.path -or $process.StartTime.ToUniversalTime().Ticks.ToString() -ne $item.startedTicks){throw 'Process identity changed; preserved'}
  Stop-Process -Id $process.Id -ErrorAction Stop
  if(-not $process.WaitForExit(10000)){throw 'Own process did not exit'}
}`;
    await powershell('powershell.exe',['-NoProfile','-NonInteractive','-Command',script],{windowsHide:true,timeout:25_000,maxBuffer:8192,env:{...process.env,CAPY_PROOF_OWNED:JSON.stringify(owned)}});
  }
  async function verify(){
    const project=join(root,'own-external-project');await mkdir(project);
    const marker=`PROBE_${randomUUID().replaceAll('-','')}`;
    const prompt=`Responda literalmente a palavra ${marker}, sem qualquer outra palavra. Não use ferramentas nem altere arquivos.`;
    profile=(await panel.invoke('list_profiles')).find(value=>value.id==='claude-default');assert.ok(profile);
    const identity=await panel.invoke('profile_identity',{id:profile.id,cwd:project});
    check('native_external_task_pins_subscription',identity.loggedIn&&identity.billing==='subscription'&&!!identity.account);
    await panel.evaluate(`(()=>{document.querySelector('#newTask').open=true;const form=document.querySelector('#startTask');form.elements.namedItem('profile').value='claude-default';form.elements.namedItem('cwd').value=${JSON.stringify(project)};form.elements.namedItem('cwd').dispatchEvent(new Event('input',{bubbles:true}));form.elements.namedItem('model').value='haiku';form.elements.namedItem('prompt').value=${JSON.stringify(prompt)};return form.elements.namedItem('mode').value;})()`).then(mode=>check('native_external_task_is_default_form_mode',mode==='external'));
    await panel.evaluate(`document.querySelector('#verifyTaskAccount').click();true`);
    await waitFor(()=>panel.evaluate(`!document.querySelector('#startTask button[type="submit"]').disabled`),'Conta externa verificada',20_000);
    await panel.evaluate(`document.querySelector('#startTask').requestSubmit();true`);
    task=await waitFor(async()=> (await panel.invoke('list_tasks')).find(value=>sameFolder(value.cwd,project)),'Tarefa externa própria registrada',25_000);
    check('native_external_task_preserves_folder_account_model_instruction_and_mode',task.mode==='external'&&task.profileId===profile.id&&task.account===identity.account&&task.billing==='subscription'&&task.model==='haiku'&&task.prompt===prompt);
    owned=await waitFor(async()=>{const values=await processes();return values.length===2?values:null;},'CLI externo e launcher próprios',20_000);
    await writeFile(join(root,'external-processes.json'),JSON.stringify({task,owned},null,2));
    const receipt=await waitFor(async()=>{
      const history=await taskHistory(task,profile);
      const response=history?.rows.find(row=>row.type==='assistant'&&row.sessionId===task.id&&row.message?.model?.toLowerCase().includes('haiku')&&row.message?.content?.some(part=>part.type==='text'&&part.text.includes(marker)));
      return response?{history,response}:null;
    },'Resposta real do CLI externo próprio (se houver confiança de pasta, não aprovar automaticamente)',60_000);
    const user=receipt.history.rows.find(row=>row.type==='user'&&row.sessionId===task.id&&JSON.stringify(row.message?.content).includes(prompt));
    check('native_external_task_provider_confirms_exact_uuid_folder_and_prompt',!!user&&sameFolder(user.cwd,task.cwd));
    check('native_external_task_does_not_execute_tools',!receipt.history.rows.some(row=>row.type==='assistant'&&row.message?.content?.some(part=>part.type==='tool_use')));
    check('native_external_task_has_no_integrated_pty',await panel.evaluate(`window.__TAURI_INTERNALS__.invoke('terminal_replay',{id:${JSON.stringify(task.id)}}).then(()=>false,()=>true)`));
    await writeFile(join(root,'external-task-receipt.json'),JSON.stringify({task,historyPath:receipt.history.path,response:receipt.response,owned},null,2));
    await panel.screenshot('external-task-panel');
    check('native_external_task_not_in_owned_exit_resources',(await panel.invoke('prepare_exit_review')).resources.length===0);
    await panel.invoke('request_exit');
    const start=Date.now();while(!appExited()&&Date.now()-start<10_000)await new Promise(resolve=>setTimeout(resolve,100));
    check('native_external_task_application_exits_without_closing_external_origin',appExited());
    const remaining=await processes();
    check('native_external_task_exact_cli_and_launcher_survive_application_exit',remaining.length===owned.length&&owned.every(item=>remaining.some(value=>value.pid===item.pid&&value.started===item.started&&value.path===item.path)));
  }
  return {verify,cleanup};
}
