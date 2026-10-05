# Atividade real do Codex

Continuação autorizada na conversa em 2026-10-05. O spike conectou um proxy do
daemon existente ao socket de controle por WebSocket e consultou duas threads:
uma `active`, outra `idle`. Nenhuma conversa foi iniciada ou retomada.

O Capy consulta somente threads já descobertas por presença e carregadas no
daemon. `thread/read` usa `includeTurns: false`; o resumo recebido é transitório,
e só identidade, projeto e estado são usados. Não há leitura de histórico de turnos,
assinatura de eventos de conversa ou resposta a pedidos.

Mapeamento: `active` com flags vazias = trabalhando; `waitingOnApproval` ou
`waitingOnUserInput` = precisa de você (responda no Codex); `idle` = ociosa;
`notLoaded`, `systemError`, flag desconhecida, formato inválido ou falha = desconhecido.
Ociosa não prova conclusão. Novo ciclo substitui integralmente o estado anterior,
eliminando pedidos que deixaram de existir. Falha de leitura elimina estados antigos.

Cada consulta usa um proxy efêmero do próprio executável do daemon, identificado
por PID e criação do processo, sem shell. Orçamento de 5 segundos para o lote,
até 64 sessões, mensagens até 1 MiB. A leitura ocorre fora da thread da interface.
O proxy é encerrado e aguardado ao fim de cada lote. Não se inicia outro daemon.

Claude, Antigravity, cotas, conteúdo de pedidos, respostas e terminais estão fora
desta entrega. A descoberta original continua retornando `unknown`; o monitor
enriquece apenas Codex com evidência runtime atual. Resumo e painel usam os mesmos
cards existentes, com novos rótulos e sem botões de resposta real.

Fontes: [App Server](https://learn.chatgpt.com/docs/app-server),
[status no código oficial](https://github.com/openai/codex/blob/main/codex-rs/app-server/src/thread_status.rs),
[tungstenite](https://docs.rs/tungstenite/latest/tungstenite/client/fn.client_with_config.html).
