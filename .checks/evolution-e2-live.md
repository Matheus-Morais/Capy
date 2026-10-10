# E2 Claude — amostras reais parciais

Atualizado em 09/10/2026 (UTC 10/10). C7 segue parcial: houve uma sessão print bem-sucedida com permission host passivo, mas aprovação positiva (`allow`), uso interativo/GUI, concorrência real, cancelamento e timeout real ainda não foram provados. Evidência versionada: [relatório](evolution-e2-live-report.json) e [eventos de pedido/resultado de ferramenta](evolution-e2-live-events.jsonl).

## Execução bem-sucedida

- Claude Code `2.1.296`, modo `print`, modelo Haiku; executável SHA-256 `7ff2a235ee38bf01b67604d092193aa1e715f0c54012a8352a5fa03e82d866f0`.
- Relatórios: `scratch/claude-capy-w96xYJ/report.json` e `events.jsonl`; sessão `9d5bc656-3849-4a3b-8d62-8d905d37ca73`.
- O runner marcou PASS: contexto alterado rejeitado; permissão exata respondida com `deny`; replay rejeitado; pergunta exata respondida com `Alpha`; Claude continuou mencionando a escolha; arquivo cuja escrita foi negada não foi criado.
- `--passive-permission-host` habilitou `--permission-prompt-tool stdio` para que `AskUserQuestion` estivesse disponível. O host recebeu um `can_use_tool` do pedido Write e **não enviou `control_response`**. A decisão deny veio do hook Capy; a resposta Alpha veio do canal de hook para AskUserQuestion.
- Essa sessão prova esses fluxos na combinação registrada. Não prova `allow`, modo interativo ou GUI, nem concorrência real, cancelamento ou timeout reais. O runner em print não demonstra experiência de uso pela interface nativa.

## Amostras anteriores preservadas

- `scratch/claude-capy-6Ylx7v/`: Claude Code 2.1.296 print sem permission host rejeitou contexto alterado, respondeu `deny` à permissão exata e rejeitou replay. AskUserQuestion não estava disponível; runner geral FAIL. Classificação: evidência negativa/parcial para aqueles critérios, não evidência de pergunta respondida.
- A execução anterior com permission host teve uma corrida no SDK. Ela permanece falha, sem valor de prova positiva.

Essas execuções têm condições distintas. A amostra mais recente não reclassifica a tentativa com AskUserQuestion indisponível nem a corrida anterior. Nenhuma delas valida o release atual ou conclui C7/E2.
