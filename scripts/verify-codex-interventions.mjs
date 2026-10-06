import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { randomUUID } from 'node:crypto';
import { connect, daemonExecutable } from './codex-probe-client.mjs';

const root = resolve(import.meta.dirname, '..');
const project = join(root, 'scratch', `codex-capy-${randomUUID()}`);
await mkdir(project, { recursive: true });
await writeFile(join(project, 'AGENTS.md'), 'Capy integration verification. Follow only the explicit test prompt. Do not use tools except request_user_input during the question tests. During the permission test request only the exact command supplied and stop if denied. No network, delegation, or extra file access.\n');
const executable = daemonExecutable();
const capy = process.env.CAPY_TEST_EXE || join(root, 'src-tauri/target/release/capy.exe');
const owner = await connect(executable);
let cli;
let threadId;
let turnId;
const result = {};
const proofDir = join(root, 'scratch', 'codex-interventions-proof');
async function mark(name) {
  result[name] = true;
  await mkdir(proofDir, { recursive: true });
  await writeFile(join(proofDir, 'proof.json'), JSON.stringify(result, null, 2));
}

async function event(predicate, timeout = 90000) {
  return owner.event(predicate, timeout);
}
function startCapy(sessionId) {
  cli = spawn(capy, ['--intervention-probe', sessionId], { env: { ...process.env, CAPY_INTERVENTION_PROBE_ROOT: project }, stdio: ['pipe', 'pipe', 'pipe'], windowsHide: true });
  let stdout = '';
  let stderr = '';
  const lines = [];
  let wake;
  cli.stdout.setEncoding('utf8').on('data', data => { stdout += data; let at; while ((at = stdout.indexOf('\n')) >= 0) { const line = stdout.slice(0, at); stdout = stdout.slice(at + 1); try { lines.push(JSON.parse(line)); wake?.(); } catch {} } });
  cli.stderr.setEncoding('utf8').on('data', data => { stderr += data; });
  async function next(type, timeout = 15000) {
    const deadline = Date.now() + timeout;
    while (Date.now() < deadline) {
      const index = lines.findIndex(v => v.event === type);
      if (index >= 0) return lines.splice(index, 1)[0];
      if (cli.exitCode !== null) throw new Error(`Capy probe exited (${cli.exitCode}): ${stderr}`);
      await new Promise(resolve => { const timer = setTimeout(resolve, Math.min(500, deadline - Date.now())); wake = () => { clearTimeout(timer); resolve(); }; });
      wake = undefined;
    }
    throw new Error(`Capy probe did not report ${type}: ${stderr}`);
  }
  function send(request, response) {
    const context = Object.fromEntries(['nonce','generation','sessionId','threadId','turnId','itemId'].map(key => [key, request[key]]));
    cli.stdin.write(`${JSON.stringify({ context, response })}\n`);
  }
  return { next, send, stderr: () => stderr };
}

