# E2 — revisão independente

09/10/2026. Verificador separado dos autores. Escopo: canal Claude, integração e configuração; alterações de produto preexistentes não foram julgadas como parte desta entrega. Nenhuma alteração de implementação feita pelo verificador.

## Resultado

Nenhum defeito substantivo confirmado de concessão indevida, troca de contexto, replay ou deadlock no código inspecionado. Isso não fecha E2: C7 permanece parcial. Revisão complementada com os testes adicionados e a amostra real `scratch/claude-capy-w96xYJ`; build e execução externa foram realizadas pelo integrador, sem compilação concorrente pelo verificador.

| Critério | Evidência e limite | Estado |
| --- | --- | --- |
| C1 | `protocol.rs:61,148,169,223`: valida evento/entrada/perguntas, decisões explícitas, uma única chave de resposta, respostas completas, seleção múltipla validada e preservação de perguntas/metadata. Quatro testes de protocolo examinados; execução de nove testes Rust direcionados relatada pelo integrador, não repetida aqui. Contrato externo fundamentado no registro `.checks/evolution-e2-research.md`, com prova real parcial descrita em C7. | Sustentado por inspeção e testes locais relatados |
| C2 | `service.rs:103-116`: comparação integral de Context e geração; remoção antes da tentativa de escrita. Teste `two_concurrent_wrong_context_replay` usa dois sockets reais, correlaciona sessões e saídas distintas e rejeita replay/contexto trocado. A versão revisada acrescenta alteração isolada de geração e confirma que o pedido correto permanece ativo. | Sustentado por inspeção e testes relatados |
| C3 | `service.rs:41-52,118-125`: reset encerra sockets inclusive cabeçalhos incompletos; loop remove pedidos desconectados/expirados/inválidos. `mod.rs:43-73` não escreve decisão nem stderr em falha, endpoint ausente, identidade inválida ou erro de leitura. Testes cobrem desconexão, timeout reduzido, reset, endpoint cleanup, revogação do validador enquanto pendente e stop com socket ativo. Hook real sem app, cancelamento e timeout nativos ainda não exercitados. | Sustentado localmente; processos reais parcialmente provados |
| C4 | Executei `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-claude-settings.ps1`: PASS. Fixture confirma opt-in, timeout120, matcher AskUserQuestion, observer2s, InstructionsLoaded10s, repetição idempotente, downgrade, remoção, preservação de hooks/configuração alheios e JSON inválido intacto. `claude-hooks.ps1:37-58` remove apenas comandos Capy com caminho exato informado. | PASS local |
| C5 | Executei `rtk node --test tests/claude-interventions.test.mjs tests/presentation.test.mjs`: 15/15 PASS. Escaping de ferramenta/entrada/cwd, identidade visível real, multiSelect/texto e semântica Codex Other cobertos. `dashboard.ts` obtém pedido corrente e contexto antes de enviar; backend faz validação independente. Build TypeScript/Vite PASS no registro de validação e na build desktop normal, cujo log/manifesto examinei. | PASS testes e build |
| C6 | `main.rs:462-466,491-498,868-884,951`: PID/criação, ancestral, presença sessão/pasta, modo real, sessão descoberta e preferência hidden reavaliados; registro separado, encaminhamento por geração, reset após liberar mutex demo e stop no encerramento. `monitor.rs:140-151` incorpora pedidos e marca waiting. `claude_activity.rs:355-376` compara PID, UUID, cwd e procStart do arquivo de presença. Os testes de serviço usam validador artificial; a amostra C7 exercita o helper nativo de identidade, com filtro próprio do probe. Smoke nativo 20/20 PASS cobre inicialização/janelas, sem pedidos Claude reais nem prova de visibilidade/preferences. | Sustentado por inspeção e amostra nativa parcial |
| C7 | Revisei runner, report e events da amostra `scratch/claude-capy-w96xYJ`: Claude 2.1.296 print, Write negado, contexto alterado/replay rejeitados, Alpha recebido por AskUserQuestion e continuação final. `tool_result_meta` registra `source:hook` para ambas as decisões; o controle de Write do host foi cancelado. No branch passivo, o runner retorna sem escrever control_response. Prova usa helper de identidade de produção, mas validador de sessão/pasta próprio do probe; visibilidade/preferences/UI de produção permanecem por inspeção. Não prova allow, interativo, GUI, concorrência real, cancelamento ou timeout reais. Amostras FAIL `yqZiYK` (corrida host) e `6Ylx7v` (pergunta indisponível em pure-print) não foram reclassificadas. | PARCIAL — E2 aberta |
| C8 | Revisão final de README, CAPACIDADES, NEXT_STEPS, PLANO_EVOLUCAO e `.checks/evolution-e2-live.md`: canal explicitamente experimental; documentam Enable -Responses, timeout120 versus observer2s, reinício, downgrade/remoção e retorno à origem em falha. Amostra print com host passivo e duas tentativas anteriores malsucedidas são diferenciadas. Não afirmam allow, interativo/GUI, concorrência/cancelamento/timeout reais nem validação do release atual. A tabela do plano rotulada Estado inicial é histórica. | PASS documentação |

