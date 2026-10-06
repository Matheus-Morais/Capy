# Cotas Codex — verificação independente

**Verdict**: PASS — delimitado aos cinco checks abaixo, sem encerrar a etapa de cotas ou a release 0.5.0.
**Profile**: light.
**Diff range**: 2791b48..f5979cd9a88544ce68193ef04a4a56071bf58fa7.
**Round**: 1 — full.
**Verifier**: subagente independente, sem contexto herdado; não escreveu implementação/testes/requisitos.

## Fontes e snapshot

Checklist lida: `.checks/quotas-codex.md`. Nenhuma fonte foi marcada binding; o confronto UI com fontes/design não é executado pelo perfil light. Não há seção `Coverage` ou `Test policy`; seus joins/veredictos não foram executados. Nenhuma fault injection foi realizada, conforme light.

HEAD confirmado em f5979cd. Diff completo da feature: 23 arquivos, 839 inserções, 24 remoções; os quatro testes nomeados fazem parte desse diff. `git status --porcelain` inicialmente vazio. `git diff --check 2791b48..f5979cd` passou. Nenhum AGENTS.md aplicável em src, src-tauri, tests ou scripts; os AGENTS encontrados estavam em scratch de provas isoladas. Instruções fornecidas de RTK e `C:/Users/MOBILTEC/.codex/RTK.md` foram lidas e aplicadas.

## Checks

Todos os nomes abaixo foram localizados com `rtk rg -n` e contexto, e apareceram individualmente no resultado do runner, no HEAD indicado.

| Check | Prova executada | Evidência localizada e expressão que assenta a claim | Resultado |
| --- | --- | --- | --- |
| C1 | Rust `quotas::tests::quota_protocol_is_read_only_and_checks_identity_twice` | `src-tauri/src/quotas/tests.rs:61`: `assert_eq!(methods, ["initialize", "initialized", "account/read", "account/rateLimits/read", "account/read"])`; :43 `assert_eq!(req["params"], json!({"refreshToken":false}))`; :71 `assert!(socket.read().is_err(), "No mutation or thread call may follow quota reads")`. :83 compara flag com `notify`, :85 rejeita `Error::AccountChanged`. | PASS |
| C2 | Rust `quotas::tests::account_bucket_and_window_validation` | `src-tauri/src/quotas/tests.rs:109` e :119: tuplas de uso/duração/reset iguais a `(25.0, 300, 2000)` e `(42.0, 10080, 3000)`; :105–118 conta, bucket, primary/secondary literais. :131 rejeita mudança de conta com `Some(Error::AccountChanged)`; :154, :165, :172, :181 rejeitam tabela local de contas inválidas, buckets vazios/inválidos, metadado maior que limite e 65 buckets; :209 `assert!(rows[0].window.is_none())`, :210 estado `"unavailable"`, para a tabela local de dados de janela inválidos. | PASS |
| C3 | Rust `quotas::tests::refresh_expiry_failure_and_recovery` | `src-tauri/src/quotas/tests.rs:220`: `cache.rows(1_059_999, || panic!("must not poll before 60s"))`; :224 tentativa em 1_060_000 falha, :225 `failed.iter().all(|r| r.window.is_none())`; :227 mensagem contém `"última observada"`; :229 recupera janela. :230 janela existe no limite de 120s, :231 ausente em 120s+1; :234 janela ausente com instante futuro; :245 `assert!(at_reset[0].window.is_none())` no reset, :246 secondary ainda válida. | PASS |
| C4 | JS `real quota rows preserve account and window provenance without stale percentages` | `tests/presentation.test.mjs:55` email escapado, :56 `assert.match(fresh, /codex · Janela principal · 300 min/)`, :59–61 reset/observação/fonte escapada; :62 `assert.doesNotMatch(fresh, /<daemon>\|38%\|72%\|61%/)`; :69 `assert.doesNotMatch(html, /%\|role="meter"/)` sobre tabela local de stale/TTL/futuro/reset/dado inválido/sem conta; :75 demo contém `38%`; :81 após timer de reset remove percentual e meter. Prova adicional :48–50 verifica lista vazia real sem percentuais. | PASS |
| C5 | `node scripts/verify-codex-quotas.mjs` — executável release padrão, após confirmação da compilação f5979cd pelo principal | `scripts/verify-codex-quotas.mjs:20`: `assert.deepEqual(before.account, after.account, 'Account changed during independent read')`; :26 percorre `['primary','secondary']`; :28 `assert.ok(row?.window, ...)`; :29 `assert.equal(row.account, before.account.email, ...)`; :32–33 duração/reset iguais à fonte; :34 percentual finito entre 0 e 100; :36 idade entre 0 e 120000 e :37 antes do reset. :41–42 Claude/Antigravity unavailable com window null. | PASS |

