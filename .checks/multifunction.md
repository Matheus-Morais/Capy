# Implementação da Capy multifuncional

Sources:
- Conversa/entrevista e plano aprovado: funcionalidades, prioridades, consentimento e movimentos.
- `.design/multifunction.md`: decisões e escopo preservados.
- `DESIGN.md`, `prototypes/front-pet.svg`: identidade visual existente a preservar.

Profile: light. Verificador independente depois da última entrega; evidência unitária não equivale a prova real de integração.

## Out of scope

- Publicação remota: dispensada pelo usuário.
- Alterar contas/sessões externas como fixtures de teste: usar somente sessões próprias.

## Landing

Reusar monitor, snapshot/eventos Tauri, pedidos Codex, fontes de quotas e arte SVG. Novos módulos separam ciclo de animação, perfis, política de quota e execução de tarefas.

| Door | Literal shape | Alternative rejected |
| --- | --- | --- |
| Movimento com memória | controlador TS por janela, relógio injetado; aceno 30s, sono 180s | mapear snapshot diretamente a CSS: repete eventos e dorme imediatamente |
| Conclusão real | identificador explícito de conclusão de provedor | inferir de Stop/idle/ausência: falsos positivos |
| Transferência | preparar → revisar → aprovar uma vez → executar | envio silencioso: contradiz escolha do usuário |
| Terminal integrado | `portable-pty` 0.9 + xterm.js 6, com PTY nativo Windows e buffers limitados | pipes em um textarea: não atendem os prompts interativos do CLI |
| Identidade de tarefa | UUID v4 próprio enviado ao CLI via `--session-id` | retomar a conversa mais recente: pode atingir outra tarefa |
| Quotas Claude | statusline oficial com início observado, conta confirmada e TTL 120s; wrapper preserva configuração anterior | ler endpoint OAuth privado/credenciais: contrato não documentado |
| Resumos longos | instruções acima de 8192 bytes em arquivo UTF-8 próprio, anexado pelo CLI com diretório dedicado | argumento direto: ultrapassa o limite real da linha de comando do Windows |
| Persistência de tarefas longas | índice UUID e um JSON limitado a 2 MiB por tarefa; manter 500 entradas | JSON único lido com teto de 1 MiB: perde tarefas válidas ao reiniciar quando o histórico cresce |
| Persistência de revisões | até 64 revisões; arquivo de estado limitado a 128 MiB para comportar resumos/escape JSON | teto de configuração de 1 MiB: perde aprovações/revisões após crescimento |
| Fim de turno Claude interativo | `idle_prompt` ou `StopFailure` com sessão/processo exatos; `Stop` sozinho não autoriza transferência | `Stop` antecede hooks que podem bloquear e continuar o turno; um atraso fixo não prova fim |
| Prova visual nativa | WebView2/CDP somente no processo de teste próprio; preferências e cache em `scratch/capy-visual-*` | renderização somente no navegador: não detecta ACL, eventos e CSS do executável |
| Chat próprio | CLI Claude oficial com sessão UUID, stdin e ferramentas desativadas; APIs OpenAI/Anthropic/Gemini em HTTPS no backend, chaves em CredWrite/CredRead do Windows | chaves no frontend/JSON ou fallback de transporte automático: perde identidade/cobrança |
| Histórico do chat | registro por UUID, mensagens limitadas a 256 KiB por conversa e JSON a 2 MiB; revisão por versão e nonce | reenvio automático após falha: pode duplicar consumo; transferência sem versão: leva contexto antigo |
| Conclusão de chat na mascote | token UUID/versão mantido somente na memória por 10s; emissão imediata e união com o monitor | celebrar o estado persistido completed ao reiniciar ou depender de polling lento: repete ou perde conclusões |
| Configuração Claude nativa | quando CLAUDE_CONFIG_DIR está ausente e a pasta é a padrão canônica USERPROFILE/.claude, omitir a variável em auth/chat/PTY/launcher; demais perfis usam pasta explícita | forçar a variável mesmo no padrão altera a localização da configuração global e perde a identidade observada no CLI real |
| Transferência de chat | revisão por UUID de origem; versão/identidades/cobrança revalidadas; journal próprio de até 8 MiB confirma origem, destino e índice antes de qualquer envio | gravar dois históricos sem journal ou consumir aprovação depois de enviar: permite duplicar destinos após falha/reinício |
| Continuação aprovada no chat | nova conversa com resumo revisado como contexto; primeiro envio inclui esse contexto em CLI/API; criar o destino não chama o provedor | usar histórico local somente na API perde o resumo no CLI; iniciar consumo antes de concluir o commit duplica operações |
| Processo próprio de chat no Windows | criar diretamente em job privado com KILL_ON_JOB_CLOSE e atributos JOB_LIST/HANDLE_LIST; pipes UTF-8; nenhum filho executa antes da associação; persistir tentativa CLI antes do spawn | matar por nome/PID após reinício atinge sessões alheias; associação posterior ao spawn deixa janela de consumo sem proteção; Drop sozinho não executa quando a Capy é encerrada à força |
| Recuperação revisada de chat | revisão persistida no próprio histórico, vinculada a versão/nonce; aprovação sem envio altera unknown para failed com aviso de consumo incerto; mensagens e nonces antigos permanecem; Claude retoma somente UUID/pasta com histórico conferido, sem processo ativo, usando proteção de job registrada; sem histórico, permitir apenas transferência revisada | reenviar a mensagem anterior, marcar completed ou recriar silenciosamente o UUID CLI perde a evidência de incerteza e pode repetir consumo/contexto; sessões CLI antigas sem proteção comprovada exigem conferir a origem |
| Capacidade da continuidade | manter todos os nonces até 512 por conversa, reservando recuperação e transferência antes de iniciar outro envio; 200 mensagens com até uma recuperação por envio usam no máximo 400 nonces, mais uma transferência; anotações de interrupção únicas por mensagem/envio/aprovação | teto de 200 nonces bloqueia a saída antes do limite de mensagens; apagar nonces antigos para liberar espaço permite replay após reinício |
| Incerteza em transferência | resumo inclui todas as mensagens com resultado perdido; revisão apresenta aviso de origem não editável, vinculado às anotações; o contexto enviado mantém esse aviso mesmo se o resumo for reescrito; journal preserva todos os campos de origem além das alterações explícitas da transferência | observar somente lastError perde avisos após uma resposta nova; confiar no campo editável ou restaurar journal sem comparar metadados pode apagar o registro original de incerteza |

