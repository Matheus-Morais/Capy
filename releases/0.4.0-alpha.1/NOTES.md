# Capy 0.4.0-alpha.1

Prévia local das respostas a pedidos do Codex. A inscrição é opcional e iniciada pelo botão de cada cartão Codex. A versão suporta perguntas de usuário e aprovações individuais de comando e arquivo pela conversa original; não abre conversas, executa terminal nem grava regras. Pedidos de rede gerenciada, permissões adicionais e formatos incompletos continuam na origem.

As respostas são enviadas uma única vez ao pedido que aparece no cartão. Identidades vencidas são recusadas; a confirmação visual só aparece depois de o Codex resolver o pedido. A lista de pedidos e as inscrições ficam apenas em memória. Campos `isSecret` usam entrada password. O cenário de demonstração permanece separado das sessões reais.

## Validação desta prévia

- 40 testes Rust e 13 testes JavaScript passaram.
- `npm run build` passou.
- `npm run verify:native` passou.
- O runner `scripts/verify-codex-interventions.mjs` passou em conversa própria no release Windows: pergunta entregue, resposta antiga recusada sem liberar a seguinte, aprovação de alteração negada e arquivo ausente.
- Verificação independente repetiu os testes, a prova real, o build de prova e o release sem harness; ver relatório `.checks/interventions-codex.round3.verified.md`.
- O runner exige a build de prova `npm run desktop:build -- --features intervention-proof`. Essa feature fica fora do executável desta entrega.

Esta é uma prévia alpha, não a entrega 0.4.0 final. QA visual nativo do cartão real segue pendente porque a janela Capy não ficou acessível à automação. A conversa de produto indicada no checklist também não foi disponibilizada ao verificador. As demais etapas do produto seguem abertas. Uma conclusão de `serverRequest/resolved` pode indicar resolução na origem ou limpeza do pedido; a UI informa somente “Pedido resolvido no Codex”.
