# Cotas Codex — parte da etapa 0.5.0

Profile: light. Base: 2791b48. Sources: conversa (cotas por conta/provedor), PRODUCT.md, .design/capy.md seção Cotas, .checks/quotas-spike.md, https://learn.chatgpt.com/docs/app-server (conta e rateLimits), DESIGN.md/interface existente.

## Landing

Reutilizar o proxy limitado do daemon existente e o monitor; Snapshot ganha quotas. Consulta no máximo a cada 60 s; evidência válida por até 120 s e antes de resetsAt. Não persistir saldo/credenciais. A identidade exibida é a conta conectada que o daemon informou; não associar automaticamente uma cota às sessões ou a outras contas.

| Door | Literal shape | Alternative rejected |
| --- | --- | --- |
| Identidade da fonte | account/read(refreshToken:false), rateLimits/read, account/read novamente; comparar conta antes/depois | juntar saldo com identidade lida só uma vez mistura contas numa troca |
| Estado por janela | provider/account/bucket/period, usedPercent, duração, resetsAt, observedAt, fresh/stale/unavailable | um percentual agregado mistura janelas e omite origem/expiração |
| Falha/expiração | retirar percentual ao falhar atualização, passar de 120 s ou atingir reset | continuar exibindo saldo antigo como atual |
| Responsividade | um worker de cotas, cache serial e caixa de resultado em memória; somente monitor principal publica Snapshot | leitura de cotas inline bloqueia atividade até 8 s; dois publishers podem emitir snapshots fora de ordem |

## Checks

**C1** — protocolo manda somente initialize/initialized, account/read(refreshToken:false) duas vezes e account/rateLimits/read; não retoma threads, renova autenticação ou altera créditos.
Proof: Rust `quotas::tests::quota_protocol_is_read_only_and_checks_identity_twice`.

**C2** — identidade chatgpt/email e bucket têm limites, troca de conta rejeita amostra; primary/secondary preservam uso, duração e reset; conta ausente/API-key/sem email, bucket vazio ou dados inválidos não geram percentual.
Proof: Rust `quotas::tests::account_bucket_and_window_validation`.

**C3** — cache consulta após 60 s, não por ciclo de 5 s; ao falhar, passar 120 s, instante futuro ou alcançar reset remove percentual; sucesso seguinte recupera; marcador de conta antiga fica explícito.
Proof: Rust `quotas::tests::refresh_expiry_failure_and_recovery`.

**C4** — cotas reais exibem conta/fonte, janela e reset, nenhum valor demonstrativo quando ausentes/expiradas; conteúdo externo escapado; demo mantém seus próprios valores.
Proof: JS `real quota rows preserve account and window provenance without stale percentages`.

**C5** — executável consulta fonte real, recebe janelas primary/secondary e conta estável, sem consultar conteúdo de conversas; relatório privado confere campos/expiração.
Proof: `node scripts/verify-codex-quotas.mjs`.

## Swept

- Validation: C2, 64 buckets e 64KiB por metadado, mensagens websocket até 1MiB.
- Failure/dependency: C2/C3; proxy timeout 8 s e filho próprio encerrado.
- Idempotency/retry: C3, leitura a cada 60 s; nenhuma escrita no provedor.
- Authorization: C1, somente conta conectada ao daemon existente.
- Concurrency: leitura serial no worker da fonte, caixa de resultado em memória e somente monitor principal publica; identidade antes/depois e notificação account/updated invalidam a amostra C1/C2.
- Lifecycle: C3; dados só em memória, reset em segundos Unix.
- Transitions: C3, indisponível não é saldo zero.
- Observability: C4, status/motivo e fonte; sem credenciais ou logs de e-mail.

## Handoff

Slice completo no principal (~10 arquivos/25k tokens). Verificador fresh obrigatório após commit. Isso não encerra a etapa de cotas: fontes reais Claude/Antigravity e múltiplas contas ainda precisam de prova e implementação. Release final 0.5.0 continua pendente.

## Execução

35 testes Rust e 9 testes JS passaram no código alpha.2, incluindo C1–C4. C5 passou contra o executável debug; precisa repetir contra o release compilado deste commit. A inspeção visual usa somente uma fixture sintética de navegador, sem equivalência com validação nativa do desktop. Publicação remota dispensada pelo usuário; artefatos locais continuam autorizados.
