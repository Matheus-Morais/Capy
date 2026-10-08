# Coleta automática Antigravity — verificação

**Verdict**: PASS — 5/5 checks comprovados.
**Profile**: light.
**Diff range**: `148de3c..f747e18` (fix revisado: `4d3233b..f747e18`).
**Round**: 2 — scoped; C1/C5 e fix revistos em `f747e18`, C2/C3/C4 carregados de `4d3233b`; todas as provas reexecutadas em `f747e18`.
**Verifier**: subagente independente (author != verifier).

## Achado resolvido

Round 1 em `4d3233b`: FAIL (3/5). A prova nativa expirou aguardando a primeira amostra automática com saldo: export intermediário identificado e recente, porém sem janelas válidas, podia fechar o job cedo. Artefato original: `scratch/capy-visual-19ca6a44-2e21-4bd4-9764-4f9069ff8f6e/report.json` (exit 1, 8 checks PASS).

Round 2 em `f747e18`: resolvido. Callback em `antigravity_quotas.rs:36` e validação final em `:40` usam `complete_export`: quatro linhas identificadas recentes e pelo menos uma `state=="fresh" && window.is_some()` (`:45-47`). Regressão `automatic_export_waits_for_quotas_not_only_identity` executou PASS: `:144-147`, `quota={}` e timestamp antigo são rejeitados por `assert!(!complete_export(...))`; export completo é aceito por `assert!(complete_export(...))`. Prova nativa sem alterações de assertions/prazos agora PASS, incluindo amostra inicial e segunda periódica.

## Checks

| Check | Prova independente e evidência localizada | Resultado |
|---|---|---|
| C1 — primeira leitura, 60 s, sem modelo/terminal | `automatic_refresh_is_initial_periodic_and_forced` PASS: `antigravity_quotas.rs:126-128`, tabela literal `(1000,false,1), (60_999,false,1), (61_000,false,2), (61_001,true,3)` e `assert_eq!(calls.get(),expected)`. Opt-in real PASS: `:143-144`, `assert_eq!(rows.len(),4)` e exige `state=="fresh" && window.is_some()`. Argv fixo `agy_usage.rs:14`; `CREATE_NO_WINDOW` em `chat_process.rs:136`. Porém a prova nativa expirou antes da primeira amostra com saldo. | FAIL |
| C2 — worker próprio, UI/provedores livres, sem sobreposição | `monitor.rs:11-21`: um worker, Cache local único, `cache.poll` síncrono dentro do loop; mutex de saída só adquirido após coleta. Outros provedores em worker distinto `:26-66`. Prova nativa executou e passou `native_agy_query_keeps_account_navigation_responsive`: `verify-pet-native.mjs:187`, `Date.now()-started<5000` e foco em `#accounts`. | PASS |
| C3 — refresh compartilhado, limite 15 s | `coalesces_and_limits_refresh_requests` PASS. `quota_refresh.rs:20-21`: `assert!(service.request().is_err())`, `assert!(service.take())`, `assert!(!service.take())`, `assert_eq!(service.revision(),1)`. Guarda literal de 15 s em `:8`; revision não consumida por take (`:13-14`). Workers usam revision (`monitor.rs:16-17`) e take (`:40-41`), respectivamente. | PASS |
| C4 — 30 s, 64 KiB, somente árvore própria | `agy_usage_rejects_model_output_and_malformed_reports` e `agy_usage_bounds_process_output_and_timeout` PASS. `agy_usage.rs:75` rejeita respostas fora do relatório; `:83` exige erro `64 KiB` para 65537 bytes; `:84-85` exige timeout e duração inferior a 3 s no prazo de fixture de 250 ms; `:87` rejeita export que nunca fica pronto. Prazo de produção `:15` é 30 s. Job privado e KILL_ON_JOB_CLOSE em `chat_process.rs:117-128`; teste `chat_process_job_kills_only_owned_tree_when_owner_is_terminated` PASS, `:191` exige morte dos próprios descendentes e `:194` exige sobrevivência de sentinel separado. | PASS |
| C5 — identidade/timestamp, falha não rejuvenesce, amostra recente real | `automatic_failure_keeps_original_observation_and_expires` PASS: `antigravity_quotas.rs:135-136` exige `observed_at==Some(now())` e retira janela após 120001 ms. `rejects_missing_identity_and_never_refreshes_observation_time` PASS: `:107-108` rejeita identidade ausente e preserva `Some(at)` retirando janelas antigas/futuras. Opt-in real PASS em `:143-144`. A prova nativa de renovação identificada com saldo falhou; export-ready não exige janela válida. | FAIL |

## Execuções

- `rtk proxy cargo test --manifest-path src-tauri/Cargo.toml`: exit 0, 144 PASS, 3 opt-ins ignorados; todos os testes nomeados acima apareceram individualmente PASS.
- `rtk proxy cargo test --manifest-path src-tauri/Cargo.toml agy_automatic_live_updates_identified_sample_without_terminal -- --ignored`: exit 0, exatamente 1 PASS, 146 filtrados.
- `rtk proxy npm test`: exit 0, 43 PASS.
- `rtk proxy npm run desktop:build`: exit 0, TypeScript/Vite e release Windows concluídos (warnings preexistentes).
- `rtk proxy node scripts/verify-pet-native.mjs --agy-auto`: exit 1, 8 checks PASS; erro `Consulta automática inicial agy identificada: tempo excedido`. Evidência: `scratch/capy-visual-19ca6a44-2e21-4bd4-9764-4f9069ff8f6e/report.json`.

## Escopo do perfil e sweep

Comparação enumerada de fontes binding, Coverage join, verdicts de Test policy e injeção de faults não executados: perfil light. Não há tabelas formais Coverage/Test policy no checklist. A prova de processo roda em Windows e o opt-in usa o CLI/login/export já existentes; não verifica todas as configurações possíveis. Nenhum prompt de modelo, ajuste externo de login/settings/Token-Watch ou encerramento de processo do usuário foi realizado.

Sweep existing relido: identidade vem do export; timestamp vem do mtime e é preservado (`antigravity_quotas.rs:57-65, :86`); validade de 120 s em `:72`; separação real/demo em `monitor.rs:15, :37-39`; erros não incluem stdout ou credenciais em `agy_usage.rs`; loop sequencial e retry de 60 s em `antigravity_quotas.rs:22-24`. O achado acima impede considerar a fronteira real concluída.
