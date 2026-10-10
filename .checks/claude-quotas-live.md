# Claude — coleta real de cotas

10/10/2026, Claude Code 2.1.296, release local `d9c73054082741aafd2fb022334c3fbbdd82660773cf8f10d6bfac35f0ad3165`. Sem alteração de código/recompilação.

Conexão pelo comando nativo existente `connect_claude_quotas`, perfil `claude-default`. Backup de settings salvo localmente em `~/.claude/capy-quota-backups/`. Statusline original `node C:/Users/MOBILTEC/scripts/claude-statusline.js` preservada em `capy-quotas/bridge.json` e encadeada pelo wrapper. Comparação semântica dos JSONs confirmou que todos os outros settings permaneceram iguais. Primeira comparação textual foi inconclusiva por ordem das chaves; comparação de objetos Python passou.

Nova sessão própria interativa ConPTY, UUID `7f1ab5f1-04e1-483f-8e5e-95573004ec08`, prompt mínimo sem ferramentas. Startup foi observado (`started: true`); atualização real trouxe `used_percentage` 12 e 78 nas janelas `five_hour` e `seven_day`. Após `refresh_quotas`, snapshot nativo em modo real confirmou as duas linhas `fresh`, conta autenticada confirmada e mesmos valores. Screenshot do painel confirmou **88% disponíveis em 5 horas e 22% na semana**. Terminal preservou a statusline personalizada e retornou COTAS_OK.

Evidências locais em `scratch/claude-quota-live/`: snapshot.json, quotas.png, preservation.json, session/terminal.raw, session/conpty-provenance.json e launch.ps1. Snapshot/screenshot mantidos locais por conterem identidade da conta. Sessão própria encerrada por `/exit`, ConPTY exit 0; sessão externa preservada. App normal reaberto PID 33332, sem CDP. Conexão de cotas permanece habilitada conforme solicitado.

Limites: uma conta/duas janelas, sem troca real de login. O coletor exige startup observado; sessões anteriores à conexão precisam ser reiniciadas pelo usuário. Amostras expiram após 120 s sem atualização; números acima são a observação do teste, não uma promessa de saldo permanente. Nenhum payload foi fabricado ou reapresentado como coleta atual.
