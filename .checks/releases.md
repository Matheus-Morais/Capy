# Entregas da Capy

Objetivo autorizado: executar todas as seis frentes e gerar uma nova versão por etapa. Publicação remota foi solicitada e depois dispensada pelo usuário; entregar executáveis locais aqui na conversa.

Base: 6f3bac3; versão inicial 0.1.0. O checkout não possui remoto. Em 2026-10-06, à pergunta sobre a URL, o usuário respondeu “Vdd, esquece”. Interpretado como dispensa da publicação remota (assunto da pergunta), mantendo as seis etapas; informado ao usuário que as versões serão locais.

## Versões

| Versão | Etapa | Critério de entrega | Estado |
| --- | --- | --- | --- |
| 0.2.0 | Antigravity | presença/projeto e atividade observada em sessões CLI/IDE; histórico não é presença | investigação |
| 0.3.0 | Origem correta | card acessa a sessão original identificada; não cria outra conversa por engano | pendente |
| 0.4.0 | Intervenções | pedido exato, resposta entregue, rejeição de pedido expirado; ações somente quando suportadas | pendente |
| 0.5.0 | Cotas | fonte real por conta/provedor, atualização e expiração, dados ausentes explícitos | pendente |
| 0.6.0 | Claude completo | perguntas, esperas longas e subagentes, sem atribuir evento à sessão errada | pendente |
| 0.7.0 | Desktop validado | cards/badge reais, ocultação/restauração, abertura/fechamento e várias sessões; provas visuais nativas | pendente |

Prévia intermediária preparada dentro da etapa 0.4.0: `0.4.0-alpha.2` implementa perguntas e aprovações Codex e animações expressivas por estado. A revisão independente verificou os testes, a prova C7, os builds e o smoke nativo; a etapa continua aberta para QA visual nativo dos cards reais.

Cada etapa terá checklist, provas, verificador independente, bump coerente npm/Cargo/Tauri, executável e notas de release com limitações reais. A versão só é concluída após conferência dos artefatos locais e entrega do link na conversa.

## Evidências e decisões

- Antigravity CLI 1.3.0 está instalado; fonte local tem `presence/<UUID>.lock` e `conversation_summaries.db` com `workspace_uris`, `parent_conversation_id` e `source`.
- SQLite será consultado em modo read-only somente por ID com bloqueio mantido; não consultar title, preview, raw_summary ou status salvo como prova de atividade.
- Hooks oficiais oferecem conversationId/workspacePaths. Não interpretar protobuf de conversa nem substituir cota real por percentual de demonstração.
- Nenhuma publicação ou etapa foi declarada concluída.
- Prévia local 0.2.0-alpha.1: implementação Antigravity em andamento e slice de acesso Codex. Não substitui 0.2.0/0.3.0 finais; provas CLI/IDE/seleção de conversa continuam pendentes. Notas em releases/0.2.0-alpha.1/NOTES.md.
- Prévia local 0.2.0-alpha.2: cotas reais da conta Codex conectada, atualização e expiração. Não encerra 0.5.0: Claude, Antigravity e múltiplas contas continuam pendentes. Notas em releases/0.2.0-alpha.2/NOTES.md.
