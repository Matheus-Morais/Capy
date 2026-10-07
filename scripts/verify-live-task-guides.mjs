import assert from 'node:assert/strict';
import {mkdir,readFile,writeFile} from 'node:fs/promises';
import {join} from 'node:path';

export function instructionProof({panel,root,project,profile,check,waitFor,sameFolder,taskHistory}){
  const files=[
    {path:join(project,'CLAUDE.md'),text:'@plan.md\n# Own guide\nDo not use tools or modify files for this receipt.\n'},
    {path:join(project,'plan.md'),text:'# Own contract\nPreserve the public fields CAPY_GUIDE_RULE_47.\n'},
    {path:join(project,'.claude','rules','own-rule.md'),text:'# Own rule\nKeep the response literal and preserve CAPY_RULE_83.\n'},
  ];
  let originalSettings;
  const settings=async()=>readFile(join(profile.configDir,'settings.json')).catch(error=>{if(error.code==='ENOENT')return null;throw error;});
  async function prepare(){
    originalSettings=await settings();await mkdir(join(project,'.claude','rules'),{recursive:true});
    for(const file of files)await writeFile(file.path,file.text);
  }
  async function verify(task){
    await panel.invoke('demo_action',{action:'scenario',id:'',answer:'real'});
    const snapshotPath=join(profile.configDir,'capy-guides',`${task.id}.json`);
    const snapshot=await waitFor(async()=>{
      const value=await readFile(snapshotPath,'utf8').then(JSON.parse).catch(async error=>{
        const status=await readFile(snapshotPath.replace(/\.json$/,'.status.json'),'utf8').then(JSON.parse).catch(()=>null);
        if(status)throw new Error(`InstructionsLoaded: ${status.stage}`);throw error;
      });
      return files.every(file=>value.guides.some(g=>sameFolder(g.instruction.filePath,file.path)))?value:null;
    },'InstructionsLoaded real dos três arquivos próprios',20_000);
    check('native_guides_capture_exact_session_folder_and_three_loaded_files',snapshot.sessionId===task.id&&sameFolder(snapshot.cwd,task.cwd)&&files.every(file=>snapshot.guides.some(g=>sameFolder(g.instruction.filePath,file.path)&&g.text===file.text)));
    const imported=snapshot.guides.find(g=>sameFolder(g.instruction.filePath,files[1].path));
    check('native_guides_capture_import_scope_reason_and_parent',imported.instruction.memoryType==='Project'&&imported.instruction.loadReason==='include'&&sameFolder(imported.instruction.parentFilePath,files[0].path));
    await writeFile(join(root,'instructions-loaded-receipt.json'),JSON.stringify({task,snapshotPath,snapshot},null,2));
    await writeFile(files[1].path,'# Changed later\nCAPY_NOT_LOADED_LATER_92\n');
    const before=await taskHistory(task,profile);
    const review=await waitFor(()=>panel.invoke('prepare_handoff',{sourceId:task.id,destinationId:profile.id,model:'haiku'}),'Revisão após fim real do turno Claude',100_000);
    check('native_guides_review_preserves_observed_rules_after_disk_change',review.sourceTaskId===task.id&&review.summary.guides.includes('CAPY_GUIDE_RULE_47')&&review.summary.guides.includes('CAPY_RULE_83')&&review.summary.guides.includes('CLAUDE.md')&&review.summary.guides.includes('plan.md')&&review.summary.guides.includes('include')&&!review.summary.guides.includes('CAPY_NOT_LOADED_LATER_92'));
    await writeFile(join(root,'instructions-handoff-review.json'),JSON.stringify(review,null,2));
    await waitFor(()=>panel.evaluate(`!!document.querySelector('form[data-handoff="${review.nonce}"] textarea[name="guides"]')`),'Resumo no painel');
    check('native_guides_editable_review_contains_observed_rules',await panel.evaluate(`document.querySelector('form[data-handoff="${review.nonce}"] textarea[name="guides"]').value.includes('CAPY_GUIDE_RULE_47')`));
    await panel.evaluate(`document.querySelector('form[data-handoff="${review.nonce}"] textarea[name="guides"]').scrollIntoView({block:'center'});true`);
    await panel.screenshot('instructions-review');
    await panel.invoke('cancel_handoff',{nonce:review.nonce});
    const after=await taskHistory(task,profile);
    check('native_guides_cancel_review_sends_no_new_turn_or_destination',(await panel.invoke('list_tasks')).length===1&&(await panel.invoke('list_handoffs')).length===0&&before.rows.filter(row=>row.type==='user').length===after.rows.filter(row=>row.type==='user').length);
    const current=await settings();
    check('native_guides_preserve_profile_settings_bytes',originalSettings===null?current===null:current?.equals(originalSettings));
    assert.ok(snapshot.guides.length>=3);
  }
  return {prepare,verify};
}
