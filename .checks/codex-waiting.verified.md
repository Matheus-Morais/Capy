# Espera real do Codex — verificação independente

**Verdict**: PASS — C1–C7 comprovados no escopo declarado.
**Profile**: light.
**Diff range**: d24eedf..36c0cb15980400450c4d7141692e6c8236c89320.
**Round**: 1 — full.
**Verifier**: subagente independente; não escreveu a implementação nem modificou suas assertions.
**Data**: 2026-10-06. Todas as provas abaixo executadas pelo verificador neste HEAD.

## Fontes e escopo

Lidos: `.checks/codex-waiting.md`, `NEXT_STEPS.md`, `.design/codex-activity.md`, todo o diff da etapa, os dois scripts e as assertions Rust. O diff acrescenta apenas scripts, testes e documentação; não altera comportamento de produção. A prova runtime reutilizou o release existente, conforme autorizado.

Documentação oficial aberta: [Codex App Server](https://learn.chatgpt.com/docs/app-server), seções de status, leitura, unsubscribe e archive. A documentação confirma que unsubscribe tem graça de 30 minutos; esta prova usa archive e mede remoção no relatório. Schemas locais consultados: `scratch/codex-schema/codex_app_server_protocol.v2.schemas.json:21652` contém ambas as flags; `CommandExecutionRequestApprovalResponse.json:68` define decline como negação; `ToolRequestUserInputResponse.json:20` define respostas por ID de pergunta. Uma tentativa inicial de abrir `v2/ThreadStatus.json` encontrou arquivo inexistente; a verificação usou o schema consolidado existente.

Comparação visual, recomputação de Coverage, julgamento de Test policy e injeção de faults não se aplicam ao profile light. O checklist não contém tabelas Coverage nem Test policy. Não houve QA visual nem interrupção do daemon compartilhado.

## Checks

| Check | Prova executada e resultado | Evidência que sustenta a claim | Verdict |
|---|---|---|---|
| C1 | Runner: `approval_waiting` PASS, estado release `waiting` | `scripts/verify-codex-waiting.mjs:80` exige evento `item/commandExecution/requestApproval` com `threadId === ownId`; `:43` exige status active e `activeFlags?.includes(flag)`, chamado com `waitingOnApproval` em `:81`; `:44` exige `value?.state === 'waiting'`; `:29` seleciona `codex:${ownId}` e `:46` assert.equal(session.origin, project). | PASS |
| C2 | Runner: `input_waiting` PASS, estado release `waiting` | `scripts/verify-codex-waiting.mjs:96` exige evento `item/tool/requestUserInput` com `threadId === ownId`; `:97` chama waiting com `waitingOnUserInput`, cuja assertion/predicate está em `:43–46`. | PASS |
| C3 | Runner: `approval_cleared`, `input_cleared`, `denied_write_absent` PASS; ambos cleared deram `working` | `scripts/verify-codex-waiting.mjs:88` envia somente `{ decision: 'decline' }`; `:99` responde ao ID do evento input selecionado. `:53` exige ausência da flag, `:54` exige working ou idle, `:55–56` fazem assert.equal(session.request, null) e assert.equal(session.command, null). `:91` faz assert.rejects(access(deniedFile), { code: 'ENOENT' }). | PASS |
| C4 | Runner: `fresh_proxy_waiting`, `own_session_removed` PASS | `scripts/verify-codex-waiting.mjs:82` cria nova conexão; `:85` faz assert.ok(status.activeFlags.includes('waitingOnApproval')); `:102` arquiva somente ownId e `:103` exige snapshot === undefined. Trata-se de novo proxy real e remoção medida, não simulação de reconexão. | PASS |
| C5 | Rust: `activity::tests::waiting_recovers_after_connection_failure ... ok` | `src-tauri/src/activity/tests.rs:136–142`: waiting, apply(Err(())), assert_eq!(state, "unknown"), request/command is_none, nova observação waiting; `:147–149` exige idle e nenhum pedido/comando. Contrato de falha, não reinício real do daemon. | PASS |
| C6 | Rust: `activity::tests::session_limit_bounds_reads ... ok` | `src-tauri/src/activity/tests.rs:158` gera 65 IDs; `:190` faz assert_eq!(reads, loaded_ids[..64]); `:191` exige erro numa leitura extra ("A 65th read must not be sent"); `:198–199` exige found.len() == 64 e último ID session-63. O servidor mede as requisições do protocolo efetivo. | PASS |
| C7 | Rust: `activity::tests::oversized_websocket_message_is_rejected ... ok` | `src-tauri/src/activity/tests.rs:207` testa 1_048_576 e 1_048_577 bytes; `:222` verifica tamanho exato do payload; `:245` faz assert_eq!(result.is_ok(), size == 1_048_576, "message size {size}"). Prova aceitação na fronteira e rejeição um byte acima através de WebSocket real loopback. | PASS |

As três provas Rust foram adicionadas neste diff e apareceram individualmente na saída completa do runner. As provas runtime também foram adicionadas neste diff, todas executaram e o processo terminou com exit 0. Nenhum filtro sem testes foi usado.

## Varredura de constraints e isolamento

| Constraint | Evidência e conclusão |
|---|---|
| Validation | ID do evento e ID do relatório restritos à sessão criada; projeto verificado em `verify-codex-waiting.mjs:29,46,76,80,96`. |
| Failure/dependency | Prazo de snapshots em `:31–38`, exec release com timeout em `:27`, conexão/request/event com prazos em `codex-probe-client.mjs:77,86,96`. Falhas de provas lançam erro; não são convertidas em PASS. |
| Retry/idempotency | Projeto exclusivo com randomUUID em `verify-codex-waiting.mjs:11`; C4/C5 provaram nova observação e substituição. |
| Authorization | Nova sessão read-only, apps e multi_agent desativados em `:72–74`; única decisão de aprovação codificada é decline em `:88`. Todas as operações mutáveis após criação usam ownId; nenhuma operação mutável recebe ID de uma conversa externa. |
| Concurrency/ordering | Os eventos próprios são esperados antes dos snapshots (`:80–81`, `:96–97`); respostas vêm depois das provas de waiting (`:88`, `:99`). |
| Lifecycle | Observer fecha em finally (`:87`); finally principal interrompe apenas ownId/turnId, arquiva apenas ownId e fecha cliente (`:106–111`). O close mata somente o ChildProcess criado com `app-server proxy`, espera exit e fecha TCP (`codex-probe-client.mjs:33,47–55`); não mata PID do daemon. |
| Transitions | C1–C5 comprovam as transições declaradas. Idle não é usado para alegar conclusão; a conclusão de cada turno próprio exige status completed em `verify-codex-waiting.mjs:67`. |
| Observability | stdout PASS contém nomes, flags e estados (`:21–24`), sem IDs, projeto ou conteúdo das conversas. Relatórios completos permanecem no scratch privado. |
| Constraints existentes | `src-tauri/src/activity.rs:8,63` limita 64 sessões; `:78,89–90` descarta estados/pedidos antigos; `src-tauri/src/activity/protocol.rs` mantém includeTurns false e limites message/frame de 1_048_576. Sem alteração destes contratos no diff. |

Após a execução, releitura do proof.json próprio confirmou passed=true e as sete provas runtime; denied.txt permaneceu ausente. O runner saiu normalmente após seus finally. Verificação read-only de PID/criação confirmou daemon existente vivo. Não encerrei nem reabri Capy; a consulta posterior encontrou zero processos correspondentes ao release e não houve baseline de janela, portanto este relatório não afirma preservação de uma janela Capy pré-existente. Git status era limpo antes e após as provas, antes de escrever este relatório.

Cleanup em falha severa do daemon não foi exercitado: o finally tenta interrupt/archive e ignora seus erros de cleanup, enquanto a falha original continua sendo falha. A execução bem-sucedida comprovou archive e saída dos proxies; não há claim de cleanup completo perante daemon indisponível.

## Gates executados

- `node scripts/verify-codex-waiting.mjs` — exit 0; approval_waiting, fresh_proxy_waiting, approval_cleared, denied_write_absent, input_waiting, input_cleared, own_session_removed: PASS.
- `rtk proxy cargo test --manifest-path src-tauri/Cargo.toml` — exit 0; 25 passaram, 0 falharam, 0 ignorados, 0 filtrados. Saída completa com nomes individuais, incluindo as três provas C5–C7.
- `rtk npm test` — exit 0; 7 passaram, 0 falharam, 0 skipped.
- `rtk npm run build` — exit 0; tsc --noEmit e Vite concluíram.

## Limitações e lacunas

Nenhuma lacuna bloqueante dentro de C1–C7. Falha/reinício real do daemon compartilhado permanece sem prova externa; C5 prova apenas o contrato de substituição dos estados. C4 prova novo proxy enquanto a espera continua pendente, não retomada após indisponibilidade real do daemon. Remoção foi medida após arquivamento da própria sessão, não após unsubscribe. O release preexistente foi executado; não houve novo build Rust release. Resposta pelo Capy, terminal, cotas, Antigravity e QA visual permanecem fora de escopo. Sem faults por profile light.
