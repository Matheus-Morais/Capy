import { spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import assert from 'node:assert/strict';
import { createHash, randomUUID } from 'node:crypto';
import { fileURLToPath } from 'node:url';

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const exe = process.env.CAPY_TEST_CAPY_EXE || path.join(repo, 'src-tauri/target/release/capy.exe');
const claude = process.env.CAPY_TEST_CLAUDE_EXE || path.join(os.homedir(), '.local/bin/claude.exe');
const config = process.env.CLAUDE_CONFIG_DIR || path.join(os.homedir(), '.claude');
const passiveHost = process.argv.includes('--passive-permission-host');
function regular(file, directory = false) {
  const absolute = path.resolve(file);
  for (let current = absolute; ; current = path.dirname(current)) {
    const stat = fs.lstatSync(current);
    assert.equal(stat.isSymbolicLink(), false, `Reparse/symlink path refused: ${current}`);
    if (current === path.dirname(current)) break;
  }
  const stat = fs.lstatSync(absolute);
  assert.ok(directory ? stat.isDirectory() : stat.isFile(), `Unexpected file type: ${absolute}`);
}
const marker = path.join(config, 'capy-activity', 'responses-enabled');
const markerContent = `capy-intervention-proof:${randomUUID()}\n`;
let ownedMarker;
function optIn() {
  if (fs.existsSync(marker)) { regular(marker); return; }
  assert.ok(process.argv.includes('--temporary-optin'), 'Enable responses in Capy or pass --temporary-optin');
  regular(config, true);
  const parent = path.dirname(marker);
  if (!fs.existsSync(parent)) fs.mkdirSync(parent);
  regular(parent, true);
  try {
    fs.writeFileSync(marker, markerContent, { flag: 'wx' });
    ownedMarker = fs.lstatSync(marker);
  } catch (error) {
    if (error.code !== 'EEXIST') throw error;
    regular(marker);
  }
}
regular(exe);
regular(claude);
const scratch = path.join(repo, 'scratch');
fs.mkdirSync(scratch, { recursive: true });
regular(scratch, true);
const dir = fs.mkdtempSync(path.join(scratch, 'claude-capy-'));
const proofRoot = dir;
const sessionId = randomUUID();
const settings = path.join(dir, 'settings.json');
const hook = { type: 'command', command: `"${exe}" --claude-intervention`, timeout: 120 };
fs.writeFileSync(settings, JSON.stringify({ hooks: {
  PermissionRequest: [{ hooks: [hook] }],
  PreToolUse: [{ matcher: '^AskUserQuestion$', hooks: [hook] }],
} }));
const version = spawnSync(claude, ['--version'], { windowsHide: true, encoding: 'utf8', timeout: 10000 });
assert.equal(version.status, 0, 'Claude version query failed');
const report = { mode: 'print', permissionTransport: passiveHost ? 'hooks-with-passive-host' : 'synchronous hooks only',
  passiveHostRequests: 0, sessionId, cwd: dir, model: 'haiku', maxBudgetUsd: 1,
  claudeVersion: version.stdout.trim(), executableSha256: createHash('sha256').update(fs.readFileSync(exe)).digest('hex'),
  startedAt: new Date().toISOString(), checks: [], passed: false };
const env = { ...process.env, CAPY_CLAUDE_INTERVENTION_ROOT: proofRoot };
const children = [];
let deadline;
let fail;
const failed = new Promise((_, reject) => { fail = reject; });
// Every asynchronous operation races the same deadline or subprocess failure.
failed.catch(() => {});
const race = operation => Promise.race([operation, failed]);
const log = (source, value) => fs.appendFileSync(path.join(dir, 'events.jsonl'), JSON.stringify({ source, value }) + '\n');
function start(command, args, name, cwd = dir) {
  const child = spawn(command, args, { cwd, env, windowsHide: true, stdio: ['pipe', 'pipe', 'pipe'] });
  children.push(child);
  child.on('error', fail);
  child.stdin.on('error', fail);
  child.stderr.on('data', chunk => fs.appendFileSync(path.join(dir, `${name}.stderr.log`), chunk));
  return child;
}
function lines(child, source, callback) {
  let buffer = '';
  child.stdout.on('data', chunk => {
    buffer += chunk;
    if (buffer.length > 4 * 1024 * 1024) { fail(new Error(`${source} output exceeded limit`)); return; }
    while (buffer.includes('\n')) {
      const at = buffer.indexOf('\n');
      const line = buffer.slice(0, at); buffer = buffer.slice(at + 1);
      if (!line.trim()) continue;
      try { const value = JSON.parse(line); log(source, value); callback(value); }
      catch (error) { fail(error); }
    }
  });
}
function events() {
  const queue = [];
  let waiter;
  return {
    push(value) { if (waiter) { const resolve = waiter; waiter = null; resolve(value); } else queue.push(value); },
    next() { return race(queue.length ? Promise.resolve(queue.shift()) : new Promise(resolve => { waiter = resolve; })); },
  };
}
function write(child, value) { child.stdin.write(JSON.stringify(value) + '\n'); }
function context(view) {
  return Object.fromEntries(['nonce', 'generation', 'sessionId', 'threadId', 'turnId', 'itemId'].map(key => [key, view[key]]));
}
function exited(child) {
  return new Promise(resolve => child.once('exit', (code, signal) => resolve({ code, signal })));
}
function killOwnTree(child) {
  if (child.pid && child.exitCode === null && child.signalCode === null) {
    spawnSync('taskkill', ['/PID', String(child.pid), '/T', '/F'], { windowsHide: true, stdio: 'ignore', timeout: 10000 });
  }
}
try {
  optIn();
  deadline = setTimeout(() => fail(new Error('Claude intervention proof exceeded 180 seconds')), 180000);
  const probe = start(exe, ['--claude-intervention-probe', sessionId, proofRoot], 'probe', repo);
  const probeExit = exited(probe);
  const channel = events();
  lines(probe, 'probe', value => channel.push(value));
  assert.equal((await channel.next()).event, 'connected', 'Probe did not connect');
  probeExit.then(() => fail(new Error('Probe exited before proof completed')));
  // Hooks race a configured permission host; first decision wins. The optional
  // passive host enables interactive tools but never sends a control_response.
  // https://code.claude.com/docs/en/hooks#permissionrequest
  const child = start(claude, ['-p', '--session-id', sessionId, '--settings', settings,
    '--setting-sources', '', '--strict-mcp-config', '--tools', 'Write,AskUserQuestion',
    '--permission-mode', 'manual', '--input-format', 'stream-json', '--output-format', 'stream-json',
    '--verbose', ...(passiveHost ? ['--permission-prompt-tool', 'stdio'] : []),
    '--max-budget-usd', '1', '--model', 'haiku',
    '--system-prompt', 'You are an isolated integration test. Follow the exact tool sequence. No subagents, network, retries, or other tools.'], 'claude');
  const childExit = exited(child);
  let result;
  let assistantAfterAnswer = false;
  let answered = false;
  lines(child, 'claude', value => {
    if (value.type === 'control_request') {
      if (passiveHost && value.request?.subtype === 'can_use_tool') {
        report.passiveHostRequests++;
        return;
      }
      if (value.request?.subtype === 'can_use_tool') {
        write(child, { type: 'control_response', response: { subtype: 'success', request_id: value.request_id,
          response: { behavior: 'deny', message: 'Hook proof failed: fallback permission host must not execute tools.' } } });
      }
      fail(new Error(`Unexpected SDK control_request fallback: ${value.request?.subtype}`));
    }
    if (value.type === 'assistant' && answered && value.message?.content?.some(part => part.type === 'text' && part.text.trim())) assistantAfterAnswer = true;
    if (value.type === 'result') {
      result = value;
      report.result = { subtype: value.subtype, summary: value.result,
        totalCostUsd: value.total_cost_usd };
      child.stdin.end();
      if (!answered) fail(new Error('Claude completed before the hook question was answered; question criterion remains unproven'));
    }
  });
  childExit.then(({ code }) => { if (code !== 0 || !result) fail(new Error(`Claude exited unexpectedly (${code})`)); });
  write(child, { type: 'user', message: { role: 'user', content:
    'Use Write exactly ONCE to create probe.txt in the current directory containing capy-proof. The user will deny it; do not retry. Then use AskUserQuestion exactly ONCE with one question "Which proof option?", header "Proof", multiSelect false, and two options: label "Alpha" description "First option", label "Beta" description "Second option". Wait for the answer and then thank the user, mentioning the selected option. Do not use any other tools.' } });
  const permission = await channel.next();
  assert.equal(permission.event, 'request');
  const view = permission.request;
  assert.equal(view.sessionId, `claude:${sessionId}`);
  assert.equal(view.body.kind, 'claudePermission');
  assert.equal(view.body.toolName, 'Write');
  assert.equal(path.resolve(view.body.toolInput.file_path), path.join(dir, 'probe.txt'));
  const exact = context(view);
  write(probe, { context: { ...exact, sessionId: `claude:${randomUUID()}` }, response: { decision: 'deny' } });
  assert.equal((await channel.next()).event, 'rejected', 'Altered context accepted');
  report.checks.push('altered context rejected');
  write(probe, { context: exact, response: { decision: 'deny' } });
  assert.equal((await channel.next()).event, 'resolved', 'Exact deny not resolved');
  report.checks.push('exact permission denied');
  write(probe, { context: exact, response: { decision: 'deny' } });
  // Requests can arrive while the replay rejection is being returned.
  let question;
  for (;;) {
    const event = await channel.next();
    if (event.event === 'request') { assert.equal(question, undefined, 'Unexpected additional request'); question = event; continue; }
    assert.equal(event.event, 'rejected', 'Replay accepted'); break;
  }
  report.checks.push('replay rejected');
  question ||= await channel.next();
  assert.equal(question.event, 'request');
  assert.equal(question.request.sessionId, `claude:${sessionId}`);
  assert.equal(question.request.body.kind, 'question');
  const questions = question.request.body.questions;
  assert.equal(questions.length, 1);
  assert.equal(questions[0].question, 'Which proof option?');
  assert.ok(questions[0].options.some(option => option.label === 'Alpha'));
  answered = true;
  write(probe, { context: context(question.request), response: { answers: { [questions[0].id]: 'Alpha' } } });
  assert.equal((await channel.next()).event, 'resolved', 'Question answer not resolved');
  report.checks.push('exact question answered');
  const exit = await race(childExit);
  assert.equal(exit.code, 0);
  assert.equal(result?.subtype, 'success');
  assert.ok(assistantAfterAnswer, 'No assistant text after answering');
  assert.match(result.result || '', /Alpha/i, 'Final result does not acknowledge selected option');
  assert.equal(fs.existsSync(path.join(dir, 'probe.txt')), false, 'Denied Write created a file');
  report.checks.push('assistant continued with selected answer', 'denied file absent');
  report.passed = true;
  probe.stdin.end();
  console.log(`PASS: Claude print hook interventions. Report: ${path.join(dir, 'report.json')}`);
} catch (error) {
  report.error = error.stack || String(error);
  console.error(`FAIL: ${error.message}. Report: ${path.join(dir, 'report.json')}`);
  process.exitCode = 1;
} finally {
  clearTimeout(deadline);
  for (const child of children) killOwnTree(child);
  if (ownedMarker) {
    try {
      regular(marker);
      const current = fs.lstatSync(marker);
      if (current.dev === ownedMarker.dev && current.ino === ownedMarker.ino
          && current.birthtimeMs === ownedMarker.birthtimeMs && current.mtimeMs === ownedMarker.mtimeMs
          && fs.readFileSync(marker, 'utf8') === markerContent) {
        fs.unlinkSync(marker);
      } else {
        report.markerCleanup = 'Marker changed externally; preserved';
      }
    } catch (error) { report.markerCleanup = `Marker preserved: ${error.message}`; }
  }
  report.finishedAt = new Date().toISOString();
  fs.writeFileSync(path.join(dir, 'report.json'), JSON.stringify(report, null, 2));
}
