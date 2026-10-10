# E2 Claude — validação da rodada

09/10/2026, horário de São Paulo. Perfil light. E2 integral aberta; canal Claude experimental.

| Prova executada | Resultado | Alcance |
| --- | --- | --- |
| `cargo test --manifest-path src-tauri/Cargo.toml claude_intervention -- --nocapture` | 9 PASS | Protocolo; dois sockets/contexto/geração/replay; desconexão/timeout/reset; autenticação/limite/lock; revogação e stop ativo |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 154 PASS, 3 ignorados, exit 0 | Regressão Rust; testes live opt-in ignorados, sem substituí-los por fixtures |
| `npm test` | 47 PASS, exit 0 | Contratos/UI e regressões JavaScript |
| `scripts/verify-claude-settings.ps1` | PASS; repetido pelo verificador | Configuração isolada, opt-in/downgrade/remoção e preservação |
| `npm run build` | PASS, exit 0 | TypeScript e Vite |
| `cargo build --manifest-path src-tauri/Cargo.toml --features intervention-proof` | PASS | Build debug de prova; hash no relatório live |
| `verify-claude-interventions.mjs --temporary-optin --passive-permission-host` | PASS, seis checks reais | Claude 2.1.296 print, deny e pergunta por hook; [resultado](evolution-e2-live-report.json), [eventos](evolution-e2-live-events.jsonl) |
| `npm run desktop:build` | PASS, exit 0; seis warnings | Build release normal, sem probe habilitado; [manifesto](evolution-e2-build.json), [log](evolution-e2-build.log) |
| `scripts/verify-native.ps1` | PASS, 20 checks, sem UI errors | Inicialização/janelas/smoke; [relatório](evolution-e2-native-smoke.json), sem exercitar cartões Claude reais |

O marker temporário criado pelo runner foi removido ao terminar. Nenhum settings global foi editado. Pastas scratch das duas tentativas anteriores foram preservadas, com FAIL pelos motivos registrados em [amostras](evolution-e2-live.md).

[Revisão independente](evolution-e2-review.md): nenhum defeito substantivo confirmado; C7 parcial e C8 documentação aprovada. Não foram injetadas falhas no perfil light. Aprovação allow, modo interativo, concorrência/cancelamento/timeout reais, QA dos cartões e desempenho com muitos pedidos permanecem sem prova. Essa prévia não encerra E2 nem o plano completo.
