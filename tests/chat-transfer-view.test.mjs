import test from 'node:test';
import assert from 'node:assert/strict';
import {chatTransferForm} from '../src/presentation.ts';

test('chat transfer exposes all review fields exact billing and fresh required consent without HTML injection',()=>{
  const target={kind:'claudeCli',profileId:'test',provider:'Claude',account:'<img src=x>',billing:'subscription',credentialRevision:null};
  const review={nonce:'own-nonce',sourceId:'source',sourceRevision:1,sourceTarget:target,destination:{...target,kind:'api',provider:'OpenAI',account:'Work',billing:'api'},model:'chosen-model',summary:Object.fromEntries(['objective','decisions','state','files','tests','nextSteps','guides'].map(key=>[key,'</textarea><script>bad</script>']))};
  const html=chatTransferForm(review);
  assert.match(html,/<h2>Revisar transferência<\/h2>/);
  assert.equal((html.match(/<textarea /g)??[]).length,7);
  assert.match(html,/name="reviewed" required/);assert.match(html,/name="billingConfirmed" required/);
  assert.match(html,/Assinatura Claude/);assert.match(html,/API · cobrança por uso/);assert.match(html,/OpenAI · Work/);
  assert.ok(!html.includes('<script>'));assert.ok(!html.includes('<img'));assert.ok(!html.includes(' checked'));
  const same=chatTransferForm({...review,destination:target});
  assert.match(same,/name="reviewed" required/);assert.ok(!same.includes('name="billingConfirmed"'));
  const automatic=chatTransferForm({...review,automatic:true});
  assert.match(automatic,/<h2>Percentual de troca atingido<\/h2>/);assert.match(automatic,/name="reviewed" required/);
  assert.match(automatic,/name="billingConfirmed" required/);assert.ok(!automatic.includes(' checked'));
});
test('chat transfer shows lost results as source metadata outside editable summary fields',()=>{
  const target={provider:'Claude',account:'Fixture',billing:'subscription'};
  const review={nonce:'own',sourceTarget:target,destination:target,model:'haiku',uncertainMessages:[0,12],summary:Object.fromEntries(['objective','decisions','state','files','tests','nextSteps','guides'].map(key=>[key,'Reviewed text']))};
  const html=chatTransferForm(review);
  assert.match(html,/<p data-chat-uncertainty>/);assert.match(html,/mensagens 1, 13 sem resposta confirmada/);
  assert.match(html,/Este aviso acompanha o contexto enviado ao destino/);
  assert.doesNotMatch(html,/name="uncertainMessages"|contenteditable/);
  assert.equal((html.match(/<textarea /g)??[]).length,7);
  assert.doesNotMatch(chatTransferForm({...review,uncertainMessages:[]}),/data-chat-uncertainty/);
});
