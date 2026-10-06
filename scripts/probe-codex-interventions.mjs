import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { randomUUID } from 'node:crypto';
import { connect, daemonExecutable } from './codex-probe-client.mjs';

const project = join(resolve(import.meta.dirname, '..'), 'scratch', `codex-routing-${randomUUID()}`);
await mkdir(project, {recursive:true});
await writeFile(join(project, 'AGENTS.md'), 'Isolated Capy routing test. Do only the requested question. No tools except request_user_input, no file access, apps or delegation.\n');
const executable = daemonExecutable();
const owner = await connect(executable);
let observer;
let late;
let threadId;
let turnId;
const result = {};
try {
  const started = await owner.request('thread/start', {
    cwd:project, sandbox:'read-only', approvalPolicy:'on-request',
    developerInstructions:'Integration test. Follow each test prompt exactly. For the bootstrap reply OK with no tools. For the question use only request_user_input. After the answer finish with OK.',
    config:{'features.apps':false,'features.multi_agent':false},
  });
  threadId = started.thread.id;
  const bootstrap = await owner.request('turn/start', {threadId,input:[{type:'text',text:'Bootstrap this isolated test session. Reply OK with no tools or question.'}]});
  turnId = bootstrap.turn.id;
  const bootDone = await owner.event(v => v.method === 'turn/completed' && v.params.threadId === threadId && v.params.turn.id === turnId);
  assert.equal(bootDone.params.turn.status,'completed');
  turnId = undefined;
  observer = await connect(executable);
  const resumed = await observer.request('thread/resume', {threadId,excludeTurns:true});
  assert.equal(resumed.thread.id, threadId);
  const turn = await owner.request('turn/start', {
    threadId, input:[{type:'text',text:'Invoke request_user_input exactly once. Question id capy_route, question Which test path?, options Continue and Stop with short descriptions. After the answer finish with OK.'}],
    collaborationMode:{mode:'plan',settings:{model:started.model,reasoning_effort:null,developer_instructions:null}},
  });
  turnId = turn.turn.id;
  const ownRequest = await owner.event(v => v.method === 'item/tool/requestUserInput' && v.params.threadId === threadId);
  assert.equal(ownRequest.params.turnId, turnId);
  const relevant = v => v.method === 'item/tool/requestUserInput' && v.params.threadId === threadId && v.params.turnId === turnId;
  const earlyRequest = await observer.event(relevant, 3000).catch(error => {
    if (error.message !== 'Expected Codex event timed out') throw error;
    return null;
  });
  late = await connect(executable);
  const lateResumed = await late.request('thread/resume', {threadId,excludeTurns:true});
  assert.equal(lateResumed.thread.id, threadId);
  const lateRequest = await late.event(relevant, 3000).catch(error => {
    if (error.message !== 'Expected Codex event timed out') throw error;
    return null;
  });
  result.subscribedObserverReceived = earlyRequest !== null;
  result.resumedPendingObserverReceived = lateRequest !== null;
  assert.ok(earlyRequest, 'Subscribed observer did not receive the new request');
  assert.ok(lateRequest, 'Resumed observer did not receive the pending request');
  const status = (await late.request('thread/read', {threadId, includeTurns:false})).thread.status;
  assert.ok(status.activeFlags?.includes('waitingOnUserInput'));
  result.freshObserverSeesWaiting = true;
  const answers = Object.fromEntries(ownRequest.params.questions.map(q => [q.id,{answers:['Continue']} ]));
  const replyClient = lateRequest ? late : earlyRequest ? observer : owner;
  const replyRequest = lateRequest ?? earlyRequest ?? ownRequest;
  replyClient.send({id:replyRequest.id,result:{answers}});
  result.responseThroughObserver = replyClient !== owner;
  const completed = await owner.event(v => v.method === 'turn/completed' && v.params.threadId === threadId && v.params.turn.id === turnId);
  assert.equal(completed.params.turn.status, 'completed');
  turnId = undefined;
  result.ownerResponseCompleted = true;
  const nextTurn = await owner.request('turn/start', {
    threadId,input:[{type:'text',text:'Invoke request_user_input exactly once again. Question id capy_route_next, question Which next test path?, options Continue and Stop. After the answer finish with OK.'}],
    collaborationMode:{mode:'plan',settings:{model:started.model,reasoning_effort:null,developer_instructions:null}},
  });
  turnId = nextTurn.turn.id;
  const nextRequest = await late.event(v => v.method === 'item/tool/requestUserInput' && v.params.threadId === threadId && v.params.turnId === turnId);
  assert.notEqual(nextRequest.id,replyRequest.id,'Server reused a resolved callback ID');
  late.send({id:replyRequest.id,result:{answers}});
  await new Promise(resolve=>setTimeout(resolve,1000));
  const stillWaiting = (await late.request('thread/read',{threadId,includeTurns:false})).thread.status;
  assert.ok(stillWaiting.activeFlags?.includes('waitingOnUserInput'),'Expired response resolved a later request');
  result.expiredResponseDidNotResolveNextRequest = true;
  late.send({id:nextRequest.id,result:{answers:Object.fromEntries(nextRequest.params.questions.map(q=>[q.id,{answers:['Stop']}]))}});
  const nextCompleted = await owner.event(v => v.method === 'turn/completed' && v.params.threadId === threadId && v.params.turn.id === turnId);
  assert.equal(nextCompleted.params.turn.status,'completed');
  turnId = undefined;
  result.nextExactResponseCompleted = true;
  await writeFile(join(project, 'proof.json'), JSON.stringify(result,null,2));
  console.log(JSON.stringify(result));
} finally {
  if (threadId) {
    if (turnId) await owner.request('turn/interrupt',{threadId,turnId}).catch(()=>{});
    await owner.request('thread/archive',{threadId}).catch(()=>{});
  }
  await Promise.allSettled([late?.close(), observer?.close(), owner.close()]);
}
