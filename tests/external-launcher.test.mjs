import test from 'node:test';
import assert from 'node:assert/strict';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {mkdtemp,mkdir,writeFile,readFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,resolve,dirname,basename} from 'node:path';

test('external launcher preserves UTF-8 literal arguments folder and isolated authentication environment',{skip:process.platform!=='win32'},async()=>{
  const root=await mkdtemp(join(tmpdir(),'capy-external-launch-'));
  try{
    const cwd=join(root,'project space');await mkdir(cwd);
    const config=join(root,'config space');await mkdir(config);
    const output=join(root,'result.json');const fixture=join(root,'arguments.mjs');
    const removeEnv=['ANTHROPIC_API_KEY','ANTHROPIC_AUTH_TOKEN','CLAUDE_CODE_OAUTH_TOKEN','ANTHROPIC_BASE_URL','CLAUDE_CODE_USE_BEDROCK','CLAUDE_CODE_USE_VERTEX','CLAUDE_CODE_USE_FOUNDRY'];
    await writeFile(fixture,`import {writeFileSync} from 'node:fs';writeFileSync(process.env.CAPY_LAUNCH_RESULT,JSON.stringify({args:process.argv.slice(2),cwd:process.cwd(),config:process.env.CLAUDE_CONFIG_DIR,removed:${JSON.stringify(removeEnv)}.every(key=>process.env[key]===undefined)}));`);
    const args=['Não alterar ação 日本語 🦫','literal "quotes" and \\slashes\\','$(literal) `literal` & pipe | ; %PATH%', '', 'C:\\path with space\\'];
    const launch=join(root,'launch.json');await writeFile(launch,JSON.stringify({executable:process.execPath,configDir:config,cwd,arguments:[fixture,...args],removeEnv}));
    await promisify(execFile)('powershell.exe',['-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-File',resolve('src-tauri/scripts/claude-terminal.ps1'),'-LaunchFile',launch],{windowsHide:true,timeout:20_000,maxBuffer:8192,env:{...process.env,...Object.fromEntries(removeEnv.map(key=>[key,'OWN_TEST_SENTINEL'])),CAPY_LAUNCH_RESULT:output}});
    const result=JSON.parse(await readFile(output,'utf8'));
    assert.deepEqual(result.args,args);assert.equal(result.cwd,cwd);assert.equal(result.config,config);assert.equal(result.removed,true);
  }finally{
    assert.equal(dirname(root),tmpdir());assert.ok(basename(root).startsWith('capy-external-launch-'));
    await rm(root,{recursive:true,force:true});
  }
});
