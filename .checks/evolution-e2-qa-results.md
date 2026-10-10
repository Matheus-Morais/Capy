# E2 — Resultados do QA adicional Claude

Rodada de 10/10/2026 para ampliar as amostras reais de Claude Code 2.1.296. Critérios mantidos em [evolution-e2-qa.md](evolution-e2-qa.md). Este registro acrescenta casos sem substituir as amostras históricas em [evolution-e2-live.md](evolution-e2-live.md). O QA alterou somente a política CSP para permitir os logos SVG `data:`; a build corrigida foi recompilada e retestada.

## Resultados observados

| Caso | Artefato | Resultado |
| --- | --- | --- |
| Aprovação `allow` | Debug `7ff2a235ee38bf01b67604d092193aa1e715f0c54012a8352a5fa03e82d866f0`, relatório [allow](evolution-e2-qa-allow.json) | PASS: hook nativo retornou allow; Claude registrou `permission_decision` accept/source hook; `probe.txt` contém exatamente `capy-proof`; houve continuação. Nenhuma decisão host. |
| Desconexão durante pedido pendente | Mesmo debug, [disconnect](evolution-e2-qa-disconnect.json) | PASS: hook encerrou com stdout/stderr vazios; fallback do host negou separadamente 8 ms depois; arquivo ausente, continuação e contexto expirado rejeitado pelo serviço substituto. Zero decisões do hook. |
| Endpoint ausente | Mesmo debug, [absent](evolution-e2-qa-absent.json) | PASS: hook encerrou silenciosamente em 53 ms; host fallback negou em seguida; arquivo ausente e continuação. Zero decisões do hook. |
| Timeout real | Mesmo debug, [timeout](evolution-e2-qa-timeout.json) | PASS: permissão pendente real expirou no limite de produção de 110 s; hook fechou silenciosamente após 110,073 ms, antes do fallback host negar. Arquivo ausente e agente continuou. |
| GUI nativa — negar e perguntar | Release `61b45972c37b1bac74b192e511fedd9e95d6c0bac5b655f860481c4ba3d32557`, [native-deny](evolution-e2-qa-native-deny.json) | PASS nos sete checks: sessão real descoberta, deny clicado, pergunta `Alpha` enviada, ambas as decisões atribuídas ao hook, continuação e arquivo negado ausente. Um pedido passivo do host foi observado; nenhuma resposta de controle host foi contabilizada. |
| GUI nativa — permitir e perguntar | Mesmo release, [native-allow](evolution-e2-qa-native-allow.json) | PASS nos sete checks funcionais: allow clicado, gravação exata, pergunta `Alpha`, decisões atribuídas ao hook e continuação. Porém, o relatório capturou três erros CSP bloqueando logos `data:image`; não é um resultado visual limpo. A política CSP foi corrigida após essa execução; o reteste seguinte da build corrigida, registrado abaixo, passou com zero erros observados. |
| GUI nativa — reteste após CSP | Release `d9c73054082741aafd2fb022334c3fbbdd82660773cf8f10d6bfac35f0ad3165`, [native-fixed](evolution-e2-qa-native-fixed.json) | PASS em 10 checks: logos Claude/Codex/Antigravity carregaram; `allow` por clique, gravação exata, pergunta `Alpha`, continuação, decisões via hook e zero exceções/erros observados após conexão CDP. |
| Smoke do release corrigido | [qa-smoke](evolution-e2-qa-smoke.json) | PASS em 20 checks; `uiErrors` vazio. |

As duas amostras de GUI usaram WebView2 nativo, seleção do modo real e cliques via CDP. O transporte de aprovação foi o hook; o host stdio permaneceu passivo. A amostra inicial de GUI terminou antes da execução de Claude por erro de inicialização/bridge (`uHVp2S`), com zero checks. A tentativa posterior acima passou os fluxos, mas os erros CSP permanecem registrados e não são apagados pelo resultado funcional.

## Isolamento e proveniência

O hash original `61b45972c37b1bac74b192e511fedd9e95d6c0bac5b655f860481c4ba3d32557` foi confirmado nos primeiros casos GUI. Após a correção CSP, a build release alpha.3 tem SHA-256 `d9c73054082741aafd2fb022334c3fbbdd82660773cf8f10d6bfac35f0ad3165` e 18.439.168 bytes; manifesto com 170 fontes, zero entradas alteradas durante a build. O reteste nativo acima passou e smoke passou em 20 checks. [evolution-e2-qa-final-environment.json](evolution-e2-qa-final-environment.json) registra marker ausente, settings iguais ao baseline capturado, zero alterações nas 170 fontes, nenhum processo de teste restante e instância normal reaberta pelo caminho release (PID 18604). No reteste GUI, o observador não viu exceções nem erros após conexão CDP; a observação não cobre a inicialização anterior.

Limitação: não foi capturado hash byte a byte de settings globais antes dos quatro primeiros casos; para esses casos, o registro inicial documenta timestamp anterior, inspeção dos runners e ausência de marker, sem equivaler isso a comparação de bytes. A linha de base de hash foi capturada antes dos casos finais e permaneceu igual depois. Consulte [evolution-e2-qa-environment.json](evolution-e2-qa-environment.json) e [revisão independente](evolution-e2-qa-review.md).

## Alcance que permanece aberto

As provas em build debug não certificam o release. A correção CSP e o fluxo GUI `allow` foram exercitados juntos no candidato corrigido. Na rodada posterior de [sessões](evolution-e2-sessions-results.md), duas sessões Write concorrentes e aprovação Write no terminal interativo real passaram. Ainda faltam perguntas/negação interativas, cancelamento/reinício reais, cobertura mais ampla de DPI/teclado e conclusão de E2 para Codex e Antigravity. O QA adicional não alterou os critérios E2 nem concluiu E2 integral. Nenhuma build nova foi empacotada ou publicada.