## Checks

### S1 — Mascote · ~14 KB de fontes existentes · ~3.5k de leitura

**C1** — Cumprimento acontece uma vez por abertura.
Proof: `node --test --test-name-pattern="greeting" tests/pet-behavior.test.mjs`

**C2** — Sono exige 180000ms sem trabalho/pedido; interação acorda.
Proof: `node --test --test-name-pattern="sleep" tests/pet-behavior.test.mjs`

**C3** — Pedido novo acena; mesmo pedido repete após 30000ms até visto.
Proof: `node --test --test-name-pattern="attention" tests/pet-behavior.test.mjs`

**C4** — Somente conclusão explícita real ou demo concluída gera comemoração; evento repetido não repete.
Proof: `node --test --test-name-pattern="completion" tests/pet-behavior.test.mjs`

**C5** — Hover olha apenas dentro da mascote; clique reage e preserva abrir/fechar e arrastar.
Proof: inspeção visual/funcional nativa registrada em `.checks/multifunction.visual.md`.

**C6** — Trabalho mostra teclado; sono olhos fechados; movimento reduzido e janela escondida desativam animações.
Proof: inspeção visual/funcional nativa registrada em `.checks/multifunction.visual.md`.

**C7** — Sons opcionais são desligados por padrão e preferência persiste.
Proof: teste nomeado de preferências e prova nativa, a registrar antes de implementar a persistência.

