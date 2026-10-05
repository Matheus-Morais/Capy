# Atividade do Codex — Verification

**Verdict**: PASS — C1–C7 comprovados (7/7).
**Profile**: light.
**Diff range**: 9545a0c..f4c955167cd6cddabdc22d4bf69adb306f739321.
**Round**: 1 — full.
**Verifier**: independent sub-agent (author != verifier), código somente leitura.

Todas as provas executadas nesta rodada no HEAD indicado. O release recém compilado fornecido pelo orquestrador foi usado diretamente, sem rebuild; executável observado com LastWriteTime 2026-10-05 19:27:28. Nenhuma conversa iniciada ou retomada e nenhum processo externo encerrado.

## Binding sources

| Source | Opened | Contradiction | Uncovered |
|---|---|---|---|
| `.checks/codex-activity.md` | sim, arquivo completo | nenhuma encontrada | limitações declaradas abaixo |
| `.design/codex-activity.md` | sim, arquivo completo | nenhuma encontrada nos checks | comparação UI exaustiva não exigida por light |
| `.design/session-discovery.md` | sim, arquivo completo | nenhuma: descoberta isolada permanece unknown | sem investigação dos trabalhos fora do escopo |

Os links de documentação nas fontes não foram tratados como provas executadas; os contratos locais acima são as fontes recebidas como binding. Step 1 de inventário de composição UI não executado: perfil light.

## Checks

| Check | Claim | Proof run | Evidence | Result |
|---|---|---|---|---|
| C1 | Estados runtime e formatos/flags desconhecidos | `cargo test --manifest-path src-tauri/Cargo.toml`: `activity::tests::runtime_status_mapping` passou | `src-tauri/src/activity/tests.rs:61`: `assert_eq!(runtime_state(&status).0, expected, "{status}")`; tabela local legível em :29–59 cobre working, ambas flags de espera e sua combinação, idle, notLoaded, systemError, tipo/flag novos, mistura de flag conhecida/desconhecida, flags ausentes ou string e null | PASS |
| C2 | Somente IDs descobertos/carregados com cwd correspondente | mesmo target: `only_matching_live_threads_are_enriched` e `websocket_probe_is_read_only` passaram | `src-tauri/src/activity/tests.rs:80`: `assert_eq!(report.sessions.len(), 2)`; :81–82: estados working/unknown para correspondência/cwd divergente; :175–176: `assert_eq!(found.len(), 1)` e `assert_eq!(found[0].id, "live")` com candidato absent em :174; implementação também rejeita ID retornado divergente em `src-tauri/src/activity/protocol.rs:78` | PASS |
| C3 | Espera removida a cada observação ou falha | mesmo target: `waiting_is_replaced_on_every_observation` passou | `src-tauri/src/activity/tests.rs:102`: estado inicial waiting; :107: `assert_ne!(report.sessions[0].state, "waiting")`, casos working/idle/ausência explícitos em :90–94; :118: `assert_eq!(report.sessions[0].state, "unknown")` após Err; :119–120: request/command ausentes | PASS |
| C4 | Somente initialize, initialized, loaded/list e read sem turnos | mesmo target: `websocket_probe_is_read_only` passou | `src-tauri/src/activity/tests.rs:160`: `assert_eq!(methods, ["initialize", "initialized", "thread/loaded/list", "thread/read"])`; :147: `assert_eq!(message["params"]["includeTurns"], false)`; enum fechada de requests em `src-tauri/src/activity/protocol.rs:10` e envio de initialized em :62 | PASS |
| C5 | Proxy sem resposta interrompido e recolhido | mesmo target: `unresponsive_child_is_reaped` passou no Windows | `src-tauri/src/activity/tests.rs:203`: `assert!(proxy::query_child(..., Duration::from_millis(200)).is_err())`; :206: `assert!(started.elapsed() < Duration::from_secs(5))`; :207: `assert!(child.try_wait().unwrap().is_some())`; produção usa 5 s em `src-tauri/src/activity/proxy.rs:119`, kill/wait em :104–105 | PASS |
| C6 | Cards reais, sem respostas/terminal; idle não conclui mascote | `npm test`: `real activity labels never expose demo actions` e `idle sessions never imply completion` passaram | `tests/presentation.test.mjs:17`: `assert.match(html, new RegExp(label))`, tabela working/waiting/idle/unknown/done→unknown em :15; :18: `assert.doesNotMatch(html, /data-action="(?:allow\|deny\|answer\|terminal)"\|dangerous\|Concluída/)`; :23–24: petState idle tanto isolado quanto junto de done. Renderer usado nos cards em `src/dashboard.ts:14` | PASS |
| C7 | Release lê daemon real e publica estados válidos sem pedidos executáveis | CLI release `--activity-report .checks/activity-local.json` exit 0 e `npm run verify:native` exit 0 | CLI implementado em `src-tauri/src/main.rs:291` e enriquecimento em :298. Após CLI, PowerShell exigiu `$working.Count -ge 1` (observado: 1 Codex working), rejeitou estados fora de working/waiting/idle/unknown e request/command não null. `src-tauri/src/smoke.rs:157`: estados permitidos, :158–159: request/command ausentes; :164–171: `demo_action(..., "allow", ...).is_err()`. Relatório nativo confirmou `real_activity_states_valid=True`, `real_responses_rejected=True`, `passed=True` | PASS |

