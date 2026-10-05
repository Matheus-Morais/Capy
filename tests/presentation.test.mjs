import test from 'node:test';
import assert from 'node:assert/strict';
import { sessionRow, petState, quotaRows } from '../src/presentation.ts';

const session = { id:'codex:11111111-1111-1111-1111-111111111111', project:'Project <script>', agent:'Codex', kind:'codex', symbol:'O', origin:'C:\\work\\Project', state:'unknown', request:null, command:null, message:'Sessão aberta.', hidden:false };
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
