import test from 'node:test';
import assert from 'node:assert/strict';
import { sessionRow } from '../src/presentation.ts';
import { captureInterventionAnswers, pendingIntervention, questionAnswer } from '../src/interventions-ui.ts';

const threadId = 'claude-thread-1';
const session = { id:`claude:${threadId}`, project:'Project', agent:'Claude Code', symbol:'C', kind:'claude', origin:'C:\\work', state:'waiting', request:null, message:'Aguardando resposta.', command:null, hidden:false };
const request = (overrides = {}) => ({
  nonce:'nonce-1', generation:'claude:generation-1', sessionId:session.id, threadId, turnId:'turn-1', itemId:'item-1', status:'pending', decisions:['allow','deny'], body:{kind:'claudePermission', toolName:'Bash', toolInput:{command:'echo <script>'}, cwd:'C:\\work\\<project>'}, ...overrides,
});
const snapshot = (candidate = request(), overrides = {}) => ({ scenario:'real', sessions:[session], interventions:[candidate], subscriptions:[], integrations:[], reduceMotion:false, ...overrides });

test('Claude interventions accept only matching visible real Claude identities', () => {
  const current = request();
  assert.equal(pendingIntervention(snapshot(current), current.nonce), current);
  assert.equal(pendingIntervention(snapshot({...current, sessionId:`codex:${threadId}`}), current.nonce), undefined);
  assert.equal(pendingIntervention(snapshot({...current, generation:'run:1'}), current.nonce), undefined);
  assert.equal(pendingIntervention(snapshot(current, {sessions:[{...session, kind:'codex'}]}), current.nonce), undefined);
  assert.equal(pendingIntervention(snapshot(current, {sessions:[{...session, hidden:true}]}), current.nonce), undefined);
  assert.equal(pendingIntervention(snapshot(current, {scenario:'waiting'}), current.nonce), undefined);
});

test('Claude interventions render escaped permission details without Codex subscription actions', () => {
  const html = sessionRow(session, true, [request()]);
  assert.match(html, /Permissão do Claude/);
  assert.match(html, /<strong>Ferramenta<\/strong><code>Bash<\/code>/);
  assert.match(html, /echo &lt;script&gt;/);
  assert.match(html, /C:\\work\\&lt;project&gt;/);
  assert.doesNotMatch(html, /<script>|connect-interventions|Conectar respostas/);
  assert.match(html, /data-action="decide-intervention"/);
  assert.match(html, /data-decision="allow">Permitir<\/button>/);
  assert.match(html, /data-decision="deny">Negar<\/button>/);
});

test('Claude interventions render multi-select plus free text, while Codex keeps exclusive Other', () => {
  const claudeQuestion = {
    ...request(), body:{kind:'question',questions:[{id:'q',header:'Choose',question:'Which?',options:[{label:'A',description:''},{label:'B',description:''}],isOther:false,isSecret:false,multiSelect:true,allowFreeText:true}],},
  };
  const html = sessionRow(session, true, [claudeQuestion]);
  assert.match(html, /Pergunta do Claude/);
  assert.equal((html.match(/type="checkbox"/g) ?? []).length, 2);
  assert.match(html, /data-free-answer="q"/);
  assert.deepEqual(questionAnswer(claudeQuestion.body.questions[0], '  custom answer  ', ['A','B']), 'custom answer');
  assert.deepEqual(questionAnswer(claudeQuestion.body.questions[0], '', ['A','B']), ['A','B']);
  assert.equal(questionAnswer(claudeQuestion.body.questions[0], '', []), undefined);

  const codexSession = {...session, id:'codex:thread-c', kind:'codex'};
  const codexQuestion = {...claudeQuestion, sessionId:codexSession.id, body:{kind:'question',questions:[{id:'other',header:'Other',question:'Answer',options:[],isOther:true,isSecret:false}]}};
  const codexHtml = sessionRow(codexSession, true, [codexQuestion]);
  assert.match(codexHtml, /Pergunta do Codex/);
  assert.match(codexHtml, /data-free-answer="other"/);
  assert.doesNotMatch(codexHtml, /type="checkbox"/);
  assert.equal(questionAnswer(codexQuestion.body.questions[0], '  free form  ', []), 'free form');
});

test('Claude interventions capture distinct selected options from the same multi-select question', () => {
  const field = value => ({
    name:'answer-nonce-1-q', value, checked:true, dataset:{questionAnswer:'q',answerOption:value === 'A' ? '0' : '1'},
    closest:() => ({dataset:{request:'nonce-1'}}), focus() {},
  });
  const saved = captureInterventionAnswers([field('A'),field('B')], null);
  assert.equal(saved.size, 2);
  assert.equal(saved.get('nonce-1\0choice:q:answer-nonce-1-q:option:0').value, 'A');
  assert.equal(saved.get('nonce-1\0choice:q:answer-nonce-1-q:option:1').value, 'B');
});

