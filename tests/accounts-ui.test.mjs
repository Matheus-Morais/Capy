import test from 'node:test';
import assert from 'node:assert/strict';
import {profileMarkup, ruleMarkup, defaultThresholds} from '../src/presentation.ts';

test('account display labels verified billing and escapes account paths',()=>{
  const profile={id:'p',label:'Work <script>',configDir:'C:\\<script>',billing:'subscription'};
  const initial=profileMarkup(profile);
  assert.match(initial,/verificar vínculo/);assert.doesNotMatch(initial,/<script>/);
  const verified=profileMarkup(profile,{loggedIn:true,account:'a@example.test',billing:'api',message:'Confirmed'});
  assert.match(verified,/API · cobrança por uso/);assert.match(verified,/a@example.test/);
});
test('quota preferences expose default and independently disabled editable thresholds',()=>{
  assert.deepEqual(defaultThresholds().map(t=>t.percent),[50,60,70,80,90]);
  const html=ruleMarkup({provider:'Claude',account:'<a>',thresholds:[{percent:55,enabled:false},{percent:70,enabled:true}]},0);
  assert.match(html,/value="55"/);assert.match(html,/value="70"/);assert.match(html,/Adicionar percentual/);assert.doesNotMatch(html,/<a>/);
  assert.equal((html.match(/data-enabled checked/g)||[]).length,1);
});
