import test from 'node:test';
import assert from 'node:assert/strict';
import {quotaBoard,usableQuota,resetCountdown,renderQuotaBoard} from '../src/quota-board.ts';
import {fallbackMarkup,quotaNotifications} from '../src/presentation.ts';

const now=Date.UTC(2026,9,8,19);
const row={provider:'Claude',account:'own@example.test',bucket:null,period:'five_hour',state:'fresh',observedAt:now,
  window:{usedPercent:76,windowDurationMins:300,resetsAt:now/1000+7200},message:'Official local observation'};

test('unavailable saved fallback is preserved instead of selecting another account',()=>{
  const result=fallbackMarkup([{id:'other',label:'Another account'}],'saved-api-id','own-model');
  assert.match(result,/<option value="saved-api-id" selected disabled>/);
  assert.doesNotMatch(result,/value="other" selected/);
});
test('real quota alerts and routing never appear as simulation and remain available in real mode',()=>{
  const data={scenario:'real',quotaAlerts:[{id:'own',provider:'Codex',account:'own@example.test',period:'primary',percent:50,resetsAt:now/1000+7200,createdAt:now}],routing:[{taskId:'own-task',state:'waitingTurn',message:'Waiting for real turn'}]};
  assert.match(quotaNotifications(data,now),/own@example.test/);assert.match(quotaNotifications(data,now),/Waiting for real turn/);
  for(const scenario of ['waiting','working','done','sleeping']) assert.equal(quotaNotifications({...data,scenario},now),'');
});

test('groups each identity separately and compares windows within an account',()=>{
  const result=quotaBoard(true,[row,{...row,period:'seven_day',window:{...row.window,windowDurationMins:10080}}, {...row,account:'second@example.test'}],now);
  assert.equal((result.match(/class="quota-account"/g)??[]).length,2);
  assert.match(result,/5 horas/);assert.match(result,/Semanal/);assert.match(result,/aria-valuenow="24"/);
  assert.match(result,/Renova em <strong>2h 0min/);
});
test('rejects stale, future, missing identity, reset and invalid balance without a meter',()=>{
  for(const invalid of [{...row,state:'stale'},{...row,observedAt:now-120001},{...row,observedAt:now+1},
    {...row,account:null},{...row,window:{...row.window,resetsAt:now/1000}},
    {...row,window:{...row.window,usedPercent:NaN}},{...row,window:{...row.window,usedPercent:101}}]){
    assert.equal(usableQuota(invalid,now),false);
    const result=quotaBoard(true,[invalid],now);assert.doesNotMatch(result,/role="meter"|aria-valuenow|<small>%/);
  }
  assert.equal(usableQuota({...row,observedAt:now-120000},now),true);
});
test('escapes source labels and messages and marks low remaining balance',()=>{
  const result=quotaBoard(true,[{...row,provider:'<img>',account:'<script>@test',bucket:'<unsafe>',message:'<iframe>',window:{...row.window,usedPercent:90}}],now);
  assert.doesNotMatch(result,/<img>|<script>|<unsafe>|<iframe>/);
  assert.match(result,/&lt;iframe&gt;/);assert.match(result,/Saldo baixo/);assert.match(result,/aria-valuenow="10"/);
});
test('countdown retains days and hours and never claims zero before renewal',()=>{
  assert.equal(resetCountdown(now/1000+90000,now),'1d 1h');
  assert.equal(resetCountdown(now/1000+30,now),'1min');
});
test('local timer drops expired balances without a backend event and cancels',()=>{
  const originalTimeout=globalThis.setTimeout,originalClear=globalThis.clearTimeout,originalDocument=globalThis.document,originalNow=Date.now;
  let callback,cleared=false;
  globalThis.document={activeElement:null};
  globalThis.setTimeout=fn=>{callback=fn;return 42;};globalThis.clearTimeout=id=>{cleared=id===42;};
  const element={innerHTML:'',querySelectorAll:()=>[]};
  try{
    const cancel=renderQuotaBoard(element,true,[row],now);assert.match(element.innerHTML,/role="meter"/);
    Date.now=()=>now+120001;callback();assert.doesNotMatch(element.innerHTML,/role="meter"/);assert.match(element.innerHTML,/Dado expirado/);
    cancel();assert.equal(cleared,true);
  }finally{globalThis.setTimeout=originalTimeout;globalThis.clearTimeout=originalClear;globalThis.document=originalDocument;Date.now=originalNow;}
});