Os nomes de todas as provas acima apareceram individualmente no output dos runners; nenhum teste filtrado/ignorado. O diff completo inclui os testes de atividade e apresentação. A prova adicional de respostas reais é existente e foi avaliada como restrição Swept, não como teste novo da feature.

## Swept existing e integração

| Constraint | Evidence | Verdict |
|---|---|---|
| Daemon identificado por PID/criação e ainda ativo | `src-tauri/src/activity/proxy.rs:51–55`: GetProcessTimes, criação esperada, STILL_ACTIVE; :61–69 exige codex.exe | presente |
| Fonte ausente/falha descarta estado e gera diagnóstico | `src-tauri/src/activity.rs:77` usa unwrap_or_default; :85 usa unknown; :100 diagnóstico indisponível; prova C3 | presente |
| Retry sem cache; lote serial fora da UI; snapshots substituídos | `src-tauri/src/monitor.rs:6` thread própria, :16–17 scan/enrich por ciclo, :27–28 substituição integral; `src-tauri/src/activity/proxy.rs:110` proxy novo por query, :100 e :104–105 timeout/kill/wait | presente |
| Respostas reais bloqueadas | `src-tauri/src/demo.rs:46–47` rejeita allow/deny/answer no cenário real; :124 `assert!(data.apply(action, "any", "").is_err())` para os três; teste passou | presente |
| Atividade transitória; preferências preservadas | `src-tauri/src/monitor.rs:23` reaplica preferências antes de :27; `src-tauri/src/discovery.rs:438–439` serializa somente IDs ocultos; :443 aplica ocultação. `hidden_preferences_survive_restart` passou | presente |
| Presença separada de atividade | `src-tauri/src/discovery.rs:91` cria unknown; monitor enriquece depois da descoberta em :16–17; enrichment restringe kind codex em `src-tauri/src/activity.rs:61` | presente |
| Limites e observabilidade | `src-tauri/src/activity.rs:8` 64 sessões; `src-tauri/src/activity/protocol.rs:53–54` 1 MiB; `src-tauri/src/activity.rs:98–100` diagnóstico conectado/indisponível; CLI real acima | presente |

## Test policy rows / Coverage / Faults

O checklist não contém seção Test policy nem tabela Coverage. Recomputação de join, julgamento de Test policy e injeção de falhas não executados conforme perfil light. Nenhum código foi mutado.

## Limitações de nível e amostragem

- C6 prova a função de apresentação realmente usada pelos cards; não é automação de DOM ou QA visual dos dois painéis. Não há requisito de prova UI no perfil light recebido.
- C7 tem prova externa real de working nesta instalação. Esperas reais e idle reais não foram fabricados; mapeamentos e remoção de espera são provas de contrato locais. O smoke nativo isolado usa `all`, que permitiria lista vazia: o CLI complementar exigiu ao menos uma sessão real working, eliminando esse vazio na prova composta de C7.
- C2 testa filtro de carregamento, ID ausente e cwd divergente. Rejeição de um ID divergente retornado pelo servidor foi confirmada estruturalmente (:78), sem caso WebSocket próprio.
- C5 injeta orçamento de 200 ms para exercitar timeout; o valor de produção de 5 s foi confirmado em código. Falhas de kill/wait e falhas de identificação do daemon não foram simuladas.
- Limites de 64 sessões e 1 MiB estão presentes no código; testes de fronteira desses limites não foram executados. Nenhum C1–C7 promete cobrir essas fronteiras.

Sem gap bloqueante contra C1–C7 no nível exigido; essas limitações delimitam o PASS e não significam testes externos de todas as combinações.

## Gate

- `npm test`: 7 passed, 0 failed.
- `cargo test --manifest-path src-tauri/Cargo.toml`: 18 passed, 0 failed, 0 ignored, 0 filtered.
- `npm run build`: exit 0 (TypeScript e Vite).
- CLI release `capy.exe --activity-report .checks/activity-local.json`: exit 0; 1 Codex real working, todos os estados válidos, request/command ausentes.
- `npm run verify:native`: exit 0, passed=True, sem uiErrors; checks de atividade e rejeição real True.
- `git check-ignore .checks/activity-local.json`: ignorado. Nenhum ID local reproduzido neste relatório.

Baseline do tree antes da verificação: `.checks/native-smoke.json` já modificado. A execução autorizada do smoke regenerou esse relatório; demais código/arquivos versionados permanecem sem alterações do verificador. Este relatório é o único artefato documental escrito pelo verificador.
