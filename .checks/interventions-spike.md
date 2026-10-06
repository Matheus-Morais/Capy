# Intervenções — investigação de transporte

2026-10-06. Etapa 0.4.0 permanece aberta; nenhum controle real foi habilitado nesta investigação.

## Claude Code 2.1.291

Fonte oficial: https://code.claude.com/docs/en/hooks (Tools that require user interaction e PermissionRequest decision control).

`PreToolUse` pode receber `AskUserQuestion` e devolver `permissionDecision: allow` junto de `updatedInput`, preservando questions e acrescentando answers por texto da pergunta. PermissionRequest tem um contrato distinto de allow/deny; não salvar regras ou alterar modo de permissões. Defer só atende processos headless e não substitui o transporte para sessões interativas externas.

Prova executada: `node scripts/probe-claude-question-hook.mjs`, exit 0. Em uma sessão própria, hook de pergunta permaneceu vivo por 35 s; aos 31 s o runner conferiu PID vivo, session_id e tool_use_id. Após devolver Stop à pergunta, o resultado do Claude confirmou esse valor e o processo terminou normalmente. A prova não usa a Capy para responder, não alcança a UI desktop e não comprova permissões ou subagentes. Usa orçamento máximo de US$ 1 e somente AskUserQuestion; nenhum processo externo foi retomado.

Esse resultado estabelece um transporte viável para implementar coleta/resposta com identidade de callback, processo e sessão, e sustentar a espera enquanto o próprio hook estiver vivo. Não justifica prolongar estado antigo usando apenas presença do Claude.

## Codex

Fonte oficial: https://learn.chatgpt.com/docs/app-server (Approvals; Read a stored thread).

Primeira observação: conexão read-only já aberta e conexão nova não receberam a solicitação de pergunta durante janelas de 3 s; a conexão nova confirmou waitingOnUserInput via thread/read e a origem respondeu/completou. Isso é ausência de evento nessas condições, não prova de que uma conexão inscrita por thread/resume seja incapaz de responder.

Thread/resume em uma thread nova sem primeiro turno retornou no rollout found. Depois de um bootstrap próprio, a prova inscrita passou: um observador inscrito antes da pergunta recebeu o pedido, e outro inscrito após a pergunta também recebeu o pendente. A resposta pelo observador tardio levou a origem ao turno completed. Script: `node scripts/probe-codex-interventions.mjs`, exit 0; subscribedObserverReceived, resumedPendingObserverReceived, freshObserverSeesWaiting, responseThroughObserver e ownerResponseCompleted todos true. Nenhuma thread externa foi retomada. Não houve prova de aprovação/permissão, pedido expirado, alteração de conta ou UI Capy; estes ainda são gates da implementação.

## Próxima implementação

Usar somente contratos que a prova real sustentar. Codex precisa manter o cliente inscrito vivo: IDs de pedidos pertencem à conexão que os recebeu e não devem ser reaproveitados em um proxy transitório. Habilitação reversível, pedido único com validade, PID/criação e projeto revalidados antes de responder; rejeitar expirados/duplicados; fallback à origem quando não suportado. Nenhuma resposta automática ou alteração de política. As perguntas necessárias aparecem somente em UI local; não registrar histórico, credenciais ou respostas em logs.
