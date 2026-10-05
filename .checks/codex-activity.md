# Atividade do Codex

Profile: light. Sources: conversa; `.design/codex-activity.md` (contrato de comportamento);
`.design/session-discovery.md` (presença permanece independente de atividade).
Feature base: 9545a0c. Out of scope: Claude hooks, Antigravity, respostas, cotas e terminais.

## Landing

Reutiliza descoberta, monitor, snapshots e apresentação existentes.

| Door | Literal shape | Alternative rejected |
|---|---|---|
| WebSocket | tungstenite sobre stdio do proxy do daemon existente | protocolo caseiro; manutenção e validação de frames |
| Estado ocioso | `idle` na Session, sem inferir `done` | conclusão inferida de ausência de turno |
| Ciclo isolado | timeout de 5 s, proxy novo por lote, sem cache de estados | conexão persistente com dados que podem sobreviver a falhas |

## Checks

### S1 — Atividade observável · ~10 arquivos · ~35 KB · ~9k de leitura

**C1** — Estados runtime documentados são mapeados conforme o design; formato/flag desconhecidos são unknown.
Proof: Rust `runtime_status_mapping`.

**C2** — Só IDs previamente descobertos e carregados recebem estado; ID/cwd divergentes não recebem estado.
Proof: Rust `only_matching_live_threads_are_enriched`.

**C3** — Uma espera deixa de aparecer após working, idle, ausência ou falha, sem cache.
Proof: Rust `waiting_is_replaced_on_every_observation`.

**C4** — O cliente envia somente initialize, initialized, thread/loaded/list e thread/read com includeTurns false.
Proof: Rust `websocket_probe_is_read_only` (servidor WebSocket de teste).

**C5** — O orçamento interrompe um proxy sem resposta; o processo filho é encerrado e aguardado.
Proof: Rust `unresponsive_child_is_reaped`.

**C6** — Os cards reais mostram working/waiting/idle/unknown e nenhum controle de resposta/terminal; idle não implica conclusão do mascote.
Proof: JS `real activity labels never expose demo actions`; `idle sessions never imply completion`.

**C7** — O release lê o daemon real existente e publica estados válidos sem pedidos executáveis no smoke nativo.
Proof: `capy.exe --activity-report .checks/activity-local.json` (Codex atual working);
`npm run verify:native`: `real_activity_states_valid` e `real_responses_rejected`.

## Swept

- Validation: C1/C2, identidade do daemon por PID/criação.
- Failure/dependency: C3/C5; fonte ausente mantém unknown e diagnóstico.
- Retry/idempotency: próximo ciclo reconecta, C3.
- Authorization: C4; guard de respostas reais existente em demo.rs.
- Concurrency/ordering: um lote serial no monitor; timeout mata proxy; snapshots substituídos, C3/C5.
- Lifecycle: estado só em memória; proxy encerrado, C5; preferências existentes preservadas.
- Transitions: C1/C3; sem conclusão inferida.
- Observability: diagnóstico de atividade conectado/indisponível, relatório local C7.

## Handoff

Um único slice (~9k leitura, abaixo de 150k); build no agente principal. Verificador independente após commit final.

## Gaps

Light: nenhuma injeção de falhas; teste WebSocket local cobre protocolo; teste nativo cobre daemon desta instalação.
Espera real externa não será fabricada: flags cobertas em testes de contrato, não por pedido real do usuário.
