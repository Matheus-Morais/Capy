# Cotas — extensão Token-Watch

Fonte: `.design/token-watch-quota.md`, pedido e resposta do usuário de 2026-10-08.

| Obrigação | Landing | Prova |
|---|---|---|
| Comparar contas sem misturar identidades e janelas | src/quota-board.ts | Teste de agrupamento, escaping e duas contas |
| Saldo disponível, janela e renovação relativa/exata | src/quota-board.ts | Teste de saldo, duração e countdown |
| Não mostrar percentuais reais expirados, inválidos ou sem identidade | src/quota-board.ts | Testes de TTL, reset e dados inválidos |
| Reavaliar validade e countdown mesmo sem evento de backend | src/quota-board.ts | Teste de timer/cancelamento |
| Cotas em destaque no painel e resumo | panel.html, summary.html, src/style.css | Build e capturas largo/compacto |
| Acesso a contas, alertas e continuidade | src/dashboard.ts | Navegação/foco nas seções existentes |
| Antigravity: quatro janelas, identidade na observação, timestamp original, desabilitada distinta de zero | src-tauri/src/antigravity_quotas.rs | preserves_all_four_windows_and_identity; rejects_missing_identity_and_never_refreshes_observation_time; disabled_invalid_and_expired_are_not_zero_balances; missing_or_oversized_file_is_unavailable |
| Atualização em segundo plano limitada a um pedido a cada 15 s, sem rejuvenescer statusline | src-tauri/src/quota_refresh.rs, src-tauri/src/monitor.rs | coalesces_and_limits_refresh_requests; inspeção da chamada de leitura preservando mtime |
| Cadeia de alternativas Claude CLI e API OpenAI/Anthropic/Gemini; revisão automática ao fim do chat Claude, sem envio nem cobrança antes de aprovação | src/settings-ui.ts, src/presentation.ts, src-tauri/src/chat_routing.rs, src-tauri/src/monitor.rs | chat_routing_reviews_other_ai_without_send_and_requires_billing_consent; chat_routing_does_not_choose_an_unconfigured_api |
| Destino ausente não muda silenciosamente para primeira conta disponível | src/presentation.ts | unavailable saved fallback is preserved instead of selecting another account |

Ainda abertas no pedido completo: Antigravity com amostra real recente (arquivo existente está vencido), leitura real Claude em contas conectadas, alertas reais e continuidade entre diferentes contas/IA. Não marcar essas obrigações aceitas por provas de interface.
