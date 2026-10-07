import test from 'node:test';
import assert from 'node:assert/strict';
import {exitConfirmation} from '../src/presentation.ts';

test('exit confirmation names exact resources and distinguishes uncertain consumption',()=>{
  const chat={kind:'chat',id:'own-chat',label:'Own chat',provider:'Claude',account:'Fixture',billing:'subscription',model:'haiku',cwd:null,revision:3,sendNonce:'own-send'};
  const terminal={...chat,kind:'terminal',id:'own-terminal',label:'Terminal integrado',cwd:'C:\\OwnProject'};
  const review={nonce:'own-review',expiresAt:60_000,resources:[chat,terminal]};
  const text=exitConfirmation(review);
  assert.match(text,/Sessão: own-chat/);assert.match(text,/Sessão: own-terminal/);assert.match(text,/Pasta: C:\\OwnProject/);
  assert.match(text,/Claude · Fixture · haiku · Assinatura Claude/);assert.match(text,/consumo pode ter ocorrido/);assert.match(text,/não reenviará mensagens automaticamente/);
  assert.match(text,/Claude · Fixture · modelo inicial: haiku · Assinatura Claude/);
  assert.match(text,/fecha estes terminais integrados/);assert.doesNotMatch(text,/concluído|concluída/);
  assert.doesNotMatch(exitConfirmation({...review,resources:[terminal]}),/consumo pode ter ocorrido/);
  assert.doesNotMatch(exitConfirmation({...review,resources:[chat]}),/fecha estes terminais/);
});