### S2 — Sessões e intervenções · ~110 KB existentes · ~28k de leitura

**C8** — Presença, projeto, identidade e atividade permanecem reais e pedidos se resolvem somente por contexto exato.
Proof: provas existentes `source-access`, `session-discovery`, `interventions-codex`, repetidas sobre a entrega final.

**C9** — Controles oficiais de pausa/parada revalidam a sessão e exigem confirmação.
Proof: teste nomeado do executor e prova em sessão própria, a registrar antes desta implementação.

### S3 — Contas, quotas e continuidade · ~30 KB existentes · ~8k de leitura

**C10** — Contas Claude existentes e login em perfil isolado não sobrescrevem credenciais de outra conta.
Proof: teste nomeado de perfis e login oficial em perfil próprio.

**C11** — Quotas 5h/semanal associadas à conta, com indisponibilidade/TTL explícitos.
Proof: teste nomeado de amostras Claude e consulta real da conta isolada.

**C12** — Limiares 50/60/70/80/90 editáveis/desativáveis e novos limiares; aviso único por conta/janela, saltos agrupados.
Proof: `cargo test --manifest-path src-tauri/Cargo.toml quota_policy`

**C13** — Cadeia por conta usa percentual independente por janela e não transfere no meio do turno, com dados antigos ou sem alternativa.
Proof: `cargo test --manifest-path src-tauri/Cargo.toml routing`

**C14** — Cada transferência tem resumo revisável e aprovação única; mudança de cobrança identificada exige confirmação.
Proof: `cargo test --manifest-path src-tauri/Cargo.toml handoff_rejects_billing_identity_and_turn_drift_without_consuming_review`; `handoff_approval_is_single_use_and_persists_across_restart`; `handoff_failure_restores_edited_summary_with_fresh_required_approval`; `handoff_cancellation_suppresses_only_the_same_source_boundary`; prova própria pelo fluxo Tauri ainda pendente.

**C15** — Resumo cita planos/.md, decisões, alterações, testes e próximos passos.
Proof: `cargo test --manifest-path src-tauri/Cargo.toml handoff_summary_cites_actual_guides_decisions_commands_and_all_sections` gera resumo com histórico fixture; `handoff_links_actual_test_results_and_read_rules_by_tool_id` verifica resultados/regras reais; `handoff_large_history_keeps_latest_turn_and_reports_partial` verifica histórico grande; revisão de resumo de sessão própria ainda pendente.

**C16** — Troca manual de modelo/conta/IA usa operação oficial suportada ou nova sessão com resumo.
Proof: prova em sessão própria de cada operação disponibilizada.

### S4 — Iniciar, chat e origem · ~15 KB de fontes existentes · ~4k de leitura

**C17** — Tarefa escolhe pasta/provedor/conta/modelo/instrução e abre CLI externo padrão ou terminal integrado.
Proof: teste nomeado de lançamento e prova própria dos dois terminais.

**C18** — Chat próprio oferece CLI/assinatura compatível e API com cobrança identificada; falhas não criam cobrança alternativa silenciosa.
Proof: `chat_cli_request_uses_exact_session_and_subscription_env`; `chat_api_contracts_pin_provider_endpoint_no_fallback`; `chat_send_is_single_flight_revision_and_result_success`; `chat_transfer_requires_revision_identity_billing_single_nonce`; prova própria de ambos os fluxos ainda pendente.

Evidência parcial em 2026-10-06: `cargo test --manifest-path src-tauri/Cargo.toml chat_ -- --nocapture` passou em seis testes. Cofre Windows roundtrip com chaves fictícias, metadados sem chave/corrupção preservada, contratos de payload/endpoint, interpretação de conclusão/parcial/recusa e histórico com versão/nonce foram exercitados. Não prova envio HTTPS real, CLI, comandos Tauri, UI, troca de cobrança nem transferência. C18 permanece aberto.

