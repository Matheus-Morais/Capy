import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { mkdirSync, writeFileSync, readFileSync, existsSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { homedir } from 'node:os';
import { randomUUID } from 'node:crypto';

const root = resolve(import.meta.dirname, '..');
const project = join(root, 'scratch', `antigravity-proof-${randomUUID()}`);
const capy = join(root, 'src-tauri/target/release/capy.exe');
const agy = process.env.CAPY_TEST_AGY_EXE || join(homedir(), 'AppData/Local/agy/bin/agy.exe');
const gemini = process.env.CAPY_ANTIGRAVITY_HOME || join(homedir(), '.gemini');
mkdirSync(join(project, '.agents'), { recursive: true });
writeFileSync(join(project, 'AGENTS.md'), 'Isolated integration test. Do only the exact requested harmless command. Do not read project files, invoke skills, modify files, or delegate.\n');
const log = join(project, 'hooks.jsonl');
const wrapper = join(project, 'hook.cjs');
writeFileSync(wrapper, `
const fs=require('node:fs'),{spawnSync}=require('node:child_process'),path=require('node:path');
let input='';process.stdin.on('data',c=>input+=c);process.stdin.on('end',()=>{
  const h=JSON.parse(input),event=process.argv[2];
  const result=spawnSync(${JSON.stringify(capy)},['--antigravity-hook',event],{input,windowsHide:true,timeout:5000});
  let state=null;try{state=JSON.parse(fs.readFileSync(path.join(${JSON.stringify(gemini)},'capy-activity',h.conversationId+'.json'),'utf8')).state;}catch{}
  fs.appendFileSync(${JSON.stringify(log)},JSON.stringify({event,id:h.conversationId,workspace:h.workspacePaths?.[0],transcriptPath:h.transcriptPath,state,collectorExit:result.status})+'\\n');
  process.stdout.write('{}');
});`);
const handlers = Object.fromEntries(['PreInvocation', 'PostToolUse', 'Stop'].map(event => {
  assert.ok(!/\s/.test(wrapper), 'This Antigravity CLI build needs a hook path without spaces');
  const hook = { type: 'command', command: `node ${wrapper.replaceAll('\\', '/')} ${event}`, timeout: 8 };
  return [event, event === 'PostToolUse' ? [{ matcher: '*', hooks: [hook] }] : [hook]];
}));
writeFileSync(join(project, '.agents/hooks.json'), JSON.stringify({ 'capy-proof-observer': handlers }));
let child;
let id;
let exit;
const timer = setTimeout(() => child?.kill(), 120000);
async function snapshot() {
  const path = join(project, 'activity.json');
  execFileSync(capy, ['--activity-report', path], { windowsHide: true, timeout: 15000 });
  return JSON.parse(readFileSync(path, 'utf8')).sessions.find(s => s.id === `antigravity:${id}`);
}
try {
  child = spawn(agy, ['--new-project', '--print-timeout', '90s', '--output-format', 'stream-json', '--print', 'Isolated Capy test: run exactly one harmless command using run_command: powershell -NoProfile -Command "Start-Sleep -Seconds 15". Then reply OK. Do not inspect files, modify files, or delegate.'], {
    cwd: project, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'],
  });
  exit = new Promise((resolve, reject) => { child.once('error', reject); child.once('exit', code => resolve(code)); });
  child.stdout.resume();
  child.stderr.on('data', chunk => writeFileSync(join(project, 'stderr.log'), chunk, { flag: 'a' }));
  const deadline = Date.now() + 90000;
  let working = false;
  while (Date.now() < deadline && child.exitCode === null) {
    if (existsSync(log)) {
      const hooks = readFileSync(log, 'utf8').trim().split('\n').map(line => JSON.parse(line));
      const event = hooks.find(h => h.state === 'working' && resolve(h.workspace) === project && h.collectorExit === 0);
      if (event) {
        id = event.id;
        const session = await snapshot();
        if (session?.state === 'working') {
          assert.equal(resolve(session.origin), project);
          assert.equal(session.project, project.split(/[\\/]/).at(-1));
          assert.equal(session.request, null);
          assert.equal(session.command, null);
          working = true;
          console.log('PASS: cli_working (native CLI hook, held presence, correct project)');
          break;
        }
      }
    }
    await new Promise(resolve => setTimeout(resolve, 200));
  }
  assert.ok(working, 'No real CLI working evidence; inspect the private scratch log');
  assert.equal(await exit, 0, 'Own CLI session failed');
  assert.equal(await snapshot(), undefined, 'Closed CLI remained in Capy');
  console.log('PASS: cli_removed');
  writeFileSync(join(project, 'proof.json'), JSON.stringify({ passed: true, checks: ['cli_working', 'cli_removed'] }));
} finally {
  clearTimeout(timer);
  if (child?.exitCode === null) { child.kill(); await exit; }
}
