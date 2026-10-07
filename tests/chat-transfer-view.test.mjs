import test from 'node:test';
import assert from 'node:assert/strict';
import {chatTransferForm} from '../src/presentation.ts';

test('chat transfer exposes all review fields exact billing and fresh required consent without HTML injection',()=>{
  const target={kind:'claudeCli',profileId:'test',provider:'Claude',account:'<img src=x>',billing:'subscription',credentialRevision:null};
  const review={nonce:'own-nonce',sourceId:'source',sourceRevision:1,sourceTarget:target,destination:{...target,kind:'api',provider:'OpenAI',account:'Work',billing:'api'},model:'chosen-model',summary:Object.fromEntries(['objective','decisions','state','files','tests','nextSteps','guides'].map(key=>[key,'</textarea><script>bad</script>']))};
  const html=chatTransferForm(review);
  assert.equal((html.match(/<textarea /g)??[]).length,7);
  assert.match(html,/name="reviewed" required/);assert.match(html,/name="billingConfirmed" required/);
  assert.match(html,/Assinatura Claude/);assert.match(html,/API · cobrança por uso/);assert.match(html,/OpenAI · Work/);
  assert.ok(!html.includes('<script>'));assert.ok(!html.includes('<img'));assert.ok(!html.includes(' checked'));
  const same=chatTransferForm({...review,destination:target});
  assert.match(same,/name="reviewed" required/);assert.ok(!same.includes('name="billingConfirmed"'));
});
