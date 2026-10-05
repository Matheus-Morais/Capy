# Descoberta de sessões externas

> Status: primeira etapa autorizada pelo usuário em 2026-10-05; contratos de atividade e resposta ainda em investigação.

## Situation

Capy está em construção. A base desktop foi validada, inclusive o arraste pelo usuário. O usuário autorizou avançar para sessões reais. As fontes locais são detalhes de implementação dos agentes e podem mudar.

## Problem

A interface ainda apresenta apenas exemplos; o usuário precisa reconhecer quais conversas externas estão abertas e a que projeto pertencem antes de acompanhar intervenções.

## Success

Uma sessão externa aberta aparece uma vez com agente, projeto e identificador. Históricos encerrados não aparecem como sessões abertas. O usuário pode ocultar uma sessão e restaurá-la após reabrir o Capy.

## Boundary

Primeira etapa: descoberta local de Claude Code e Codex, atualização a cada cinco segundos, ocultação persistente e distinção de demonstração. Antigravity recebe diagnóstico explícito de integração pendente.

Atividade, pedidos, respostas, acesso ao terminal e cotas reais dependem de provas posteriores. Esta etapa não altera configurações nem retoma conversas dos agentes.

## Shape

O resumo mostra sessões com evidência de presença atual. Claude Code fornece registro de processo e identidade de criação; Codex mantém um bloqueio por conversa aberta. As integrações documentadas por hooks e app-server serão avaliadas para os próximos estados. Iniciar um app-server separado não demonstra o estado das sessões externas do usuário.

## Key decisions

1. **Histórico não é presença.** Um registro do Claude só entra se o processo ainda existe e sua criação corresponde ao registro; um Codex só entra se há bloqueio mantido e metadados correspondentes.
2. **Presença não é atividade.** Toda sessão descoberta nesta etapa tem estado `unknown`; não gera avisos de pedido, conclusão ou animação de trabalho.
3. **Identidade é agente mais identificador da conversa.** Duas conversas no mesmo projeto permanecem distintas; registros duplicados da mesma conversa são consolidados.
4. **Ocultar não interfere no agente.** A preferência pertence ao Capy e persiste entre aberturas, inclusive quando a sessão desaparece e reaparece.
5. **A demonstração é opcional e explícita.** O desktop inicia com descoberta real; exemplos e cotas simuladas só aparecem nos cenários de demonstração. O navegador permanece um preview simulado.

## Work

| Slice | Delivers | Status |
|---|---|---|
| Descoberta | Claude Code/Codex abertos no resumo e painel | clear |
| Atividade | Estados e pedidos de sessões externas | spike |
| Antigravity | Presença e projeto na CLI e IDE | spike |

### Descoberta

| State | What should happen |
|---|---|
| Registro com presença comprovada | Mostrar projeto, agente, identificador e estado desconhecido |
| Histórico ou processo reutilizado | Omitir da lista de sessões abertas |
| Fonte ausente | Informar ausência da fonte local |
| Fonte ilegível ou formato incompatível | Informar limitação; continuar lendo os demais agentes |
| Sessão oculta | Excluir dos totais e permitir restaurar |
| Nenhuma sessão | Mostrar vazio sem sugerir conclusão de tarefas |

IPC `demo_snapshot` mantém o contrato existente e acrescenta diagnósticos de descoberta; `demo_action` aceita cenário `real`, ocultar e restaurar. Um relatório local de diagnóstico permite verificar a descoberta sem iniciar a interface.

### Atividade

Codex: conexão de leitura ao daemon existente demonstrada; `.design/codex-activity.md` define o enriquecimento por estado runtime e a remoção de esperas antigas. A descoberta isolada permanece `unknown`. Hooks do Claude ainda exigem prova própria.

### Antigravity

Spike: verificar se os bloqueios de presença locais estão mantidos por uma sessão real e obter o projeto sem interpretar conversas protobuf. Diretórios de histórico encontrados não comprovam atividade. Parar após demonstrar presença/projeto ou registrar a lacuna.

## Sources

- PRODUCT.md e .design/capy.md: descoberta automática de sessões externas e opção de ocultar.
- [Codex app-server](https://learn.chatgpt.com/docs/app-server): histórico e estado de threads carregadas são capacidades distintas.
- [Claude hooks](https://code.claude.com/docs/en/hooks): eventos documentados para a etapa de atividade.
- [Antigravity hooks](https://antigravity.google/docs/hooks): variantes CLI/IDE e eventos de execução.
- [Rust File](https://doc.rust-lang.org/std/fs/struct.File.html): tentativa não bloqueante de bloqueio compartilhado.
- [Windows GetProcessTimes](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getprocesstimes): identidade de criação do processo.
