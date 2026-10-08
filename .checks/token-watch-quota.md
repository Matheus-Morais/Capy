# Cotas — extensão Token-Watch

Fonte: `.design/token-watch-quota.md`, pedido e resposta do usuário de 2026-10-08.

| Obrigação | Landing | Prova |
|---|---|---|
| Comparar contas sem misturar identidades e janelas | src/quota-board.ts | groups each identity separately and compares windows within an account; escapes source labels and messages and marks low remaining balance |
| Saldo disponível, janela e renovação relativa/exata | src/quota-board.ts | groups each identity separately and compares windows within an account; countdown retains days and hours and never claims zero before renewal; verificador independente da data localizada |
| Não mostrar percentuais reais expirados, inválidos ou sem identidade | src/quota-board.ts | rejects stale, future, missing identity, reset and invalid balance without a meter |
| Reavaliar validade e countdown mesmo sem evento de backend | src/quota-board.ts | local timer drops expired balances without a backend event and cancels; verificador independente do minuto/foco/disclosure |
| Cotas em destaque no painel e resumo | panel.html, summary.html, src/style.css | native_quota_board_precedes_session_grid; native_summary_quota_board_has_all_demo_accounts; native_quota_real_board_at_minimum_panel_width_has_no_overflow; capturas 760/540/380 |
| Acesso a contas, alertas e continuidade | src/dashboard.ts | native_quota_accounts_shortcut_focuses_account_controls; native_quota_alerts_shortcut_focuses_preferences; native_quota_switch_shortcut_opens_and_focuses_destination |
| Antigravity: quatro janelas, identidade na observação, timestamp original, desabilitada distinta de zero | src-tauri/src/antigravity_quotas.rs | preserves_all_four_windows_and_identity; rejects_missing_identity_and_never_refreshes_observation_time; disabled_invalid_and_expired_are_not_zero_balances; missing_or_oversized_file_is_unavailable |
| Atualização em segundo plano limitada a um pedido a cada 15 s, sem rejuvenescer statusline | src-tauri/src/quota_refresh.rs, src-tauri/src/monitor.rs | coalesces_and_limits_refresh_requests; inspeção da chamada de leitura preservando mtime |
| Cadeia de alternativas Claude CLI e API OpenAI/Anthropic/Gemini; revisão automática ao fim do chat Claude, sem envio nem cobrança antes de aprovação | src/settings-ui.ts, src/presentation.ts, src-tauri/src/chat_routing.rs, src-tauri/src/monitor.rs | chat_routing_reviews_other_ai_without_send_and_requires_billing_consent; chat_routing_does_not_choose_an_unconfigured_api |
| Destino ausente não muda silenciosamente para primeira conta disponível | src/presentation.ts | unavailable saved fallback is preserved instead of selecting another account |
| Avisos reais e roteamento não aparecem sob o rótulo de simulação | src/presentation.ts, src/settings-ui.ts | real quota alerts and routing never appear as simulation and remain available in real mode; native_quota_real_alerts_do_not_leak_into_simulation |

Ainda abertas no pedido completo: Antigravity com amostra real recente (arquivo existente está vencido), leitura real Claude em contas conectadas, alertas reais e continuidade entre diferentes contas/IA. Não marcar essas obrigações aceitas por provas de interface.

Resultado local independente: 11/11 PASS em `.checks/token-watch-quota.verified.md`, fonte `d843983..2200140`. 43 JS, 140 Rust/2 live ignorados, build, 36 checks nativos de cotas e 37 de revisão API sintética. Pedido completo PARTIAL: prova Codex fresca anterior foi distinguida da consulta final indisponível; demais provas reais acima UNPROVEN. A revisão visual resolveu a única correção material (aviso real na simulação), com verdict pass `ship` restrito a essa correção.
