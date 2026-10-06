# Acesso à origem — verificação independente

**Verdict: PASS — C1–C4 no limite das provas unitárias do slice Codex (4/4).**
**Profile:** light. **Diff range:** 839e6a2..27167c2699d2d1ac3f1e2fd16c6e2610e336c84c.
**Round:** 1 — full. **Verifier:** subagente independente, author != verifier.

Este PASS não comprova seleção de uma conversa no aplicativo de origem, não fecha 0.3.0 nem o objetivo das seis etapas. Todas as evidências abaixo foram verificadas em HEAD 27167c2. Nenhuma abertura de link, criação ou retomada de conversa foi executada.

## Fontes abertas

Foram lidos `.checks/source-access.md`, PRODUCT.md, DESIGN.md, `.design/capy.md` (incluindo Sessões) e `.impeccable/surfaces/src-style-css.md`. O perfil light não exige a enumeração formal de telas do passo 1, o join Coverage nem fault injection. Não há seção Test policy ou tabela Coverage no checklist.

- [Commands — Deep links](https://learn.chatgpt.com/docs/reference/commands): o contrato distingue `codex://threads/<thread-id>` de criação e de parâmetros de prompt/path. O URL gerado corresponde à forma canônica para chat local existente.
- [ShellExecuteW](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shellexecutew): sucesso exige retorno maior que 32; recomenda inicialização COM. Implementação usa ambos.
- [AssocQueryStringW](https://learn.microsoft.com/en-us/windows/win32/api/shlwapi/nf-shlwapi-assocquerystringw): sucesso S_OK e saída terminada em null; implementação valida resultado, tamanho e primeiro caractere. A existência de associação não é confirmação de seleção do chat.

Não foi identificada contradição do slice com Sessões: identidade, projeto e agente continuam visíveis; capacidade é opcional e demonstrada por agente. DESIGN.md contém descrição anterior de dados inteiramente simulados, mas sua regra de modo real exige não oferecer ações simuladas; o renderer respeita essa separação.

## Checks e assertions

Os testes nomeados existem, foram acrescentados no range e executaram individualmente nas duas invocações batched abaixo. A busca foi `rtk proxy rg -n -C 9 'stale_or_unsupported|source access' src-tauri/src/source_access/tests.rs tests/presentation.test.mjs`, complementada por busca de assertions no mesmo arquivo Rust.

| Check | Prova executada | Evidence | Resultado e alcance |
|---|---|---|---|
| C1 | `source_access::tests::opens_only_the_current_codex_identity` — ok | `src-tauri/src/source_access/tests.rs:22`: `assert_eq!(sent, ["codex://threads/11111111-1111-1111-1111-111111111111"])` | PASS do destino exato e uma chamada ao dispatcher injetado; UUID e Report são sintéticos, com `real=true`. Não prova uma sessão real no app. |
| C2 | `source_access::tests::stale_or_unsupported_requests_never_dispatch` — ok | `src-tauri/src/source_access/tests.rs:37–42` enumera ausência, origin alterada, demo, ID com `?prompt=oops`, kind Claude; `:45` é `panic!("must not dispatch")`; `:47` é `assert!(result.is_err())` | PASS das cinco classes nomeadas antes do dispatcher. Amostra de um ID malformado e de um agente sem destino, não todas as strings nem todos os agentes. |
| C3 | `source_access::tests::missing_handler_and_dispatch_errors_are_visible` — ok | `src-tauri/src/source_access/tests.rs:53–55`: `assert!(registered_handler(…))` para executável/AppID e `assert!(!registered_handler(|_| false))`; `:56–58` assertam sucesso 33 e mensagens de erro 32/0; `:63` compara motivo ausente; `:70` compara `"Windows refused"` | PASS do fallback e propagação de erro nos helpers. Consulta Win32 real e COM não são executadas por esse teste. Os códigos 33/32/0 cobrem a fronteira, sem enumerar todos os erros Windows. |
| C4 | `source access is driven by backend availability and escapes its reason` — ok | `tests/presentation.test.mjs:7`: `assert.doesNotMatch(…, /data-action="open-source"/)`; `:9` ação disponível; `:12–14` disabled, label/reason escapados e ausência de `<script>`; `:16–17` terminal simulado e ausência de open-source | PASS do HTML produzido pelo renderer para ausência, disponível, indisponível e demo. Não exercita clique, IPC ou WebView nativa. |

Inspection adicional: `src-tauri/src/main.rs:217` recebe somente `id`, localiza a Session no snapshot real e faz nova descoberta; `src-tauri/src/source_access.rs:11–13` deriva URL apenas de kind Codex/UUID; `:39–45` exige mesma identidade/kind/origin atual. `:65–69` mapeia consulta executável/AppID às constantes Win32. `:107–114` passa apenas URL ao ShellExecuteW, com parâmetros e diretório null. Essa leitura suporta o wiring, sem convertê-lo em teste de integração.

## Swept relido contra o código

| Restrição | Evidência em HEAD | Limite |
|---|---|---|
| Validation/Authorization | `source_access.rs:11–13,34–45`; `main.rs:217–234` | Provas dos helpers; sem prova invocando o comando Tauri. |
| Failure/dependency/Transitions | `source_access.rs:46–49,62–82,122–128`; C3 | Associação pode mudar depois da consulta; erro do dispatcher retorna erro. |
| Idempotency | `dashboard.ts:45–48` bloqueia repetição durante chamada; `:19–21` reaplica disabled/aria-busy ao render; URL não contém prompt | Não há teste de clique concorrente; não se promete deduplicação entre as duas janelas. |
| Concurrency | `main.rs:232–234` refaz descoberta; `source_access.rs:39–45` compara origin/ID/kind | Corrida entre scan e entrega ao app permanece, conforme checklist. |
| Lifecycle | `discovery.rs:186–189` deriva source_action em cada scan; `source_access.rs:11–23` não armazena destino | Preferências existentes persistem IDs ocultos, não URLs; relatório diagnóstico é snapshot, não mecanismo de abertura. |
| Observability | `dashboard.ts:49–53` retorna erro a showError; `bridge.ts:45–49` mostra texto com textContent; mensagens próprias de source_access não contêm conversa | Não foi capturado erro na UI nativa. |

## UI inspecionada

Imagem `scratch/source-access-preview.png` aberta pela ferramenta de imagem. A fixture sintética ocupa coluna de 380px e mantém linhas planas/divisórias, identidade e origem legíveis. Botão disponível tem affordance normal; indisponível aparece desabilitado, com motivo completo em duas linhas abaixo, sem corte ou sobreposição visível. O motivo preserva a hierarquia de texto secundário. `src/style.css:33` usa cor muted e 12px/1.5 existentes; ação/motivo pertencem à área de ações, sem nova região ou card aninhado. A mudança de cursor distingue indisponibilidade e busy.

Isso comprova apenas esses dois estados na imagem enviada. Não há inspeção de painel completo, foco/teclado, erro, loading ou interação real nessa fixture; o screenshot não prova seleção do ID no aplicativo nativo. Não apareceu defeito visual concreto nessa amostra.

## Gates executados

- `rtk proxy cargo test --manifest-path src-tauri/Cargo.toml`: 32 passed, 0 failed, 0 ignored, 0 filtered out; todos os três source_access nomeados aparecem com `ok`.
- `node --test tests/*.test.mjs`: 8 passed, 0 failed, 0 skipped; C4 aparece individualmente.
- `rtk npm run build`: TypeScript e Vite passaram (20 módulos).
- Nenhum release build foi executado. Fault injection não executada: perfil light.

## Lacunas e conclusão delimitada

1. **Prova exigida pela etapa completa:** abrir uma conversa própria no app e comprovar seleção do ID, sem criar conversa. Ausente; associação e sucesso ShellExecute não bastam. Nenhuma ação em conversas do usuário foi feita pelo verificador.
2. **Lacuna de nível no slice:** testes encerram em `open_verified`/helpers e string do renderer. Não cobrem comando Tauri → descoberta → API Windows, enriquecimento `source_action` real, nem clique/busy/erro pelo DOM. O PASS de 4/4 vale para as provas nomeadas do checklist light, não para essa cadeia completa.
3. **Lacuna de amostragem:** um ID malformado, Claude como agente sem destino e três valores de retorno Windows. Não há afirmação de cobertura exaustiva.
4. **Escopo restante:** origem Claude e Antigravity continuam pendentes com prova real por agente. 0.2.0-alpha.1 é prévia com trabalho em andamento, não release final nem conclusão de 0.3.0.

Não houve mutação de código, commit ou push. A única escrita versionável do verificador é este relatório.
