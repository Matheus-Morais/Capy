import test from 'node:test';
import assert from 'node:assert/strict';
import { sessionRow, petState, quotaRows, renderQuotas } from '../src/presentation.ts';
import { pendingIntervention, interventionContext, captureInterventionAnswers, restoreInterventionAnswers } from '../src/interventions-ui.ts';
import { runInterventionSubscriptionClick } from '../src/intervention-subscription.ts';

const session = { id:'codex:11111111-1111-1111-1111-111111111111', project:'Project <script>', agent:'Codex', kind:'codex', symbol:'O', origin:'C:\\work\\Project', state:'unknown', request:null, command:null, message:'Sessão aberta.', hidden:false };
test('source access is driven by backend availability and escapes its reason', () => {
  assert.doesNotMatch(sessionRow(session, true), /data-action="open-source"/);
  const available = sessionRow({...session, source_action:{ label:'Abrir no Codex', available:true, reason:null }}, true);
  assert.match(available, /data-action="open-source">Abrir no Codex/);
  assert.doesNotMatch(available, / disabled|data-action="(?:allow|deny|answer|terminal)"/);
  const missing = sessionRow({...session, source_action:{ label:'Abrir <app>', available:false, reason:'Link ausente <script>' }}, true);
  assert.match(missing, /data-action="open-source" disabled>Abrir &lt;app&gt;/);
  assert.match(missing, /Link ausente &lt;script&gt;/);
  assert.doesNotMatch(missing, /<script>/);
  const demo = sessionRow({...session, source_action:{ label:'Abrir no Codex', available:true, reason:null }}, false);
  assert.match(demo, /Ver terminal simulado/);
  assert.doesNotMatch(demo, /open-source/);
});
test('real rows show identity and unknown status without executable demo actions', () => {
  const html = sessionRow({...session, request:'permission', command:'dangerous'}, true);
  assert.match(html, /11111111-1111-1111-1111-111111111111/);
  assert.match(html, /Estado desconhecido/);
  assert.match(html, /Project &lt;script&gt;/);
  assert.match(html, /data-action="hide"/);
  assert.doesNotMatch(html, /data-action="(?:allow|deny|answer|terminal)"|dangerous/);
});
test('real activity labels never expose demo actions', () => {
  for (const [state, label] of [['working', 'Trabalhando'], ['waiting', 'Precisa de você'], ['idle', 'Ociosa'], ['unknown', 'Estado desconhecido'], ['done', 'Estado desconhecido']]) {
    const html = sessionRow({...session, state, request:'permission', command:'dangerous'}, true);
    assert.match(html, new RegExp(label));
    assert.doesNotMatch(html, /data-action="(?:allow|deny|answer|terminal)"|dangerous|Concluída/);
  }
});
test('real interventions preserve exact request identity and discard stale controls', () => {
  const request = { nonce:'n-1', generation:'g', sessionId:session.id, threadId:session.id.slice(6), turnId:'turn', itemId:'item', status:'pending', decisions:[], body:{kind:'question',questions:[{id:'q',header:'Header <x>',question:'Choose & continue',isOther:false,isSecret:false,options:[{label:'Safe <option>',description:'Description'}]}]} };
  const html = sessionRow(session, true, [request]);
  assert.match(html, /data-action="connect-interventions"/);
  assert.match(html, /Header &lt;x&gt;/);
  assert.match(html, /Safe &lt;option&gt;/);
  assert.match(html, /data-action="submit-intervention"/);
  assert.doesNotMatch(html, /<x>|<option>/);
  assert.doesNotMatch(sessionRow({...session,kind:'claude'},true), /connect-interventions/);
  const malicious = sessionRow(session,true,[{...request,body:{kind:'command',command:'<img src=x>',cwd:'C:\\<x>',reason:'& please'}}]);
  assert.doesNotMatch(malicious, /<img|<x>/);
  assert.match(malicious, /&lt;img src=x&gt;/);
  assert.match(html, /data-question-answer="q"/);
  assert.match(html, /data-request="n-1"/);
  const secret = sessionRow(session, true, [{...request, nonce:'secret', body:{kind:'question',questions:[{id:'token',header:'Secret',question:'Enter value',isOther:true,isSecret:true,options:[]}]}}]);
  assert.match(secret, /type="password" data-free-answer="token"/);
  assert.match(secret, /autocomplete="new-password"/);
  assert.doesNotMatch(secret, /type="text" data-free-answer="token"/);
  const subscribed = sessionRow(session, true, [], {sessionId:session.id,status:'connected',message:'Conectado'});
  assert.match(subscribed, /data-action="connect-interventions" data-enabled="false"/);
  assert.match(subscribed, /Desconectar respostas/);
  assert.match(html, /data-action="connect-interventions" data-enabled="true"/);
  assert.match(html, /Conexão local explícita · este cartão Codex/);
  assert.doesNotMatch(sessionRow({...session,kind:'claude'},true), /connect-interventions/);
});
test('intervention submission resolves only the current visible pending Codex identity', () => {
  const request = { nonce:'fresh',generation:'run:2',sessionId:session.id,threadId:session.id.slice(6),turnId:'turn-7',itemId:'item-3',status:'pending',body:{kind:'question'},decisions:[] };
  const data = { scenario:'real',sessions:[session],interventions:[request],subscriptions:[],integrations:[],reduceMotion:false };
  assert.equal(pendingIntervention(data,'fresh'),request);
  assert.deepEqual(interventionContext(request),{nonce:'fresh',generation:'run:2',sessionId:session.id,threadId:request.threadId,turnId:'turn-7',itemId:'item-3'});
  assert.equal(pendingIntervention({...data,scenario:'waiting'},'fresh'),undefined);
  assert.equal(pendingIntervention({...data,interventions:[{...request,status:'submitting'}]},'fresh'),undefined);
  assert.equal(pendingIntervention({...data,sessions:[{...session,hidden:true}]},'fresh'),undefined);
  assert.equal(pendingIntervention({...data,interventions:[{...request,generation:''}]},'fresh'),undefined);
});
test('only an explicit Codex subscription button click invokes the connection', async () => {
  const attrs = {};
  const calls = [];
  const subscribe = async (...args) => { calls.push(args); return 'connected'; };
  const other = {dataset:{action:'hide',enabled:'true'},disabled:false,setAttribute:(key,value) => attrs[key]=value};
  assert.equal(runInterventionSubscriptionClick(other, session.id, subscribe), undefined);
  assert.deepEqual(calls, []);
  const button = {dataset:{action:'connect-interventions',enabled:'true'},disabled:false,setAttribute:(key,value) => attrs[key]=value};
  const operation = runInterventionSubscriptionClick(button, session.id, subscribe);
  assert.equal(button.disabled, true);
  assert.equal(attrs['aria-busy'], 'true');
  assert.equal(await operation, 'connected');
  assert.deepEqual(calls, [[session.id,true]]);
  assert.equal(runInterventionSubscriptionClick(button, session.id, subscribe), undefined);
  assert.equal(calls.length, 1);
});
test('intervention answer and focus survive redraw only while the same request nonce remains', () => {
  const field = (nonce, value, checked = false) => {
    const calls = [];
    const input = { name:`answer-${nonce}-q`, value, checked, dataset:{questionAnswer:'q'}, closest:() => ({dataset:{request:nonce}}), focus:options => calls.push(options), calls };
    return input;
  };
  const before = field('same','Continue',true);
  const saved = captureInterventionAnswers([before],before);
  const redraw = field('same','',false);
  restoreInterventionAnswers([redraw],saved);
  assert.equal(redraw.value,'Continue');
  assert.equal(redraw.checked,true);
  assert.deepEqual(redraw.calls,[{preventScroll:true}]);
  const replacement = field('new','',false);
  restoreInterventionAnswers([replacement],saved);
  assert.equal(replacement.value,'');
  assert.equal(replacement.checked,false);
  assert.deepEqual(replacement.calls,[]);
});
test('idle sessions never imply completion', () => {
  const idle = {...session, state:'idle'};
  assert.equal(petState({sessions:[idle]}), 'idle');
  assert.equal(petState({sessions:[idle,{...session,state:'done'}]}), 'idle');
  assert.equal(petState({sessions:[idle,{...session,state:'working'}]}), 'working');
  assert.equal(petState({sessions:[idle,{...session,state:'waiting'}]}), 'waiting');
  assert.equal(petState({sessions:[{...idle,hidden:true}]}), 'sleeping');
});
test('unknown sessions never imply completion on the mascot', () => {
  assert.equal(petState({sessions:[session]}), 'idle');
  assert.equal(petState({sessions:[{...session,hidden:true}]}), 'sleeping');
  assert.equal(petState({sessions:[{...session,state:'done'},session]}), 'idle');
});
test('real quotas are unavailable while demo quota values remain explicit', () => {
  assert.match(quotaRows(true), /ainda não estão conectadas/);
  assert.doesNotMatch(quotaRows(true), /%/);
  assert.match(quotaRows(false), /38%/);
});
test('real quota rows preserve account and window provenance without stale percentages', t => {
  const row = { provider:'Codex', account:'a<me>@example.test', bucket:'codex', period:'primary', state:'fresh', message:'Fonte <daemon>', observedAt:1_000_000, window:{usedPercent:25,windowDurationMins:300,resetsAt:2000} };
  const fresh = quotaRows(true, [row], 1_000_000);
  assert.match(fresh, /a&lt;me&gt;@example.test/);
  assert.match(fresh, /codex · Janela principal · 300 min/);
  assert.match(fresh, /aria-valuenow="75"/);
  assert.match(fresh, /75%/);
  assert.match(fresh, /Renova em/);
  assert.match(fresh, /Consultado em/);
  assert.match(fresh, /Fonte &lt;daemon&gt;/);
  assert.doesNotMatch(fresh, /<daemon>|38%|72%|61%/);
  for (const [changed, now] of [
    [{...row,state:'stale',window:null},1_000_000], [row,1_120_001], [row,999_999],
    [{...row,observedAt:2_000_000},2_000_000], [{...row,window:{...row.window,usedPercent:101}},1_000_000],
    [{...row,account:null},1_000_000],
  ]) {
    const html = quotaRows(true, [changed], now);
    assert.doesNotMatch(html, /%|role="meter"/);
    assert.match(html, /indisponível/);
  }
  const absent = quotaRows(true, [{...row,state:'unavailable',account:null,window:null,message:'Conta ausente'}],1_000_000);
  assert.match(absent, /Conta ausente/);
  assert.doesNotMatch(absent, /%/);
  assert.match(quotaRows(false, [row]), /38%/);
  t.mock.timers.enable({apis:['Date','setTimeout'],now:1_000_000});
  const element = {innerHTML:''};
  const cancel = renderQuotas(element, true, [{...row, window:{...row.window,resetsAt:1001}}]);
  assert.match(element.innerHTML, /75%/);
  t.mock.timers.tick(1000);
  assert.doesNotMatch(element.innerHTML, /%|role="meter"/);
  assert.match(element.innerHTML, /Dado expirado/);
  cancel();
});
