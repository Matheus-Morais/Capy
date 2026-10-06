import test from 'node:test';
import assert from 'node:assert/strict';
import { sessionRow, petState, quotaRows, renderQuotas } from '../src/presentation.ts';

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
