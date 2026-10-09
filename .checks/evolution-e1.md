# Evolução E1 — base reconciliada

Sources: `PLANO_EVOLUCAO.md`, E1; conversa de 09/10/2026 (seguir, com subagentes sol); código e relatórios locais como evidência do estado atual.

Profile: light. Escopo desta entrega: reconciliação documental e inventário de artefatos. Não demonstra novamente comportamento nativo nem fecha E2–E8.

## Out of scope

Implementar integrações, executar sessões de IA, modificar configurações do usuário, substituir pacotes históricos, publicar ou declarar uma nova release validada.

## Landing

Reutilizar provas existentes, citando sua amostra/versão. Acrescentar `CAPACIDADES.md` e `ENTREGA_ATUAL.md`; reconciliar README, topo de NEXT_STEPS e estado E1 do plano. Inventariar executáveis com PowerShell sem executá-los.

Nenhuma decisão irreversível. Proveniência desconhecida do binário será registrada literalmente como desconhecida, nunca inferida de timestamp, versão ou HEAD atual. Seleção de candidato não equivale a validação de release.

## Checks

### S1 — Saber o que existe e qual entrega usar · documentação · menos de 100 KB de saída

**C1** — A matriz cobre Claude Code, Codex, Antigravity CLI e Antigravity IDE nas oito dimensões: descoberta, atividade, aprovações, perguntas, origem, cotas, chat e tarefas; cada estado remete ao código/prova ou identifica ausência/limitação.
Proof: revisão independente de `CAPACIDADES.md` contra código e fontes citadas, com localização por linha de cada grupo.

**C2** — README descreve respostas Codex e chat/tarefas próprios já implementados; diferencia cotas Antigravity comprovadas, CLI experimental e IDE não comprovada; mantém API real e contas alternativas como lacunas.
Proof: revisão independente das passagens README versus matriz e fontes, incluindo busca de afirmações antigas contraditórias.

**C3** — O inventário registra todos os executáveis encontrados na raiz, releases e target/release/capy.exe, com caminho relativo, tamanho e SHA-256 recalculáveis; proveniência desconhecida não recebe commit de build inventado.
Proof: `Get-FileHash -Algorithm SHA256` e `Get-Item` repetidos independentemente nos caminhos de `ENTREGA_ATUAL.md`.

**C4** — Um único candidato local é indicado para próxima validação; pacotes históricos e cópia da raiz não são apresentados como equivalentes sem comparar hashes; HEAD/alterações locais da inspeção são distintos da origem do binário.
Proof: revisão independente de `ENTREGA_ATUAL.md` e `rtk git status --short` / `rtk git log -1 --format=%H`.

**C5** — NEXT_STEPS e PLANO_EVOLUCAO apontam para os documentos reconciliados; E1 não fecha prova de correspondência fonte/binário se ainda não existir, e E2–E8 continuam planejados.
Proof: revisão independente das linhas atualizadas e links relativos via `Test-Path`.

### S2 — Associar o candidato às fontes atuais

**C6** — O candidato de trabalho é recompilado a partir de fontes inventariadas por SHA-256, com comando, código de saída e hash final registrados; os hashes de entrada continuam iguais após a build.
Proof: `npm run desktop:build` com exit 0, inventário de entradas antes/depois e `Get-FileHash` final, registrados em `.checks/evolution-e1-build.json`. Verificador recalcula os hashes e confere o log da compilação. Isso prova proveniência de build, não validação funcional nativa.

Adição antes da compilação: o inventário inicial revelou que a origem do executável atual não é demonstrável. Recompilar o candidato é necessário para fechar o resultado fonte/artefato já exigido por E1; os pacotes históricos não serão substituídos. Nenhuma instância Capy estava aberta na inspeção.

## Swept

- validation: C1–C5; sem nova lógica de produto.
- failure modes: C3/C4, origem desconhecida explícita.
- idempotency and retry: não se aplica; sem chamadas externas ou envios.
- authorization: somente documentos e leituras locais autorizados.
- concurrency and ordering: autores em arquivos separados; inventário situado no instante da leitura.
- data lifecycle: sem apagar ou substituir artefatos existentes.
- external-dependency failure: provas externas históricas não reexecutadas, limitações C1/C2.
- state transitions: classificação documental não altera estados do aplicativo.
- observability: C3/C4, caminho/hash/HEAD e limites da evidência.

## Handoff

Uma fatia documental e uma prova de proveniência, estimativa de leitura menor que 50k tokens, abaixo de 150k. Dois autores com superfícies separadas: matriz/README e inventário. Orquestrador integra retomada/plano e registra build; verificador fresco revisa C1–C6. Sem testes novos de aplicação: nenhuma lógica de execução mudou por esta entrega.
