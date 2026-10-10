# E2 — QA adicional solicitado pelo usuário

Escopo: testar candidato release atual e completar amostras próprias do canal Claude. Não alterar sessões/conversas externas do usuário, login ou settings globais. Reutilizar critérios E2; nenhum critério anterior é removido.

| Check | Prova exigida |
| --- | --- |
| Q1 — Artefato | SHA-256 release igual a ENTREGA_ATUAL antes/depois; fontes ainda iguais ao manifesto |
| Q2 — Aprovação real | Hook permite Write de `probe.txt` próprio, arquivo contém `capy-proof`, resultado de ferramenta identifica hook e Claude continua |
| Q3 — Desconexão/ausência | Pedido próprio perde canal sem aprovação; eventual decisão do host fallback explicitamente separada; cliente sem app encerra silenciosamente, sem JSON de aprovação |
| Q4 — Interface real | Build release normal exibe pedido da sessão própria via descoberta/monitor; clicar negar e responder Alpha entrega pelo hook, Claude continua, arquivo negado ausente; capturar telas/erros |
| Q5 — Isolamento | Settings globais intactos, marker temporário restaurado, processos próprios de teste encerrados; reabrir instância inicialmente aberta após teste exclusivo |
| Q6 — Revisão | Verificador independente examina resultados, falhas e limites; sem PASS por fixture para critérios reais |
| Q7 — Logos locais | Após defeito observado, logos `data:image/svg+xml` carregam na build nativa e não há erros CSP/JS no período observado; manter CSP de scripts/conexões. Fonte: [CSP oficial Tauri](https://v2.tauri.app/security/csp/) inclui `data:` para imagens. |

Runners novos sob responsabilidades separadas; execução com Claude e marker compartilhado sequencial pelo orquestrador. Modo interativo terminal, duas sessões reais simultâneas e prazo real de 110 s não se tornam aprovados por essa rodada sem execução correspondente. Testes existentes só serão repetidos se uma correção de produção exigir. Decisão Q7: permitir `data:` somente em img-src para os SVGs já presentes; evitar reescrever a arte ou desativar CSP.
