import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { mkdir, readFile, writeFile, access } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { randomUUID } from 'node:crypto';
import { connect, daemonExecutable } from './codex-probe-client.mjs';

const exec = promisify(execFile);
const root = resolve(import.meta.dirname, '..');
const project = join(root, 'scratch', `codex-waiting-${randomUUID()}`);
const release = join(root, 'src-tauri/target/release/capy.exe');
const deniedFile = join(project, 'denied.txt');
await mkdir(project, { recursive: true });
await writeFile(join(project, 'AGENTS.md'), 'Isolated Capy verification. Do only the exact requested test. Do not explore files, invoke skills, use apps, or delegate.\n');
const executable = daemonExecutable();
const client = await connect(executable);
let ownId;
let turnId;
const proofs = [];
function pass(name, evidence = {}) {
  proofs.push({ name, ...evidence });
  console.log(JSON.stringify({ proof: name, result: 'PASS', ...evidence }));
}
async function snapshot() {
  const path = join(project, `activity-${randomUUID()}.json`);
  await exec(release, ['--activity-report', path], { windowsHide: true, timeout: 20000 });
  const report = JSON.parse(await readFile(path, 'utf8'));
  return report.sessions.find(session => session.id === `codex:${ownId}`);
}
async function waitUntil(read, predicate, label, timeout = 20000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    const value = await read();
    if (predicate(value)) return value;
    await new Promise(resolve => setTimeout(resolve, 200));
  }
  throw new Error(`${label} timed out`);
}
async function runtime(observer = client) {
  return (await observer.request('thread/read', { threadId: ownId, includeTurns: false })).thread.status;
}
async function waiting(flag, name) {
  const status = await waitUntil(runtime, value => value.type === 'active' && value.activeFlags?.includes(flag), flag);
  const session = await waitUntil(snapshot, value => value?.state === 'waiting', 'Capy waiting');
  assert.equal(session.origin, project);
  assert.equal(session.request, null);
  assert.equal(session.command, null);
  pass(name, { flag, capyState: session.state });
  return status;
}
async function cleared(flag, name) {
  await waitUntil(runtime, value => !value.activeFlags?.includes(flag), 'Cleared flag');
  const session = await waitUntil(snapshot, value => value && ['working', 'idle'].includes(value.state), 'Capy cleared');
  assert.equal(session.request, null);
  assert.equal(session.command, null);
  pass(name, { capyState: session.state });
}
async function start(text, collaborationMode) {
  const result = await client.request('turn/start', {
    threadId: ownId, input: [{ type: 'text', text }], ...(collaborationMode ? { collaborationMode } : {}),
  });
  turnId = result.turn.id;
}
async function completed() {
  const event = await client.event(value => value.method === 'turn/completed' && value.params.threadId === ownId && value.params.turn.id === turnId);
  assert.equal(event.params.turn.status, 'completed', 'Test turn did not complete normally');
  turnId = undefined;
}
try {
  const started = await client.request('thread/start', {
    cwd: project, sandbox: 'read-only', approvalPolicy: 'on-request', approvalsReviewer: 'user',
    developerInstructions: 'You are running an isolated integration test. Use only the exact tool requested. Do not inspect unrelated files, invoke skills, use apps, delegate, or retry denied actions. After a denied command or an answered question, finish with OK.',
    config: { 'features.apps': false, 'features.multi_agent': false },
  });
  ownId = started.thread.id;
  await writeFile(join(project, 'session.json'), JSON.stringify({ threadId: ownId, project }));
  console.log(JSON.stringify({ stage: 'own_thread_created' }));
  await start(`Integration test: call exec_command exactly once to run PowerShell Set-Content -LiteralPath '${deniedFile}' -Value 'capy-test', with sandbox_permissions=require_escalated and a short justification. The client will decline it. Do not try an unprivileged command first, do not use apply_patch, and do not retry. After denial finish with OK.`);
  const approval = await client.event(value => value.method === 'item/commandExecution/requestApproval' && value.params.threadId === ownId);
  await waiting('waitingOnApproval', 'approval_waiting');
  const observer = await connect(executable);
  try {
    const status = await runtime(observer);
    assert.ok(status.activeFlags.includes('waitingOnApproval'));
    pass('fresh_proxy_waiting', { flag: 'waitingOnApproval' });
  } finally { await observer.close(); }
  client.send({ id: approval.id, result: { decision: 'decline' } });
  await cleared('waitingOnApproval', 'approval_cleared');
  await completed();
  await assert.rejects(access(deniedFile), { code: 'ENOENT' });
  pass('denied_write_absent');
  await start('Integration test: invoke request_user_input exactly once with one question whose id is capy_choice and whose options are Continue and Stop. Ask which test path to use. Wait for the tool response, then finish with OK. Do not ask in plain text and do not use other tools.', {
    mode: 'plan', settings: { model: started.model, reasoning_effort: null, developer_instructions: null },
  });
  const input = await client.event(value => value.method === 'item/tool/requestUserInput' && value.params.threadId === ownId);
  await waiting('waitingOnUserInput', 'input_waiting');
  const answers = Object.fromEntries(input.params.questions.map(question => [question.id, { answers: ['Continue'] }]));
  client.send({ id: input.id, result: { answers } });
  await cleared('waitingOnUserInput', 'input_cleared');
  await completed();
  await client.request('thread/archive', { threadId: ownId });
  await waitUntil(snapshot, value => value === undefined, 'Own session removal');
  pass('own_session_removed');
  await writeFile(join(project, 'proof.json'), JSON.stringify({ passed: true, proofs }, null, 2));
} finally {
  if (ownId) {
    if (turnId) await client.request('turn/interrupt', { threadId: ownId, turnId }).catch(() => {});
    await client.request('thread/archive', { threadId: ownId }).catch(() => {});
  }
  await client.close();
}
