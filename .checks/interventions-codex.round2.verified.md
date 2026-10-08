# Verificação de intervenções Codex

**Veredito**: FAIL
**Perfil**: light (declarado no checklist; `AGENTS.md` não está presente no repositório)
**Range**: f5434a1..7495d0c
**Rodada**: 2 — scoped
**Verificador**: subagente independente (autor != verificador)

## Fontes vinculantes

| Fonte | Estado nesta rodada | Contradição | Não coberto |
|---|---|---|---|
| Conversa indicada em `.checks/interventions-codex.md:4` | carried from `cc1a01a24f403fec4c3b4e7a4b616f2cc5676b78`; o conteúdo não veio no brief e não existe arquivo referenciado no workspace | A contradição anterior com a regra de dados sintéticos foi resolvida na edição de `DESIGN.md:103` | A decisão original da conversa segue sem verificação; checks que dependem dela não podem ser comparados diretamente com essa fonte |
| `PRODUCT.md`, `.design/capy.md` | carried from `cc1a01a24f403fec4c3b4e7a4b616f2cc5676b78` (sem mudança no range desta rodada) | Nenhuma decisão concreta de composição dos cartões foi relatada na rodada 1 | Não especificam o layout dos novos cartões |
| `.checks/interventions-spike.md` | carried from `cc1a01a24f403fec4c3b4e7a4b616f2cc5676b78`; mudança textual pequena no range não altera o contrato Codex citado | Nenhuma | Não decide layout visual |
| [Codex App Server](https://learn.chatgpt.com/docs/app-server) | aberta novamente; documentação atual descreve `item/fileChange/requestApproval` e `serverRequest/resolved` em linhas 2016–2023 | Nenhuma nos formatos cobertos | A documentação não decide a apresentação do Capy |
| `DESIGN.md` | aberta nesta rodada; linha 103 agora afirma que dados de demonstração são sintéticos e integrações reais aparecem na visão experimental real | A contradição registrada em round 1 foi corrigida | Define as regiões/superfícies gerais em linhas 158 e 181–183, mas não a composição interna do cartão de intervenção real |

## Checks

| Check | Prova executada nesta rodada | Evidência da asserção | Resultado |
|---|---|---|---|
| C1 — contrato e limites | `cargo test --manifest-path src-tauri/Cargo.toml interventions::tests:: -- --nocapture` — exit 0; `request_contract_and_bounds` listado como `ok` | `src-tauri/src/interventions/tests.rs:14-20` confere ID RPC, thread/turn/item e resposta; `:61`, `:74`, `:83`, `:93`, `:96` verificam limites de perguntas/opções/texto, payload e método | PASS |
| C2 — contexto e decisões | Mesmo comando Rust — exit 0; `approval_context_and_decisions` listado como `ok` | `src-tauri/src/interventions/tests.rs:102-109` compara o contexto command/cwd/reason esperado; `:120`, `:129`, `:133`, `:144` rejeitam decisões e contextos não suportados e confere arquivo | PASS |
| C3 — identidade e submissão única | Mesmo comando Rust — exit 0; `exact_request_and_single_submission` listado como `ok` | `src-tauri/src/interventions/tests.rs:194` exige `Err(Invalid::Identity)` para cada campo errado; `:225` confere o payload enviado; `:228` confere `submitting`; `:230` rejeita reenvio | PASS |
| C4 — resolução e ciclo da conexão | Mesmo comando Rust — exit 0; `resolution_and_connection_lifecycle` listado como `ok` | `src-tauri/src/interventions/tests.rs:301` confere que envio não vira `resolved`; `:307` exige lista vazia após evento; `:328` confere nova geração; `:336` rejeita callback antigo | PASS |
| C5 — inscrição e protocolo | Mesmo comando Rust — exit 0; `subscription_and_response_protocol` listado como `ok` | `src-tauri/src/interventions/tests.rs:379`, `:394`, `:411-412` conferem initialize/account/read/thread/resume e `excludeTurns:true`; `:434`, `:447` conferem resposta ao ID correto e ausência de mensagens extras | PASS |
| C6 — apresentação, inscrição explícita e controles | `npm test -- --test-name-pattern="real interventions preserve exact request identity and discard stale controls|intervention submission resolves only the current visible pending Codex identity|intervention answer and focus survive redraw only while the same request nonce remains"` — exit 0. O runner executou 12 testes e listou os três nomes como `ok` | `tests/presentation.test.mjs:39-48` verifica texto escapado, ação de resposta e nonce; `:50-52` confere `isSecret:true` como `type="password"` e exclui `type="text"`; `:54-58` confere estados conectado/desconectado, estado inicial habilitado e ausência do controle em Claude; `:64-68`, `:80-87` cobrem identidade visível, submissão e preservação de resposta/foco | **FAIL — incompleto:** nenhuma asserção testa a interação do botão de inscrição nem prova que o backend só inscreve depois do clique; as linhas 54–58 provam apenas o markup/estado do botão. A inspeção visual nativa exigida pelo checklist também não foi possível. O código não recebeu inspeção visual por browser como substituto. |
| C7 — entrega real no release | Build release de prova `npm run desktop:build -- --features intervention-proof` — exit 0; `node scripts/verify-codex-interventions.mjs` — exit 0 | Saída literal: `PASS: question_delivered, expired_rejected, next_request_preserved, approval_declined, denied_write_absent`. `scripts/verify-codex-interventions.mjs:80,85` verifica pedido/resposta da pergunta; `:95-100` exige nonce novo, rejeição expirada e pedido seguinte ainda pendente; `:122,127,132` verifica pedido de arquivo, decisão decline e arquivo ausente | PASS |

## Política de testes e cobertura

O checklist não declara `Test policy` nem `Coverage`; não há linhas dessas seções para julgar ou recomputar. O perfil `light` não exige injeção de faults. As cinco provas Rust e as três provas JS foram reexecutadas no HEAD; os nomes aparecem individualmente na saída. A alteração do range desta rodada cobre as novas asserções C6 e `DESIGN.md`; as provas C1–C5 e C7 foram repetidas integralmente.

## QA visual nativo

`npm run verify:native` — exit 0; `passed: True`, `ready: {panel, pet, summary}`, `uiErrors: {}`. O script `scripts/verify-native.ps1` executa `capy.exe --self-test` em janela oculta e valida inicialização/tamanhos; não mostra os cartões de intervenção.

Tentei obter a janela nativa pelo inventário do Computer Use. `cua.getState()` retornou `apps: []`; não havia janela Capy disponível para inspecionar nem identificador de app para abrir. `cua.listWindows()` também não está disponível no runtime ativo. Portanto não observei pixels nem afirmo QA visual aprovado. A composição de cartões e seus estados reais continuam abertos conforme `.checks/interventions-codex.md:8`.

## Gates executados

| Comando | Evidência |
|---|---|
| `cargo test --manifest-path src-tauri/Cargo.toml interventions::tests:: -- --nocapture` | exit 0 — 5 passed, 0 failed; os cinco testes nomeados aparecem como `ok` |
| `npm test -- --test-name-pattern="<três testes C6 nomeados>"` | exit 0 — 12 passed, 0 failed; os três testes C6 aparecem como `ok` |
| `npm run build` | exit 0 — TypeScript e Vite concluíram |
| `npm run desktop:build -- --features intervention-proof` | exit 0 — release com harness de prova construído |
| `node scripts/verify-codex-interventions.mjs` | exit 0 — cinco flags C7 passaram |
| `npm run desktop:build` | exit 0 — release padrão concluído sem `intervention-proof`; houve warnings de código inalcançável e variáveis não usadas |
| `npm run verify:native` | exit 0 — smoke nativo passou, três janelas prontas, sem erros de UI |
| QA visual nativo | não executado — nenhuma janela/app Capy disponível no runtime de Computer Use |

## Lacunas em ordem

1. **C6 — QA visual nativo e composição dos cartões.** Smoke só verifica startup; nenhuma janela nativa ficou acessível para inspeção.
2. **C6 — comportamento de inscrição por ação explícita.** Asserções novas cobrem os estados do botão e isolamento de Claude, mas não exercitam o clique nem a inscrição resultante.
3. **Fonte conversa.** O conteúdo que o checklist declara vinculante não foi fornecido; essa parte continua sem verificação nesta rodada.
