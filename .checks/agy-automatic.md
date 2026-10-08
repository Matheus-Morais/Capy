# Coleta automática Antigravity

Fonte vinculante: pedido de 2026-10-08 — abrir `agy` em outro terminal para atualizar cotas deve ser desnecessário.
Contrato oficial: https://www.antigravity.google/docs/cli/headless/ e https://www.antigravity.google/docs/cli/commands/usage/. Instalado: `agy --print /usage --print-timeout 15s` confirmou quatro linhas tabulares e atualização do statusline identificado.

| Check | Landing | Prova |
|---|---|---|
| Consultar /usage automaticamente na primeira leitura real e cada 60 s; nenhum prompt ao modelo ou terminal visível | src-tauri/src/antigravity_quotas.rs, src-tauri/src/agy_usage.rs, src-tauri/src/monitor.rs | automatic_refresh_is_initial_periodic_and_forced; revisão argv fixo e CREATE_NO_WINDOW; agy_automatic_live_updates_identified_sample_without_terminal |
| Consulta em worker próprio, sem bloquear outros provedores/UI e sem duas consultas simultâneas | src-tauri/src/monitor.rs | inspeção worker com Cache único e loop sequencial; prova nativa agy-auto com controles de painel durante consulta |
| Atualizar fontes solicita todos os workers; limite manual continua 15 s | src-tauri/src/quota_refresh.rs, src-tauri/src/monitor.rs | coalesces_and_limits_refresh_requests; inspection revision independent of take |
| Prazo externo 30 s e saída 64 KiB, encerrar apenas processo/descendentes próprios | src-tauri/src/agy_usage.rs, src-tauri/src/chat_process.rs | agy_usage_rejects_model_output_and_malformed_reports; agy_usage_bounds_process_output_and_timeout; processo reutiliza job privado existente |
| Amostra continua com identidade e timestamp originais; falha de coleta não rejuvenesce saldo; sem export recente informar indisponibilidade | src-tauri/src/antigravity_quotas.rs | automatic_failure_keeps_original_observation_and_expires; rejects_missing_identity_and_never_refreshes_observation_time; agy_automatic_live_updates_identified_sample_without_terminal |

Limite: usa o login e o statusline já configurados no CLI. Não altera configuração/login, não escreve no arquivo exportado nem em Token-Watch e não executa tarefas de IA. Se o export não existir, erro explícito; sem associar texto tabular a conta presumida. Consulta oficial pode atualizar seu cache/statusline. Prova real deste pedido deve exigir janelas recentes identificadas, não sucesso com zero medidores.

Landing adicional: aguardar a finalização do export oficial dentro do prazo total, mantendo o job próprio vivo depois da saída do CLI. Encerrar imediatamente pode interromper seu statusline; associar texto à identidade antiga foi rejeitado.

Perfil: light; um lote, escopo localizado (~10k de leitura), sem handoff de build. Sweep: validação e falhas cobertas por relatório limitado e export identificado; retries a cada 60 s; autenticação existente; worker sequencial evita sobreposição; timestamps originais expiram em 120 s; indisponibilidade de dependência explícita; real/demo separadas; mensagens de erro sem stdout/credenciais. Test policy: decisões de intervalo e falha têm testes próprios; processo externo tem fixture de prazo/saída; fronteira real e UI exigem o opt-in e prova nativa.

Provas do autor: 144 Rust passaram/3 opt-ins ignorados; opt-in real executado explicitamente e passou (1 teste); 43 JS passaram; desktop:build passou. Prova nativa --agy-auto: 39 checks PASS em scratch/capy-visual-dac548fd-1281-49f2-821e-8e6a6c52424b/report.json; exigiu primeira e segunda amostras identificadas novas, sem botão/terminal manual, medidores reais e navegação responsiva. Tentativa inicial headless com encerramento imediato falhou; corrigida mantendo job até o export recente. Não contabilizar execução --exact sem nome completo (zero testes).
