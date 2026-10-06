# Entregas da Capy

Objetivo autorizado: executar todas as seis frentes, gerar uma nova versão por etapa e publicar uma release no repositório remoto indicado pelo usuário.

Base: 6f3bac3; versão inicial 0.1.0. O checkout não possui remoto. URL solicitada ao usuário em 2026-10-06; publicação aguarda resposta, implementação pode avançar.

## Versões

| Versão | Etapa | Critério de entrega | Estado |
| --- | --- | --- | --- |
| 0.2.0 | Antigravity | presença/projeto e atividade observada em sessões CLI/IDE; histórico não é presença | investigação |
| 0.3.0 | Origem correta | card acessa a sessão original identificada; não cria outra conversa por engano | pendente |
| 0.4.0 | Intervenções | pedido exato, resposta entregue, rejeição de pedido expirado; ações somente quando suportadas | pendente |
| 0.5.0 | Cotas | fonte real por conta/provedor, atualização e expiração, dados ausentes explícitos | pendente |
| 0.6.0 | Claude completo | perguntas, esperas longas e subagentes, sem atribuir evento à sessão errada | pendente |
| 0.7.0 | Desktop validado | cards/badge reais, ocultação/restauração, abertura/fechamento e várias sessões; provas visuais nativas | pendente |

Cada etapa terá checklist, provas, verificador independente, bump coerente npm/Cargo/Tauri, executável e notas de release com limitações reais. Release só é concluída após publicação e conferência dos artefatos remotos.

## Evidências e decisões

- Antigravity CLI 1.3.0 está instalado; fonte local tem `presence/<UUID>.lock` e `conversation_summaries.db` com `workspace_uris`, `parent_conversation_id` e `source`.
- SQLite será consultado em modo read-only somente por ID com bloqueio mantido; não consultar title, preview, raw_summary ou status salvo como prova de atividade.
- Hooks oficiais oferecem conversationId/workspacePaths. Não interpretar protobuf de conversa nem substituir cota real por percentual de demonstração.
- Nenhuma publicação ou etapa foi declarada concluída.
