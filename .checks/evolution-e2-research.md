# E2 — investigação preliminar de respostas Claude

09/10/2026. Pesquisa somente de leitura por subagente `gpt-6.1-sol`. Não implementa nem valida a integração Windows. E2 permanece planejada.

## Fontes oficiais consultadas

- [Hooks](https://code.claude.com/docs/en/hooks): contratos PermissionRequest e PreToolUse, timeout e saída do hook.
- [Entrada e aprovações no SDK](https://code.claude.com/docs/en/agent-sdk/user-input): AskUserQuestion, respostas e permission host.
- [Permissões no SDK](https://code.claude.com/docs/en/agent-sdk/permissions): canUseTool e cancelamento.

## Resultado da pesquisa

PermissionRequest possui resposta síncrona documentada com `hookSpecificOutput.hookEventName = PermissionRequest` e `decision.behavior = allow | deny`. Recebe sessão, pasta, ferramenta e entrada; não recebe `tool_use_id`. A própria invocação viva deve identificar a resposta. Não cobre aprovação de rede de comando sandboxed.

A documentação atual permite responder AskUserQuestion por PreToolUse, retornando `permissionDecision: allow` e `updatedInput` com perguntas originais e respostas. Somente permitir não fornece uma resposta. Em modo print, um hook que fornece a resposta pode satisfazer o requisito de permission host; essa possibilidade ainda precisa ser exercitada na versão instalada.

No timeout, Claude cancela o hook e descarta a saída tardia; timeout não equivale a negação. A API SDK `canUseTool` é um caminho distinto, não prova a possibilidade de anexar um host a qualquer sessão externa aberta.

## Implicações para a implementação

- O observador atual desserializa metadados e sinaliza waiting; o script usa prazo de dois segundos. Ele não pode se transformar implicitamente em um canal de aprovação.
- Criar prova isolada de um canal síncrono separado, com nonce de uso único, sessão/projeto, PID/criação, prazo e invalidação quando a invocação termina.
- Correlacionar PermissionRequest pela invocação e PreToolUse também pelo identificador de ferramenta quando presente; nunca por nome da ferramenta ou último evento da sessão.
- Confirmar contratos da versão instalada antes da implementação. O transporte Windows e o mecanismo de resposta entre hook e UI ainda precisam ser definidos e provados.

## Próxima prova necessária

Duas sessões próprias simultâneas no Windows, aprovação e pergunta reais, tentativa de resposta cruzada rejeitada, cancelamento, timeout e confirmação na sessão exata. Exercitar print e interativo separadamente. Não tomar prova de SDK como prova de intervenção em sessão externa.