Continuação: `chat_cli_request_uses_exact_session_and_subscription_env`, `chat_cli_completion_rejects_exit_alone_error_and_wrong_session` e `chat_cli_native_pipe_delivers_utf8_literal_and_kills_own_timeout` passaram na suíte completa de 78 testes. O último executa processos PowerShell próprios, sem IA/consumo, e prova UTF-8 longo e encerramento por prazo. Comandos/UI foram ligados; uso do Claude real e transferências continuam sem prova. A revisão independente final permanece pendente.

Prova adicional real: `chat_cli_live_subscription_keeps_exact_history_across_model_change -- --ignored --nocapture` passou com assinatura, UUID preservado e contexto recuperado após Sonnet → Haiku. `node scripts/verify-pet-native.mjs --live-chat` passou em 21 checks, incluindo turno CLI pelo Tauri, mascote working/conclusão finita e seleção do chat exato. `chat_presence_completion_is_ephemeral_exact_and_never_inferred_from_history` passou para token de memória/TTL e ausência de comemoração por histórico persistido. API real e transferência revisada continuam pendentes; C18 e o objetivo completo permanecem abertos.

Transferência de chat em 2026-10-06: `chat_transfer_requires_revision_identity_billing_single_nonce`, `chat_transfer_journal_recovers_commit_without_repeating_provider_call` e `chat_transfer_in_progress_does_not_drop_another_conversation_result` passaram. O journal recupera gravação parcial e commit completo sem repetir envio; revisão substituída/versão alterada/identidade e cobrança divergentes são rejeitadas. `node scripts/verify-pet-native.mjs --live-chat --live-transfer` passou inicialmente em 26 checks com dois turnos reais próprios por assinatura: formulário editou o resumo, backend rejeitou consentimento ausente, criou novo UUID em Haiku e rejeitou replay sem terceiro destino. Artefatos em `scratch/capy-visual-1d10cf42-4dd8-449b-83eb-4b9ca7b81a23/report.json`. A build seguinte acrescenta aceno/badge por nonce de revisão, limite de 64 revisões e serialização com outros resultados; suíte atual: 82 Rust aprovados e 1 prova live opt-in não executada na suíte padrão, 28 JS aprovados. API real e outra conta/provedor continuam sem prova; o objetivo completo permanece aberto.

Continuação de ciclo de vida: `chat_process_job_kills_only_owned_tree_when_owner_is_terminated` passou com encerramento forçado do proprietário fixture, filho e neto encerrados e processo alheio com mesmo executável preservado. `chat_process_quotes_empty_quotes_and_trailing_slashes` e `chat_cli_native_pipe_delivers_utf8_literal_and_kills_own_timeout` passaram para argumentos, ambiente, pipes UTF-8 e prazo. `chat_cli_attempt_survives_interruption_without_assuming_result_or_resending` comprova tentativa persistida e estado incerto após reinício, preservando mensagem/nonce sem inferir sucesso ou reenviar. Suíte: 86 Rust aprovados, um opt-in ignorado; 28 JS e frontend build aprovados. Recuperação revisada do estado incerto ainda não foi implementada. Build desktop e prova CLI real deste launcher estão em andamento.

**C19** — Abrir/trocar conversa original preserva identidade, sem abrir sessão errada.
Proof: testes source_access existentes e prova própria das novas origens.

Atualização da prova do launcher: desktop build passou; `node scripts/verify-pet-native.mjs --live-chat --live-transfer` passou nos 29 checks em `scratch/capy-visual-f58c027c-3270-49a8-bb23-2687d5e49ed8/report.json`. Smoke nativo passou em 20 checks sem erros de frontend e a build local foi reaberta. Somente Claude na mesma conta por assinatura foi exercitado; recuperação revisada, API real e demais lacunas permanecem abertas.

