# Cotas — documentação visual da extensão

Revisão independente em 2026-10-08, sobre `d843983..2200140`. Escopo: extensão comum da interface existente, conforme `.design/token-watch-quota.md` e o pedido confirmado de contas, cotas, alertas e continuidade entre conta/IA. Não houve composição aprovada à parte: a implementação parte do código e da identidade incumbente. Este registro documenta a superfície; não substitui o sistema global.

## Compatibilidade com o incumbente

O resultado mantém **The Oat-Sheet Companion**: Segoe UI, papel aveia, texto castanho, oliva para rotina e âmbar para atenção, controles discretos e separação por linhas finas. `src/style.css` acrescenta 37 linhas de estilos de cotas, usando as variáveis existentes de papel, texto secundário, divisória, oliva e âmbar. Os novos grupos não introduzem sombras, fundos de cartão ou outra família tipográfica. Os controles reutilizam borda quente, raio de 7px e foco âmbar do incumbente.

Os medidores circulares são uma decisão local desta extensão: anéis por `conic-gradient`, com recorte do mesmo papel e números tabulares, sem novos arquivos de imagem distribuídos. A arte SVG do mascote permanece fora do diff revisado. A nova composição prioriza cotas por conta antes da grade de sessões; isso está registrado no brief da superfície e corresponde ao pedido, sem promover essa ordem a regra para todo o produto.

## Composição e comportamento

- Painel de 760px: duas colunas de contas, separadas por divisórias; cabeçalho preserva título e indicação da fonte. Cada conta identifica provedor, conta e estado; cada janela mostra período, saldo disponível e renovação relativa. Fonte, horário e renovação exata ficam em uma divulgação nativa.
- Resumo de 380px: conta/estado à esquerda e janelas à direita; anéis de 56px e números de 18px substituem os 74px/21px do painel. O texto introdutório é omitido. O acesso ao painel e o rodapé fixo continuam visíveis. O countdown de 2h 0min quebra em duas linhas na captura, mantendo o valor legível.
- Painel mínimo de 540px: uma coluna de contas; ações quebram por linha e o botão de atualização fica na segunda linha. Não há corte horizontal nas capturas examinadas. Rolagem vertical é necessária nas três larguras; sessões e contas adicionais continuam abaixo do primeiro viewport.
- Estado real: o botão de atualização aparece e a indicação da seção passa a “Fonte e atualização”. Saldo não confirmado usa anel neutro, traço e texto “Indisponível”, em vez de percentual fictício. Código inspecionado também remove saldo/reset após expiração ou invalidade; reduzido a 20% ou menos recebe âmbar. Esses estados adicionais são sustentados pelo código/testes, não todos por uma captura visual.
- Contas, alertas e troca de conta/IA acessam as seções existentes. O relatório nativo registra foco nos controles de destino e expansão da seção de chat. O resumo abre o painel completo. Não se acrescentou um fluxo visual novo para aprovação de transferência neste trabalho.

## Evidência examinada e proveniência

Leitura direta: `PRODUCT.md`, `DESIGN.md`, `.impeccable/design.json`, `.impeccable/surfaces/src-style-css.md`, `.design/token-watch-quota.md`, `src/quota-board.ts`, `src/style.css`, `panel.html`, `summary.html`; diff da extensão e navegação em `src/dashboard.ts`; `.checks/token-watch-quota.md` e `.checks/token-watch-quota.verified.md`.

Inspeção visual direta das quatro capturas finais em `scratch/capy-visual-48d99913-aae1-4401-a770-cfc579441fc7/`:

| Captura | Superfície e evidência visível |
|---|---|
| `panel.png` | Painel demo, 760×680: grupos Claude pessoal/trabalho e Codex; cotas antes das sessões; seis anéis previstos pelo relatório; rótulo explícito de simulação. |
| `summary.png` | Resumo demo, 380×650 capturados: duas contas aparecem no viewport, terceira abaixo; versão compacta, rodapé e nenhum alerta real sob rótulo simulado. |
| `quota-real.png` | Painel real, 760×680: Codex/Claude sem conta confirmada e Antigravity identificado; saldos indisponíveis e nenhum exemplo de conta pessoal/trabalho. |
| `quota-real-compact.png` | Painel real, 540×680: conta em coluna única e navegação quebrada; identidade e estado mantidos. |

O runner existente `scripts/verify-pet-native.mjs` captura PNG via `Page.captureScreenshot` de alvos `tauri.localhost` das WebViews nativas, com `captureBeyondViewport:false`. A largura mínima de 540px foi aplicada por emulação de métricas à WebView do painel; isso prova layout, não um redimensionamento físico da janela. `report.json` desse diretório registra sucesso para ordenação, agrupamento, countdown, foco dos três atalhos, ausência de overflow no painel/resumo e na largura mínima, troca real/demo e isolamento dos alertas. Este documentador leu artefatos existentes; não abriu browser, executou runner, acessou provedores ou alterou processos.

## Drift e limites de validação

Há drift anterior à extensão: `PRODUCT.md` ainda descreve partes como pendentes e uma exploração inicial; `DESIGN.md` conserva composição de cotas na lateral e dimensões iniciais do resumo, enquanto o brief atual já registra cotas antes das sessões. O sidecar também conserva narrativa integralmente sintética e exemplos do medidor linear antigo. Não se corrigiu esse drift sem autorização para atualização global. O detector executado pelo fluxo principal uma vez trouxe avisos preexistentes de cores/fontes e headings HTML a 16px por não resolver a importação de CSS via dashboard; as capturas finais mostram a tipografia real carregada. Esses avisos não autorizam reescrever tokens incumbentes.

As capturas cobrem primeiro viewport, não configuração expandida, divulgação de fonte aberta, foco visível, conta longa ou revisão de transferência completa. O resumo capturado tem 650px de altura, diferente dos 620px iniciais documentados; não se presume prova visual no mínimo de altura. O snapshot real final não contém saldos recentes: a igualdade dos medidores com dados reais passou com zero medidores frescos. Portanto as capturas demonstram ausência/identidade/estado, sem aceitar leitura real recente Claude/Antigravity, disparo de alerta em outra conta ou envio/continuidade real entre provedores.

A revisão de finish aceitou a hierarquia incumbente dentro desses limites e apontou um defeito concreto: alerta real vazava para a demonstração. A correção `2200140` está comprovada pelo teste, check nativo real→demo e resumo recapturado. A preservação de alertas em modo real está sustentada por código e teste unitário, não por alerta visível nas capturas reais finais indisponíveis. O veredito final é ship apenas para essa correção, sem regressões introduzidas encontradas. Não equivale a aprovação visual de todos os fluxos nem aceitação do pedido completo; o verificador independente registra as 11 obrigações locais como PASS e o pedido completo como PARTIAL.

`DESIGN.md`, `.impeccable/design.json` e demais arquivos de sistema foram preservados. Única escrita deste documentador: `.checks/token-watch-quota.design.md`.
