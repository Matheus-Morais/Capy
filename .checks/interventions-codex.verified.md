# Verificação de intervenções Codex

**Veredito**: FAIL
**Perfil**: light
**Range**: f5434a1..cc1a01a24f403fec4c3b4e7a4b616f2cc5676b78
**Rodada**: 1 — full
**Verificador**: subagente independente (autor != verificador)

## Fontes vinculantes

| Fonte | Aberta | Contradição | Não coberto |
|---|---|---|---|
| Conversa indicada em `.checks/interventions-codex.md:4` | Não; o brief desta verificação confirmou autorização para a prova da conversa própria, mas não forneceu a conversa como fonte | Não foi possível determinar se a decisão posterior substitui a regra anterior em `DESIGN.md` | A decisão de produto que deveria resolver esse conflito permanece sem verificação; nenhum conteúdo ou identificador da conversa foi copiado para este relatório |
| `PRODUCT.md` e `.design/capy.md` | Sim | Nenhuma decisão concreta sobre o layout de cartões de intervenção foi encontrada | A composição dos controles reais de intervenção não é decidida por esses artefatos |
| `.checks/interventions-spike.md` | Sim | Nenhuma; registra a prova de transporte e limites de escopo | Não define apresentação visual ou fluxo no Capy |
| [Codex App Server](https://learn.chatgpt.com/docs/app-server) | Sim | Nenhuma no contrato provado: a documentação descreve pedidos de aprovação e evento `serverRequest/resolved` em linhas 2002–2019 | Não define a apresentação do produto Capy |
| `DESIGN.md` | Sim | `DESIGN.md:103` afirma que todas as sessões/respostas exibidas são sintéticas e que a interface não implica integração real; C6 introduz pedidos e respostas reais do Codex. A fonte de conversa que poderia substituir essa decisão não foi disponibilizada | Também não especifica a composição visual dos novos cartões reais |

## Checks

| Check | Prova executada | Evidência localizada | Resultado |
|---|---|---|---|
| C1 — contrato e limites | `cargo test interventions::tests:: -- --nocapture` — exit 0; os cinco testes nomeados passaram | `src-tauri/src/interventions/tests.rs:14` — `assert_eq!(request.rpc_id, json!(77));`; `:15` confere `(thread_id, turn_id, item_id)` contra os três valores esperados; `:20` confere o envelope da resposta; `:61` verifica `(1..=4).contains(&count)`; `:74` verifica `(2..=4).contains(&count)`; `:83` verifica `len == 4096`; `:93` rejeita 64 KiB e `:96` rejeita método não suportado | PASS |
| C2 — contexto e decisões de aprovação | Mesmo comando Rust — exit 0 | `src-tauri/src/interventions/tests.rs:102` compara `request.body` com `Body::Command { command, cwd, reason }`; `:120` rejeita decisão não permitida; `:129` rejeita contexto de rede/permissões; `:133` rejeita stdin; `:144` confere `Body::Files` | PASS |
| C3 — identidade e submissão única | Mesmo comando Rust — exit 0 | `src-tauri/src/interventions/tests.rs:194` — `assert_eq!(registry.prepare(...), Err(Invalid::Identity), "{field}");`; `:225` confere resposta serializada; `:228` — `assert_eq!(registry.views()[0].status, "submitting");`; `:230` rejeita repetição | PASS |
| C4 — expiração e ciclo de conexão | Mesmo comando Rust — exit 0 | `src-tauri/src/interventions/tests.rs:301` — `assert_eq!(registry.views()[0].status, "submitting", "send must not imply resolved");`; `:307` — `assert!(registry.views().is_empty());`; `:328` confere geração nova; `:336` rejeita callback antigo | PASS |
| C5 — inscrição e protocolo | Mesmo comando Rust — exit 0 | `src-tauri/src/interventions/tests.rs:379` confere `initialize`; `:394` confere `account/read`; `:411` confere `thread/resume`; `:412` — `assert_eq!(resume["params"], json!({"threadId":"thread-a","excludeTurns":true}));`; `:434` confere resposta ao ID exato; `:447` verifica que nenhuma mensagem extra é enviada | PASS |
| C6 — apresentação e resposta na interface | `npm test -- --test-name-pattern="real interventions preserve exact request identity and discard stale controls|intervention submission resolves only the current visible pending Codex identity|intervention answer and focus survive redraw only while the same request nonce remains"` — exit 0; os três testes nomeados aparecem como `ok` | `tests/presentation.test.mjs:39` — `assert.match(html, /Header &lt;x&gt;/);`; `:41` confere ação de enviar; `:43` exclui inscrição Codex da linha Claude; `:53` confere requisição pending; `:55` exclui modo demo; `:70` restaura resposta e `:75` não transfere resposta a outro nonce | **FAIL — cobertura incompleta:** o fixture usa `isSecret:false` em `tests/presentation.test.mjs:36`; nenhum teste usa `isSecret:true` ou verifica `type="password"`. O teste de apresentação verifica markup, não composição visual nativa. O checklist exclui explicitamente fixture de navegador como substituta de QA visual (`.checks/interventions-codex.md:8`), mas `scripts/verify-native.ps1:10` roda apenas `--self-test` e `:15` consulta `passed`; não captura/inspeciona a tela. A inscrição por ação explícita também não tem asserção de interação no trio nomeado. |
| C7 — entrega real no executável release | `npm run desktop:build -- --features intervention-proof` — exit 0; depois `node scripts/verify-codex-interventions.mjs` — exit 0. Resultado literal: `PASS: question_delivered, expired_rejected, next_request_preserved, approval_declined, denied_write_absent`. `npm run desktop:build` sem feature — exit 0 | `scripts/verify-codex-interventions.mjs:80` confere tipo/ID do pedido e `:85` conclusão após resposta; `:95` exige nonce diferente; `:97` rejeita callback vencido; `:100` confere que o pedido seguinte ainda espera; `:122` exige pedido fileChange; `:127` envia decline; `:132` exige arquivo ausente. `src-tauri/Cargo.toml:8` declara feature opcional; `src-tauri/src/main.rs:373` compila harness só com feature, `:378` o exclui sem feature e `:585` restringe a função de prova | PASS |

## Política de testes

O checklist não contém seção `Test policy`; nenhum requisito de política foi declarado para auditar. Os testes Rust e JavaScript nomeados existem dentro do range e foram executados no HEAD. O filtro Rust selecionou cinco testes, todos listados individualmente na saída. A execução JavaScript listou os três nomes solicitados individualmente como `ok` (a chamada rodou 12 testes no total, todos passando).

O checklist não contém uma tabela `Coverage`; não há join declarado para recomputar. Nos conjuntos explicitamente enumerados nas alegações, C1–C5 têm asserções correspondentes acima. C6 deixa sem prova `isSecret: true`/campo password e inscrição por ação explícita; a inspeção de composição nativa requerida também não foi realizada.

## Faults injetados

Não aplicável ao perfil `light`; injeção de faults é exigida somente nos perfis `standard` e `ui`.

## Gates executados

| Comando | Evidência |
|---|---|
| `cargo test interventions::tests:: -- --nocapture` | exit 0 — `5 passed; 0 failed; 35 filtered out`; todos os cinco nomes aparecem individualmente |
| `npm test -- --test-name-pattern="<três nomes C6 do checklist>"` | exit 0 — os três testes C6 aparecem individualmente como `ok`; 12 testes rodaram, todos passaram |
| `npm run build` | exit 0 — `tsc --noEmit` e Vite concluíram |
| `npm run verify:native` | exit 0 — `passed: True`, janelas `summary`, `panel`, `pet` prontas e `uiErrors: {}`; é smoke de startup, não QA visual |
| `npm run desktop:build -- --features intervention-proof` | exit 0 — build release concluiu |
| `node scripts/verify-codex-interventions.mjs` | exit 0 — os cinco flags C7 passaram |
| `npm run desktop:build` | exit 0 — build release de entrega sem a feature de prova concluiu; compilador emitiu warnings de código inalcançável/variáveis não usadas |

## Lacunas em ordem

1. **C6 — QA visual nativo e composição.** O checklist exige que a fixture de navegador não substitua o QA visual nativo, mas o smoke apenas inicializa as janelas. Não há evidência localizada de inspeção dos pedidos reais em tela.
2. **C6 — resposta secreta e inscrição.** As asserções nomeadas não exercitam a ramificação `isSecret: true` nem provam que a inscrição só ocorre após ação explícita. Embora `src/presentation.ts:8-10` escolha `type="password"`, a expressão de produção não substitui a asserção exigida pelo check.
3. **Fontes — decisão que resolve regra anterior.** A fonte “conversa” está listada, mas não foi incluída no material entregue ao verificador. Com isso, não dá para verificar se a decisão de intervenção real substituiu o compromisso sintético em `DESIGN.md:103`.