Recuperação revisada em implementação: `chat_recovery_requires_fresh_approval_identity_and_keeps_uncertainty_without_send`, `chat_recovery_cli_without_history_never_recreates_same_session`, `chat_recovery_cli_checks_exact_history_folder_and_live_process_before_resume` e `chat_presence_interruption_requests_review_and_never_celebrates_acknowledgement` passaram. Mensagem, nonce original e aprovação ficam registrados; confirmações não enviam nada e não geram conclusão. Fonte CLI sem proteção comprovada não é presumida encerrada. A conferência lê até 2 MiB de metadados do histórico para UUID/pasta, preservando o conteúdo integral para o CLI oficial. Suíte atual: 90 Rust aprovados, um opt-in ignorado; 29 JS e frontend build aprovados. Build desktop e `--live-recovery` estão pendentes; a simulação nativa perde o resultado na gravação do registro próprio após um turno real confirmado, não simula falha de rede ou timeout do provedor.

Atualização da recuperação: `chat_recovery_missing_history_blocks_resume_flag_from_an_earlier_turn` passou; suíte final deste trecho: 91 Rust aprovados e um opt-in ignorado, 29 JS aprovados. Desktop build, smoke de 20 checks e runner `--live-recovery --live-transfer` de 38 checks passaram. Contexto Claude por assinatura foi retomado no mesmo UUID com Haiku depois de revisão, preservando o aviso de resultado perdido; a transferência posterior criou outra conversa com aprovação própria. A falha de layout anterior foi corrigida mantendo as assertions. Evidências e limites em `.checks/multifunction.visual.md`; C18 completo e revisão independente ainda permanecem abertos.

Continuidade com avisos de origem: `chat_transfer_keeps_lost_results_after_success_and_edited_summary` e `chat_transfer_journal_rejects_changes_outside_transfer_and_preserves_files` passaram; a aprovação mantém todas as anotações e acrescenta o aviso ao contexto mesmo quando o resumo editável o remove. `chat_recovery_capacity_retains_every_nonce_and_allows_full_history_transfer`, `chat_nonce_capacity_reserves_recovery_transfer_and_rejects_overflow_without_pruning` e `chat_history_preserves_unknown_metadata_and_rejects_duplicate_interruption_receipts` passaram para capacidade, replay, reserva de saída e preservação de registros incompatíveis. Suíte: 96 Rust aprovados e um opt-in ignorado, 30 JS aprovados. Builds frontend/desktop passaram; runner nativo `--live-recovery --live-transfer` passou em 40 checks. Evidência visual e limites em `.checks/multifunction.visual.md`. O escopo completo permanece aberto.

## Swept

- validation: limites percentuais, IDs/pastas/modelos/perfis validados nos adaptadores; C10–C18.
- failure modes: TTL/indisponibilidade, cadeia esgotada, terminal/login/IA indisponível; C9–C19.
- idempotency/retry: eventos de movimento/alertas deduplicados; transferências e turnos não reenviados; C1/C3/C4/C12/C14/C18.
- authorization: pedidos exatos existentes; login oficial; confirmação por transferência/parada/cobrança; C8/C9/C10/C14.
- concurrency/ordering: fim do turno antes de handoff; operações identificadas e revalidadas; C13/C14/C16.
- data lifecycle: preferências persistidas, quotas expiram, credenciais com provedor; C7/C10/C11.
- dependency failure: comunicar indisponibilidade e nunca inventar saldo/sucesso; C11/C13/C16–C19.
- state transitions: movimento com relógio, conclusão explícita, handoff revisado; C1–C4/C13/C14.
- observability: erros existentes Tauri e estados explícitos na UI, sem registrar credenciais/prompts em logs.

## Handoff

Limites desenhados a partir da superfície, pois o plano veio de conversa. S1–S4 ~43.5k de leitura mínima; implementação sequencial no agente principal, dentro de 150k por lote. Verificador fresco após a última entrega. Provas ainda não existentes são lacunas abertas, não checks concluídos.