try {
  const started = await owner.request('thread/start', {
    cwd: project, sandbox: 'read-only', approvalPolicy: 'on-request',
    approvalsReviewer: 'user',
    developerInstructions: 'You are running an isolated Capy integration test. Use only the exact tool requested. Do not inspect unrelated files, invoke skills, use apps, delegate, or retry denied actions. After a denied command or answered question, finish with OK.',
    config: { 'features.apps': false, 'features.multi_agent': false },
  });
  threadId = started.thread.id;
  let bootstrap = await owner.request('turn/start', { threadId, input: [{ type: 'text', text: 'Bootstrap this Capy verification conversation. Reply OK without tools.' }] });
  turnId = bootstrap.turn.id;
  await event(v => v.method === 'turn/completed' && v.params.threadId === threadId && v.params.turn.id === turnId);
  turnId = undefined;
  const child = startCapy(`codex:${threadId}`);
  const connected = await child.next('connected');
  assert.equal(connected.sessionId, `codex:${threadId}`);

  const plan = { mode: 'plan', settings: { model: started.model, reasoning_effort: null, developer_instructions: null } };
  const ask = async (id, title, options) => owner.request('turn/start', {
    threadId, collaborationMode: plan,
    input: [{ type: 'text', text: `Use request_user_input exactly once, with id ${id}, header Capy proof, question ${title}, and these options with brief descriptions: ${options.map(option => `${option} (used only for this local verification)`).join('; ')}. After the answer finish with OK.` }],
  });
  bootstrap = await ask('capy_release_path', 'Which release proof?', ['Continue', 'Stop']);
  turnId = bootstrap.turn.id;
  const firstOwner = await event(v => v.method === 'item/tool/requestUserInput' && v.params.threadId === threadId && v.params.turnId === turnId);
  const first = await child.next('request');
  assert.equal(first.request.body.kind, 'question');
  assert.equal(first.request.body.questions[0].id, 'capy_release_path');
  const answer = { answers: { capy_release_path: 'Continue' } };
  child.send(first.request, answer);
  await child.next('resolved');
  const firstDone = await event(v => v.method === 'turn/completed' && v.params.threadId === threadId && v.params.turn.id === turnId);
  assert.equal(firstDone.params.turn.status, 'completed');
  assert.equal(firstOwner.params.questions[0].id, 'capy_release_path');
  await mark('question_delivered');
  turnId = undefined;

  bootstrap = await ask('capy_next_step', 'Which verification step?', ['Continue', 'Stop']);
  turnId = bootstrap.turn.id;
  const secondOwner = await event(v => v.method === 'item/tool/requestUserInput' && v.params.threadId === threadId && v.params.turnId === turnId);
  const second = await child.next('request');
  assert.notEqual(second.request.nonce, first.request.nonce);
  child.send(first.request, answer);
  const stale = await child.next('rejected');
  assert.match(stale.message, /expirado|mudou/i);
  const waiting = await owner.request('thread/read', { threadId, includeTurns: false });
  assert.ok(waiting.thread.status.activeFlags?.includes('waitingOnUserInput'));
  await mark('expired_rejected');
  result.next_request_preserved = secondOwner.params.turnId === second.request.turnId;
  assert.ok(result.next_request_preserved);
  await mark('next_request_preserved');
  child.send(second.request, { answers: { capy_next_step: 'Stop' } });
  await child.next('resolved');
  await event(v => v.method === 'turn/completed' && v.params.threadId === threadId && v.params.turn.id === turnId);
  turnId = undefined;

  const marker = join(project, 'must-not-exist.txt');
  bootstrap = await owner.request('turn/start', {
    threadId,
    collaborationMode: { mode: 'default', settings: { model: started.model, reasoning_effort: null, developer_instructions: null } },
    input: [{ type: 'text', text: `Integration test: use apply_patch exactly once to create ${marker} with only the text capy-test. This is a read-only sandbox, so wait for the file-change approval request; the client will decline it. Do not call exec_command, do not retry, and after denial finish with DENIED.` }],
  });
  turnId = bootstrap.turn.id;
  const approvalNotice = await event(v =>
    ((v.method === 'item/commandExecution/requestApproval' || v.method === 'item/fileChange/requestApproval') && v.params.threadId === threadId && v.params.turnId === turnId)
    || (v.method === 'turn/completed' && v.params.threadId === threadId && v.params.turn.id === turnId),
    90000,
  );
  assert.equal(approvalNotice.method, 'item/fileChange/requestApproval', `The model ended the permission turn without a file-change approval request (status: ${approvalNotice.params.turn?.status ?? 'unknown'}).`);
  const approval = await child.next('request', 90000);
  assert.equal(approval.request.body.kind, 'files');
  assert.ok(approval.request.body.changes.some(change => change.path.endsWith('must-not-exist.txt')));
  assert.ok(approval.request.decisions.includes('decline'));
  child.send(approval.request, { decision: 'decline' });
  await child.next('resolved');
  const denied = await event(v => v.method === 'turn/completed' && v.params.threadId === threadId && v.params.turn.id === turnId, 90000);
  assert.equal(denied.params.turn.status, 'completed');
  turnId = undefined;
  await assert.rejects(readFile(marker));
  await mark('approval_declined');
  await mark('denied_write_absent');

  assert.deepEqual(Object.keys(result).sort(), ['approval_declined', 'denied_write_absent', 'expired_rejected', 'next_request_preserved', 'question_delivered'].sort());
  console.log('PASS: ' + Object.keys(result).join(', '));
} finally {
  if (cli && cli.exitCode === null) {
    cli.stdin.end();
    await new Promise(resolve => { const timer = setTimeout(() => { cli.kill(); resolve(); }, 5000); cli.once('exit', () => { clearTimeout(timer); resolve(); }); });
  }
  if (threadId) {
    if (turnId) await owner.request('turn/interrupt', { threadId, turnId }).catch(() => {});
    await owner.request('thread/archive', { threadId }).catch(() => {});
  }
  await owner.close();
}
