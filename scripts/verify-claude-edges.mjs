import { spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import assert from 'node:assert/strict';
import { randomUUID, createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const which = process.argv[process.argv.indexOf('--case') + 1];
assert.ok(['allow', 'disconnect', 'absent', 'timeout'].includes(which), 'Use --case allow|disconnect|absent|timeout');
const exe = process.env.CAPY_TEST_CAPY_EXE || path.join(repo, 'src-tauri/target/debug/capy.exe');
const claude = process.env.CAPY_TEST_CLAUDE_EXE || path.join(os.homedir(), '.local/bin/claude.exe');
function regular(file, directory = false) {
  for (let current = path.resolve(file); ; current = path.dirname(current)) {
    assert.equal(fs.lstatSync(current).isSymbolicLink(), false, `Unsafe path: ${current}`);
    if (current === path.dirname(current)) break;
  }
  assert.ok(directory ? fs.statSync(file).isDirectory() : fs.statSync(file).isFile());
}
regular(exe); regular(claude);
fs.mkdirSync(path.join(repo, 'scratch'), { recursive: true }); regular(path.join(repo, 'scratch'), true);
const dir = fs.mkdtempSync(path.join(repo, 'scratch/claude-capy-'));
const session = randomUUID();
const endpointRoot = which === 'absent' ? path.join(dir, 'no-app') : dir;
const marker = path.join(process.env.CLAUDE_CONFIG_DIR || path.join(os.homedir(), '.claude'), 'capy-activity/responses-enabled');
const markerText = `capy-edge-proof:${randomUUID()}\n`;
let markerStat;
const report = { case: which, cwd: dir, session, startedAt: new Date().toISOString(), passed: false,
  executableSha256: createHash('sha256').update(fs.readFileSync(exe)).digest('hex'), checks: [], hostDecisions: [], hookDecisions: 0 };
const children = [];
let timer;
let fail;
const failure = new Promise((_, reject) => { fail = reject; }); failure.catch(() => {});
const race = promise => Promise.race([promise, failure]);
function queue() { const values = []; let waiter; return { push(v) { if (waiter) { const w = waiter; waiter = null; w(v); } else values.push(v); }, take() { return values.shift(); }, next() { return race(values.length ? Promise.resolve(values.shift()) : new Promise(r => { waiter = r; })); } }; }
const events = queue();
const log = (source, value) => fs.appendFileSync(path.join(dir, 'events.jsonl'), JSON.stringify({ at: Date.now(), source, value }) + '\n');
const env = { ...process.env, CAPY_CLAUDE_INTERVENTION_ROOT: endpointRoot };
function start(command, args, name, cwd = dir) {
  const child = spawn(command, args, { cwd, env, windowsHide: true, stdio: ['pipe', 'pipe', 'pipe'] }); children.push(child);
  child.on('error', fail); child.stdin.on('error', fail);
  child.stderr.on('data', data => fs.appendFileSync(path.join(dir, `${name}.stderr.log`), data));
  let buffer = '';
  child.stdout.on('data', data => {
    buffer += data;
    if (buffer.length > 4 * 1024 * 1024) return fail(Error('Output exceeds bound'));
    while (buffer.includes('\n')) {
      const i = buffer.indexOf('\n'); const line = buffer.slice(0, i); buffer = buffer.slice(i + 1);
      if (!line.trim()) continue;
      try { const value = JSON.parse(line); log(name, value); events.push({ source: name, value }); } catch (e) { fail(e); }
    }
  });
  child.done = new Promise(resolve => child.once('exit', (code, signal) => { log(`${name}-exit`, { code, signal }); resolve({ code, signal }); }));
  return child;
}
function write(child, value) { child.stdin.write(JSON.stringify(value) + '\n'); }
function ownKill(child) {
  if (child.pid && child.exitCode === null && child.signalCode === null) spawnSync('taskkill', ['/PID', String(child.pid), '/T', '/F'], { windowsHide: true, stdio: 'ignore', timeout: 10000 });
}
// The wrapper records actual native hook stdout and exit, forwarding it unchanged.
// Claude remains the native hook process ancestor; the permission host stays passive
// until a no-decision hook has closed. A host denial never counts as hook approval.
const hookLog = path.join(dir, 'hook.jsonl');
const wrapper = path.join(dir, 'hook-wrapper.cjs');
fs.writeFileSync(wrapper, `const {spawn}=require('node:child_process');const fs=require('node:fs');const t=Date.now();const p=spawn(${JSON.stringify(exe)},['--claude-intervention'],{windowsHide:true,stdio:['pipe','pipe','pipe']});let out='',err='';process.stdin.pipe(p.stdin);p.stdout.on('data',d=>{out+=d;process.stdout.write(d)});p.stderr.on('data',d=>{err+=d;process.stderr.write(d)});p.on('error',e=>{fs.appendFileSync(${JSON.stringify(hookLog)},JSON.stringify({error:String(e)})+'\\n');process.exitCode=1});p.on('close',code=>{fs.appendFileSync(${JSON.stringify(hookLog)},JSON.stringify({code,stdout:out,stderr:err,elapsedMs:Date.now()-t,endedAt:Date.now()})+'\\n');process.exitCode=code||0});`);
const settings = path.join(dir, 'settings.json');
fs.writeFileSync(settings, JSON.stringify({ hooks: { PermissionRequest: [{ hooks: [{ type: 'command', command: `"${process.execPath}" "${wrapper}"`, timeout: 120 }] }] } }));
function hooks() { return fs.existsSync(hookLog) ? fs.readFileSync(hookLog, 'utf8').trim().split('\n').filter(Boolean).map(JSON.parse) : []; }
let probe;
try {
  if (fs.existsSync(marker)) regular(marker);
  else {
    assert.ok(process.argv.includes('--temporary-optin'), 'Enable responses or pass --temporary-optin');
    regular(path.dirname(path.dirname(marker)), true);
    fs.mkdirSync(path.dirname(marker), { recursive: true }); regular(path.dirname(marker), true);
    fs.writeFileSync(marker, markerText, { flag: 'wx' }); markerStat = fs.statSync(marker);
  }
  const version = spawnSync(claude, ['--version'], { windowsHide: true, encoding: 'utf8', timeout: 10000 });
  assert.equal(version.status, 0, 'Claude version query failed'); report.claudeVersion = version.stdout.trim();
  timer = setTimeout(() => fail(Error('Edge proof exceeded 180 seconds')), 180000);
  if (which !== 'absent') {
    probe = start(exe, ['--claude-intervention-probe', session, dir], 'probe', repo);
    assert.equal((await events.next()).value.event, 'connected');
  } else assert.equal(fs.existsSync(path.join(endpointRoot, 'claude-interventions.json')), false);
  const child = start(claude, ['-p', '--session-id', session, '--settings', settings, '--setting-sources', '', '--strict-mcp-config', '--tools', 'Write', '--permission-mode', 'manual', '--input-format', 'stream-json', '--output-format', 'stream-json', '--verbose', '--permission-prompt-tool', 'stdio', '--max-budget-usd', '1', '--model', 'haiku', '--system-prompt', 'Isolated test: use exactly the requested tool once. No retries, subagents or other tools.'], 'claude');
  write(child, { type: 'user', message: { role: 'user', content: 'Use Write exactly once to create probe.txt in the current directory with exact content capy-proof (no newline). Whether allowed or denied, do not retry. Then say EDGE_COMPLETE.' } });
  let permission, host, result, resolved = false, disconnected = false, continuation = false;
  const toolResults = []; const toolMetadata = []; const toolUses = [];
  while (!result) {
    // Polling permits fallback only after the wrapper proves hook process closure.
    let event = events.take();
    if (!event) { await race(new Promise(r => setTimeout(r, 100))); event = events.take(); }
    if (event?.source === 'probe') {
      const value = event.value;
      if (value.event === 'request') {
        assert.equal(permission, undefined, 'Unexpected extra tool request'); permission = value.request;
        assert.equal(permission.sessionId, `claude:${session}`); assert.equal(permission.body.toolName, 'Write');
        assert.equal(path.resolve(permission.body.toolInput.file_path), path.join(dir, 'probe.txt'));
        assert.equal(permission.body.toolInput.content, 'capy-proof');
        if (which === 'allow') {
          write(probe, { context: Object.fromEntries(['nonce','generation','sessionId','threadId','turnId','itemId'].map(k => [k, permission[k]])), response: { decision: 'allow' } });
        } else if (which === 'disconnect') {
          probe.stdin.end(); assert.equal((await race(probe.done)).code, 0); disconnected = true;
        }
      } else if (value.event === 'resolved') { assert.equal(which, 'allow'); resolved = true; }
      else assert.fail(`Unexpected probe event ${value.event}`);
    }
    if (event?.source === 'claude') {
      const value = event.value;
      if (value.type === 'control_request') { assert.equal(value.request?.subtype, 'can_use_tool'); assert.equal(host, undefined); host = value; }
      if (value.type === 'assistant') for (const part of value.message?.content || []) if (part.type === 'tool_use') toolUses.push(part);
      if (value.type === 'user') toolMetadata.push(...(value.tool_result_meta || []));
      if (value.type === 'user') for (const part of value.message?.content || []) if (part.type === 'tool_result') toolResults.push(part);
      if (value.type === 'assistant' && toolResults.length && value.message?.content?.some(p => p.type === 'text' && /EDGE_COMPLETE/.test(p.text))) continuation = true;
      if (value.type === 'result') { result = value; child.stdin.end(); }
    }
    const completed = hooks();
    if (which !== 'allow' && host && completed.length && report.hostDecisions.length === 0) {
      assert.equal(completed.length, 1); assert.equal(completed[0].code, 0); assert.equal(completed[0].stdout, ''); assert.equal(completed[0].stderr, '');
      if (which === 'disconnect') assert.ok(disconnected);
      if (which === 'timeout') {
        assert.ok(permission && !disconnected, 'Timeout requires a real request with service kept alive');
        assert.equal(probe.exitCode, null, 'Service exited before native timeout');
        assert.ok(completed[0].elapsedMs >= 105000 && completed[0].elapsedMs <= 125000, 'Expected unchanged production 110-second timeout');
        report.timeout = { productionSeconds: 110, outerHookSeconds: 120, actualHookElapsedMs: completed[0].elapsedMs };
      }
      const decision = { source: 'fallback-host-after-hook-exit', at: Date.now(), requestId: host.request_id, behavior: 'deny' }; report.hostDecisions.push(decision); log('fallback-host', decision);
      write(child, { type: 'control_response', response: { subtype: 'success', request_id: host.request_id, response: { behavior: 'deny', message: 'Fallback host denial after native hook returned no decision; do not retry.' } } });
    }
  }
  assert.equal((await race(child.done)).code, 0); assert.equal(result.subtype, 'success');
  assert.ok(continuation, 'No assistant continuation after tool result'); assert.equal(toolResults.length, 1); assert.equal(toolUses.length, 1); assert.equal(toolUses[0].name, 'Write');
  assert.equal(toolResults[0].tool_use_id, toolUses[0].id);
  const metadata = toolMetadata.filter(meta => meta.id === toolResults[0].tool_use_id);
  assert.equal(metadata.length, 1, 'Missing or duplicate matching Claude tool result metadata');
  report.toolResultMetadata = metadata[0];
  const completed = hooks(); assert.equal(completed.length, 1); report.nativeHook = completed[0]; assert.equal(completed[0].code, 0, 'Native hook failed');
  if (which === 'allow') {
    assert.ok(permission && resolved); assert.equal(metadata[0].permission_decision?.source, 'hook'); assert.equal(metadata[0].permission_decision?.decision, 'accept'); assert.equal(report.hostDecisions.length, 0);
    assert.equal(JSON.parse(completed[0].stdout).hookSpecificOutput.decision.behavior, 'allow'); report.hookDecisions = 1;
    assert.ok(!toolResults[0].is_error); assert.equal(fs.readFileSync(path.join(dir, 'probe.txt'), 'utf8'), 'capy-proof');
    report.checks.push('exact real hook allow', 'successful Claude tool result', 'exact file content', 'assistant continuation');
  } else {
    assert.equal(metadata[0].permission_decision?.source, 'user_reject'); assert.equal(metadata[0].permission_decision?.decision, 'reject');
    assert.equal(completed[0].stdout, ''); assert.equal(report.hostDecisions.length, 1); assert.ok(toolResults[0].is_error);
    assert.equal(fs.existsSync(path.join(dir, 'probe.txt')), false);
    if (which === 'absent') assert.ok(completed[0].elapsedMs < 5000, 'Absent app did not return quickly');
    report.checks.push('native hook exited silently without decision', 'fallback host alone denied', 'file absent', 'assistant continuation');
    if (which === 'timeout') {
      assert.ok(permission && !disconnected); assert.equal(report.hookDecisions, 0);
      report.checks.push('real pending permission expired at production 110-second timeout', 'service remained alive through native hook closure');
      probe.stdin.end(); assert.equal((await race(probe.done)).code, 0);
    }
    if (which === 'disconnect') {
      assert.ok(permission && disconnected);
      const replacement = start(exe, ['--claude-intervention-probe', session, dir], 'replacement', repo);
      const connected = await events.next(); assert.equal(connected.source, 'replacement'); assert.equal(connected.value.event, 'connected');
      write(replacement, { context: Object.fromEntries(['nonce','generation','sessionId','threadId','turnId','itemId'].map(k => [k, permission[k]])), response: { decision: 'allow' } });
      const stale = await events.next(); assert.equal(stale.source, 'replacement'); assert.equal(stale.value.event, 'rejected');
      replacement.stdin.end(); assert.equal((await race(replacement.done)).code, 0);
      report.checks.push('real pending permission disconnected', 'disconnected context rejected by replacement service');
    }
  }
  report.passed = true;
} catch (e) { report.error = e.stack; process.exitCode = 1; }
finally {
  clearTimeout(timer); for (const child of children) ownKill(child);
  if (markerStat) {
    try { regular(marker); const current = fs.statSync(marker);
      if (current.ino === markerStat.ino && current.dev === markerStat.dev && current.birthtimeMs === markerStat.birthtimeMs && current.mtimeMs === markerStat.mtimeMs && fs.readFileSync(marker, 'utf8') === markerText) fs.unlinkSync(marker);
      else report.markerCleanup = 'Changed externally; preserved';
    } catch (e) { report.markerCleanup = `Preserved: ${e.message}`; }
  }
  report.finishedAt = new Date().toISOString(); fs.writeFileSync(path.join(dir, 'report.json'), JSON.stringify(report, null, 2));
  console.log(`${report.passed ? 'PASS' : 'FAIL'} ${which}: ${path.join(dir, 'report.json')}`);
}
