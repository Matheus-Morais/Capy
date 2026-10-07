import assert from 'node:assert/strict';
import test from 'node:test';
import {chatRecoveryForm} from '../src/presentation.ts';

test('chat recovery keeps consumption uncertain and requires fresh consent without sending or injection',()=>{
  const chat={state:'unknown',target:{provider:'<Claude>',account:'<script>account</script>',kind:'claudeCli',billing:'subscription'},recoveryReview:null};
  const prepare=chatRecoveryForm(chat);
  assert.match(prepare,/data-prepare-chat-recovery/);assert.match(prepare,/consumo pode ter ocorrido/);assert.doesNotMatch(prepare,/<script>/);
  chat.recoveryReview={nonce:'own-nonce',resumeCli:true};
  const resumed=chatRecoveryForm(chat);
  assert.match(resumed,/data-chat-recovery="own-nonce"/);assert.match(resumed,/type="checkbox" name="reviewed" required/);
  assert.doesNotMatch(resumed,/\bchecked\b/);assert.match(resumed,/Confirmar revisão sem reenviar/);assert.match(resumed,/retomará esse UUID/);
  chat.recoveryReview.resumeCli=false;
  assert.match(chatRecoveryForm(chat),/transferência com resumo para uma nova conversa/);
  chat.target.kind='api';assert.match(chatRecoveryForm(chat),/escrever uma nova mensagem/);
  chat.state='completed';assert.equal(chatRecoveryForm(chat),'');
});
