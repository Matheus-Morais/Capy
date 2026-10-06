# Espera real do Codex

Profile: light. Feature base: d24eedf.

## Sources

- Conversa: seguir com a validação de pedidos reais de aprovação/resposta e sua remoção.
- `NEXT_STEPS.md`: item 1, reconexão e limites de 64 sessões/1 MiB.
- `.design/codex-activity.md`: contrato de observação e descarte de estados antigos.
- Documentação oficial: https://learn.chatgpt.com/docs/app-server; schemas gerados pelo executável local.

## Out of scope

Responder pelo Capy, terminal, cotas, Antigravity e QA visual. Não interromper o daemon compartilhado nem alterar conversas externas. A prova de falha do daemon será de contrato; a reconexão real usará proxies próprios.

## Landing

Reutiliza o release, descoberta e monitor de atividade. Acrescenta um verificador opt-in que cria somente uma conversa própria por execução, com projeto em scratch e respostas de teste pelo cliente de origem.

| Door | Literal shape | Alternative rejected |
| --- | --- | --- |
| Transporte do verificador | WebSocket nativo do Node por ponte TCP loopback efêmera para stdio do proxy | implementação manual de frames duplicaria um protocolo já disponível |
| Aprovação de teste | negar a única solicitação; nunca aceitar execução ou alteração de política | aceitar comandos exigiria uma superfície de efeitos desnecessária |

## Checks

### S1 — Prova runtime e fronteiras · ~6 arquivos · ~32 KB · ~8k leitura

**C1** — Pedido real `item/commandExecution/requestApproval` da sessão própria coincide com `waitingOnApproval` em `thread/read` e `waiting` no relatório release.
Proof: `node scripts/verify-codex-waiting.mjs` — `approval_waiting`.

**C2** — Pedido real `item/tool/requestUserInput` da sessão própria coincide com `waitingOnUserInput` em `thread/read` e `waiting` no relatório release.
Proof: mesmo comando — `input_waiting`.

**C3** — Responder a cada pedido na origem remove sua flag e o release sai de `waiting`, com request/command nulos; o arquivo da aprovação negada não existe.
Proof: mesmo comando — `approval_cleared`, `input_cleared`, `denied_write_absent`.

**C4** — Um proxy de observação novo continua vendo a espera pendente; ao descarregar a sessão própria ela desaparece do relatório release.
Proof: mesmo comando — `fresh_proxy_waiting`, `own_session_removed`.

**C5** — Após falha, estado waiting passa a unknown sem pedido, e nova observação permite waiting e depois idle.
Proof: Rust `waiting_recovers_after_connection_failure`.

**C6** — 65 IDs carregados produzem exatamente 64 leituras; ID 65 não é consultado.
Proof: Rust `session_limit_bounds_reads`.

**C7** — Mensagem WebSocket acima de 1 MiB é rejeitada.
Proof: Rust `oversized_websocket_message_is_rejected`.

## Swept

- Validation: C1/C2 verificam identidade e projeto; clientes mutáveis limitados ao ID criado por esta execução.
- Failure/dependency: C5 e prazo finito do runner; falhas não serão transformadas em PASS.
- Retry/idempotency: C4/C5; cada execução usa diretório exclusivo.
- Authorization: negar aprovação; responder somente a perguntas da sessão própria. Observador Capy permanece somente leitura.
- Concurrency/ordering: esperar evento real antes de snapshot; responder depois das provas de espera.
- Lifecycle: interromper somente turno próprio, descarregar somente sessão própria e recolher proxies no finally.
- Transitions: C1–C5; idle não é done.
- Observability: saída com nomes de provas e evidência resumida sem conversa/caminhos/IDs locais.

## Handoff

Um único slice (~8k leitura); build no agente principal. Verificador independente depois do commit final. Profile light: sem injeção de falhas em worktree nem QA visual.
