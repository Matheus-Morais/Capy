# Capy 0.4.0-alpha.2

Prévia local com respostas individuais a pedidos suportados do Codex e animação mais expressiva da mascote. Quando uma sessão aguarda resposta, um sinal de atenção aparece junto à capivara e pulsa em três batidas suaves; os demais estados mantêm seus movimentos próprios: respiração e piscada em repouso, digitação durante trabalho, salto curto ao concluir e respiração com “Z” ao dormir. “Reduzir movimento” do Windows e a opção interna continuam desativando as animações.

As respostas Codex permanecem vinculadas ao pedido original e exigem conexão explícita no cartão. Esta versão não abre conversas, executa terminal nem grava regras. Claude e Antigravity ainda não têm esses controles.

## Validação

- 40 testes Rust e 13 testes JavaScript passaram na revisão da implementação Codex; os testes JavaScript atuais também incluem a checagem da inscrição por clique.
- `npm run build` passou após a alteração de animação.
- A arte foi conferida na prévia web no estado “aguardando resposta”.
- O smoke nativo e a prova real Codex passaram na revisão independente anterior; o binário alpha.2 será gerado sem a feature de harness.
- Os cartões de intervenção ainda precisam de QA visual na janela nativa. Veja `.checks/interventions-codex.round3.verified.md`.

Esta é uma prévia alpha, não a conclusão da etapa 0.4.0. A prova de resposta real no Codex passou, mas falta a inspeção visual nativa dos cartões antes do release final.
