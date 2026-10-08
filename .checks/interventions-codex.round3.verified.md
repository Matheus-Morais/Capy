# Verificação de intervenções Codex

**Veredito**: FAIL — as provas funcionais passaram; QA visual nativo continua sem observação
**Perfil**: light (checklist; sem AGENTS.md no repositório)
**Range**: f5434a1..76c4ce986df1e23cfb3f9f83746b8e06ab364365
**Rodada**: 3 — scoped (nova asserção C6 e lacuna anterior; demais conclusões carregadas explicitamente)
**Verificador**: independente do autor; somente leitura no código de produto

## Fontes vinculantes

| Fonte | Estado nesta rodada | Contradição / limite |
|---|---|---|
| Conversa identificada em `.checks/interventions-codex.md:4` | carried from cc1a01a24f403fec4c3b4e7a4b616f2cc5676b78; a transcrição não está no workspace nem foi fornecida no brief | Não consigo confrontar decisões que dependam exclusivamente da conversa. |
| `PRODUCT.md`, `.design/capy.md` | lidos nesta rodada; registram dados de demonstração como sintéticos e ações reais a provar separadamente | Nenhuma contradição encontrada para o slice Codex. |
| `.checks/interventions-spike.md` | lido nesta rodada; a seção Codex registra transporte inscrito e limites das provas anteriores | Nenhuma contradição nos formatos e fluxos implementados. |
| `DESIGN.md` | lido nesta rodada, especialmente linhas 101–103, 176–183 | Realização deve ficar na visão experimental; respostas pedem conexão explícita e escolha por pedido. Sessões são linhas planas com projeto/agente/origem e status/ação. Não especifica arranjo interno novo para o conteúdo de intervenção. |
| Codex App Server, documentação oficial | aberta nesta rodada; seção Approvals, linhas 1992–2023 | Confirma pedidos de aprovação iniciados pelo servidor, IDs thread/turn, eventos `serverRequest/resolved` e limpeza de perguntas pendentes. Nenhuma divergência observada. [Documentação oficial](https://learn.chatgpt.com/docs/app-server) |

## Checks

| Check | Evidência em código e execução nesta rodada | Resultado |
|---|---|---|
| C1 — contrato e limites | `src-tauri/src/interventions/tests.rs:14-21` compara RPC ID/thread/turn/item e envelope da resposta; `:31-36` rejeita identidades vazias, multilinha e acima do limite; `:38-50` valida IDs RPC. O teste nomeado executou em `cargo test --manifest-path src-tauri/Cargo.toml interventions::tests:: -- --nocapture`. | PASS |
| C2 — contexto e decisões | `src-tauri/src/interventions/tests.rs:102-120` compara comando/cwd/motivo e payload decline e rejeita `acceptForSession`, amendment e decisões desconhecidas; `:122-138` rejeita contextos especiais e filtra decisão disponível. Mesmo comando Rust; teste nomeado executou. | PASS |
| C3 — identidade e submissão única | `src-tauri/src/interventions/tests.rs:184-200` varia nonce, geração, sessão, thread, turn e item; cada identidade alterada retorna `Err(Invalid::Identity)` e deixa estado pending. O mesmo teste também cobre envio único e estado submitting (linhas 225–230; conferido na rodada 2 e carregado, sem alteração da superfície). | PASS |
| C4 — resolução e ciclo de conexão | `src-tauri/src/interventions/tests.rs:292-310` cobre resolved, conclusão/interrupção, encerramento e mudança de conta; envio permanece submitting até evento e callback é removido depois. Teste nomeado executou; os casos adicionais de geração/conexão foram carregados da rodada 2. | PASS |
| C5 — inscrição e protocolo | `src-tauri/src/interventions/tests.rs:378-382` exige initialize e capability; o restante do teste valida handshake, account/read, thread/resume, resposta no ID certo e ausência de mensagens extras (linhas 394–447, rodada 2). Teste nomeado executou. | PASS |
| C6 — controles e clique explícito | `tests/presentation.test.mjs:36-59` testa escaping, botão somente em Codex, segredo como password e estado conectado; `:61-70` rejeita contexto oculto, demo, submitting ou geração ausente. A nova prova `:71-85` chama `runInterventionSubscriptionClick`: ação diferente não chama subscribe; botão explícito chama uma vez com `[session.id,true]`, fica disabled/aria-busy e clique duplicado não repete. `src/dashboard.ts:67-70` usa essa função no handler real de clique. Os quatro testes nomeados C6 foram encontrados por `rtk rg` e executados por `npm test -- --test-name-pattern="..."`; resultado 13/13 passou (o runner executou todo o arquivo). | PASS funcional |
| C7 — entrega real no release | Build com feature: `npm run desktop:build -- --features intervention-proof` passou. `node scripts/verify-codex-interventions.mjs` passou com `question_delivered, expired_rejected, next_request_preserved, approval_declined, denied_write_absent`. O script exige literalmente `item/fileChange/requestApproval` em `scripts/verify-codex-interventions.mjs:122`, body files e caminho alvo em `:124-125`, decline em `:127`, e ausência do arquivo em `:132`. Build de entrega `npm run desktop:build` passou sem a feature; `src-tauri/Cargo.toml:8` declara a feature e `src-tauri/src/main.rs:373-378,585` condiciona o harness. | PASS |

## Validação executada

| Comando | Resultado |
|---|---|
| `cargo test --manifest-path src-tauri/Cargo.toml interventions::tests:: -- --nocapture` | PASS — 5 testes, 0 falhas |
| `npm test -- --test-name-pattern="real interventions preserve exact request identity and discard stale controls|intervention submission resolves only the current visible pending Codex identity|only an explicit Codex subscription button click invokes the connection|intervention answer and focus survive redraw only while the same request nonce remains"` | PASS — 13 testes, 0 falhas; os quatro nomes C6 apareceram individualmente como aprovados |
| `npm run build` | PASS — TypeScript e Vite |
| `npm run desktop:build -- --features intervention-proof` | PASS — executável release do harness compilado |
| `node scripts/verify-codex-interventions.mjs` | PASS — cinco flags C7 |
| `npm run desktop:build` | PASS — release sem `intervention-proof`; warnings de código inalcançável e variáveis não usadas |
| `npm run verify:native` | PASS — `passed: True`, summary/panel/pet prontos, `uiErrors: []` |
| QA visual nativo | NÃO OBSERVADO — ver abaixo |

## QA visual nativo

O inventário de Computer Use retornou `apps: []`; a API `cua.listWindows` não está disponível no runtime ativo. O smoke nativo comprova inicialização e prontidão das três janelas, mas não exibe nem captura os cartões Codex. Não consegui observar pixels nem verificar composição e estados visuais reais; portanto esse gate permanece aberto e determina o FAIL.

`.checks/native-smoke.json` estava limpo no início, foi alterado pelo script e restaurado com `git restore` ao baseline. Não removi artefatos scratch: o script de C7 pode coexistir com artefatos de rodadas anteriores e não fiz limpeza que pudesse apagar prova alheia.

## Política e cobertura

O checklist não tem seções `Test policy` ou `Coverage`; não há linhas para julgar/recomputar. O perfil é `light`, portanto fault injection da seção 4 do verifier não se aplica. A etapa geral de composição visual nativa continua explicitamente pendente. Sem a conversa original, decisões exclusivamente nela não foram verificadas.

## Lacuna remanescente

1. QA visual nativo dos cartões reais e seus estados: requer janela nativa acessível para inspecionar a tela. Smoke e testes de markup não substituem essa observação.

