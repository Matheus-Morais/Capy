import test from 'node:test';
import assert from 'node:assert/strict';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {mkdtemp,copyFile,writeFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,dirname,basename,resolve} from 'node:path';

test('statusline wrapper preserves Unicode in the existing command',{skip:process.platform!=='win32'},async()=>{
  const root=await mkdtemp(join(tmpdir(),'capy-statusline-wrapper-'));
  try{
    const script=join(root,'statusline.ps1');
    await copyFile(resolve('src-tauri/scripts/claude-statusline.ps1'),script);
    const expected='ação_日本語_🦫';
    await writeFile(join(root,'bridge.json'),JSON.stringify({executable:process.execPath,configDir:root,bash:null,original:{type:'command',command:`[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false);[Console]::Write('${expected}')`}}));
    const pending=promisify(execFile)('powershell.exe',['-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-File',script],{windowsHide:true,timeout:20_000,maxBuffer:8192});
    pending.child.stdin.end('{}\n');
    const {stdout}=await pending;
    assert.equal(stdout,expected);
  }finally{
    assert.equal(dirname(root),tmpdir());assert.ok(basename(root).startsWith('capy-statusline-wrapper-'));
    await rm(root,{recursive:true,force:true});
  }
});