## Swept existente confrontado com código

| Restrição | Evidência do código | Resultado |
| --- | --- | --- |
| Validation | `src-tauri/src/quotas.rs:68–71` texto não vazio, limitado e sem controles; :85 email 320 bytes; :92 até 64 buckets; :97 id 128 bytes; :118 metadado até 65536 bytes; `src-tauri/src/activity/protocol.rs:61–62` websocket/frame 1048576 bytes. | Presente |
| Failure/dependency | `src-tauri/src/activity/proxy.rs:121` timeout 8s; :107 recv_timeout; :111–112 kill e wait do filho próprio. | Presente |
| Idempotency/retry | `src-tauri/src/quotas.rs:6`, :224–229 intervalo 60000ms; `src-tauri/src/activity/protocol.rs:81–92` só sequência de leitura. | Presente |
| Authorization | Proxy do daemon existente em `src-tauri/src/activity/proxy.rs:124–131`; `src-tauri/src/quotas.rs:79–87` conta estável chatgpt com identidade. | Presente |
| Concurrency | `src-tauri/src/monitor.rs:13–25` um worker, cache local serial e caixa Mutex; :43–65 monitor principal copia caixa e publica Snapshot; `src-tauri/src/activity/protocol.rs:45–46`, :85–92 marca account/updated; `src-tauri/src/quotas.rs:39–43` rejeita amostra marcada. | Presente |
| Lifecycle/transitions | `src-tauri/src/quotas.rs:214–218` cache em memória; :258–272 invalida por idade/reset com conversão segundos Unix ×1000; :236–251 falha remove window, usa stale/unavailable, nunca transforma indisponibilidade em zero. | Presente |
| Observability | `src/presentation.ts:35–43` fonte/conta/status/motivo escapados; worker não loga email; `src-tauri/src/monitor.rs:66` erro de emissão apenas. | Presente |

## Limites de evidência

- C1 cruza o protocolo websocket real com servidor de teste; C2/C3 são camada própria de parser/cache. C5 alcança executável release e daemon real por proxy somente leitura. Não houve inferência, leitura de conversas ou impressão de contas/percentuais privados.
- C4 assenta HTML e expiração do renderer com timer simulado. Não assenta geometria, CSS, acessibilidade completa, montagem nativa do desktop ou toda a ligação Snapshot/evento/dashboard. Não há claim de fidelidade visual nestes cinco checks.
- Amostragem C2: os limites de texto do bucket (128 bytes/controles), duração máxima 525600 e todas as combinações de dados inválidos não possuem casos individuais. Os guardas foram localizados; as provas exercitam as categorias e limites 64 buckets/65536 bytes declarados em Swept. Sem afirmação de cobertura exaustiva.
- C5 compara identidade, duração, reset e frescor; valida o intervalo do percentual, sem comparar `usedPercent` por igualdade entre duas leituras feitas em momentos diferentes. O teste de C2 compara uso literal de ambas as janelas. C5 é uma observação válida da conta conectada no momento da execução, sem prova de outras contas/provedores ou permuta de conta real.
- O caminho diagnóstico `--quota-report` escreve relatório privado em scratch por autorização do teste; persistência operacional de saldos não foi introduzida no monitor/cache. O conteúdo privado não foi lido ou copiado para este relatório.
- Fonte vinculante não declarada e perfil light não autorizam conclusão sobre alinhamento com todos os elementos de um design. Sem faults, não há evidência de mutantes mortos. Nenhum level gap obrigatório nos cinco checks delimitados; os limites acima permanecem explícitos.

## Gate

- `rtk proxy cargo test --manifest-path src-tauri/Cargo.toml quotas::tests:: -- --nocapture`: exit 0, 3 testes nomeados passaram, 0 falhas, 32 filtrados.
- `rtk proxy node --test tests/presentation.test.mjs`: exit 0, 7 testes nomeados passaram, 0 falhas.
- `rtk proxy node scripts/verify-codex-quotas.mjs`: exit 0, `PASS: real Codex primary/secondary quota source, stable account, reset units and absence handling`.
- `rtk proxy git diff --check 2791b48..f5979cd`: exit 0.

5/5 checks delimitados provados. Nenhum arquivo de implementação/teste/checklist alterado pelo verificador; apenas este relatório foi escrito. Sem commit.