## Observação não classificada como bug

`service.rs:46-52,226-232` chama o validador sob o mutex global, aproximadamente a cada 20 ms por conexão. O validador de produção (`main.rs:868-884`) faz enumeração de processos e descoberta em disco. A ordem de locks examinada não mantém preferences ao readquirir demo; reset Claude ocorre depois de liberar demo. Não encontrei deadlock. O custo com muitas sessões ainda não foi medido; não se deve inferir capacidade interativa para 64 pedidos apenas do limite de memória.

## Distinção de provas

- Execuções independentes nesta revisão: fixture PowerShell e 15 testes JS, ambos com exit code 0.
- Inspeção independente: protocolo, transporte/cleanup/locks, encaminhamento, identidade, UI e preservação de configuração.
- Evidência delegada ao integrador: execução Rust, builds e prova Claude real.
- Ausência de prova não foi registrada como defeito confirmado.

## Complemento de evidência

Integrador relatou 9 testes Rust direcionados, 47 JS e build frontend PASS. Inspecionei as assertions novas de geração, revogação e stop; não repeti compilação. Amostra real executada pelo integrador e auditada aqui nos arquivos preservados: sessão `9d5bc656-3849-4a3b-8d62-8d905d37ca73`, SHA-256 `7ff2a235ee38bf01b67604d092193aa1e715f0c54012a8352a5fa03e82d866f0`. Esse executável de prova não representa automaticamente o release normal.

O runner limita ferramentas a Write/AskUserQuestion, orçamento a US$1 e processos aos próprios filhos; respostas são enviadas ao probe, com contexto integral. O marker temporário exige opção explícita e só é removido se identidade/conteúdo não mudaram. Diretório da prova e endpoint são próprios. Nenhum defeito substantivo adicional confirmado nessa inspeção. A execução passiva comprovada não emite decisão pelo host SDK; isso não altera a classificação da tentativa anterior com host ativo.

Revisão final de documentação concluída após atualização dos autores. C8 PASS; C7 permanece PARCIAL. Release normal e smoke foram examinados no complemento abaixo; permanecem distintos do executável de prova.

## Candidato normal e regressão final

Examinei `.checks/evolution-e2-build.json`, o final do log desktop, `.checks/evolution-e2-native-smoke.json`, `.checks/evolution-e2-validation.md` e ENTREGA_ATUAL. Manifesto: 170 entradas, zero changedInputs, exitCode 0. Conferi diretamente o SHA-256 do executável normal: `61b45972c37b1bac74b192e511fedd9e95d6c0bac5b655f860481c4ba3d32557`, igual ao manifesto e à documentação. Log confirma release compilado com seis warnings. Smoke contém 20 checks aprovados, passed true e uiErrors vazio. Isso sustenta build/contrato TypeScript e inicialização nativa, sem demonstrar intervenções Claude pela GUI.

Registro do integrador: regressão Rust completa 154 PASS e 3 live opt-in ignorados, JS completo 47 PASS. Não repeti essas execuções; a distinção de autoria da prova foi preservada. Não houve redução de critérios para converter amostras FAIL em PASS: C7 continua parcial, e os casos nativos ausentes continuam explicitamente abertos. A amostra de hook usa outro hash/build com feature de prova. Candidato normal recompilado não implica instalador, publicação nem E2 concluída.
