# Prova nativa parcial da Capy multifuncional

## Preservação de configuração das quotas — 2026-10-07

Revisão independente da preservação aprovou 108 Rust/um opt-in ignorado, 32 JS e 26 checks nativos em `.checks/quota-settings.verified.md`. Extensão posterior: teste local `statusline wrapper preserves Unicode in the existing command` falhou com mojibake e passou após explicitar UTF-8 no Get-Content do bridge. Os 33 JS foram repetidos pelo verificador. A build foi recompilada e `--quota-settings` passou em 27 checks em `scratch/capy-visual-e995794c-13c1-45a2-880d-82c0f117e47b/report.json`: executou o script gerado pelo executável com comando original Unicode e conferiu stdout literal. Entrada `{}` é inválida para o coletor e não gera amostra; a statusline anterior continua executando. Repetição independente deste novo check pendente. Nenhuma prova de percentuais reais/login/envio.

Três testes novos falharam antes da correção: leitura malsucedida criava artefatos, desconexão removia wrapper sem backup válido e statusline retirada na origem era restaurada silenciosamente. A leitura agora distingue NotFound dos demais erros e limita settings.json a 1 MiB. Restauração exige bridge com campo original explícito, mesma pasta e comando compatível. Backup incompleto também foi demonstrado em vermelho e corrigido. Os seis testes `claude_quota_` passaram; suíte completa antes da última guarda adicional: 108 Rust aprovados, um opt-in ignorado. 32 JS e builds frontend/desktop passaram.

`node scripts/verify-pet-native.mjs --quota-settings` passou em 26 checks na build final deste trecho: `scratch/capy-visual-ab72f917-45f2-4585-8866-5598fee9d1e6/report.json`. Perfil/settings/backup foram criados somente na raiz própria. Comandos Tauri conectaram e restauraram a configuração original, preservaram campos alheios/Unicode e rejeitaram backup corrompido/incompleto/ausente e alteração na origem, mantendo os bytes de settings.json intactos. Nenhum login/envio a provedor; não altera a statusline do usuário. A prova anterior de 24 checks antecede a guarda de backup incompleto e não é a final. Não prova percentuais reais, login de outra conta, statusline em execução ou C11 completo. Revisão independente deste trecho pendente.

Verificação independente desta rodada: `.checks/live-task-model.round2.verified.md` aprovou 36 checks nativos/32 JS, com renderer totalmente dentro do frame; `.checks/live-task-external.verified.md` aprovou 25 checks nativos, argumento literal e sobrevivência dos processos externos após saída. Ambos os runners encerraram seus processos próprios. Smoke posterior passou em 20 checks sem erros de frontend. Esses relatórios não encerram o checklist multifuncional completo.

## Revisão do layout e launcher externo — 2026-10-07

A primeira revisão independente da troca confirmou o comportamento, mas reprovou o layout: o renderer do xterm ultrapassava o fundo do terminal. O padding foi separado em `.terminal-frame`, mantendo o viewport sem padding. A prova do autor passou em 36 checks em `scratch/capy-visual-c1164bc5-e21b-44ff-b325-e3f9fae1f799/report.json`, incluindo limites do renderer dentro do viewport; `model-switch-result.png` foi aberto e inspecionado. Repetição independente pendente; o relatório inicial `.checks/live-task-model.verified.md` conserva a reprovação.

A primeira prova externa chegou à resposta real Haiku, mas falhou ao comparar a instrução: PowerShell decodificou `Não` como `NÃ£o`. A leitura agora explicita UTF-8. O teste local posterior também revelou corrupção de aspas, argumento vazio e barras finais pelo splatting nativo do PowerShell; o launcher passou a iniciar diretamente o processo com argumentos Windows escapados e diretório explícito. `external launcher preserves UTF-8 literal arguments folder and isolated authentication environment` passou, assim como os 32 testes JS. A tentativa externa falha em `scratch/capy-visual-489a01b3-bd31-4b23-a504-73f4e3b95120` não conta como prova aprovada. Repetição real após recompilar pendente.

O usuário informou que não possui serviço de API. Provas HTTPS reais permanecem dependentes de credenciais; contratos locais e Claude por assinatura continuam disponíveis. Isso não fecha C18 nem o objetivo completo.

Após recompilar, `node scripts/verify-pet-native.mjs --live-task-external` passou em 25 checks: `scratch/capy-visual-cdbbe8e5-1454-4670-91ee-c963889e48e0/report.json`, `external-task-receipt.json` e `external-processes.json`. O formulário iniciou o modo externo padrão com conta/cobrança/modelo/pasta/instrução exatos. Assistant Haiku respondeu no histórico do mesmo UUID; prompt UTF-8 foi conferido literalmente e nenhuma ferramenta foi observada. Não há ConPTY; os processos CLI/launcher sobreviveram à saída da Capy, com PID/caminho/criação idênticos. Limpeza posterior encerrou somente os dois processos próprios revalidados. A captura `external-task-panel.png` foi aberta e inspecionada, mas não é captura do terminal externo. Não houve automação do terminal nem aprovação de permissões. Tentativas intermediárias passaram nos 25 checks de produto e falharam na limpeza por array não enumerado do ConvertFrom-Json do Windows PowerShell; não foram contabilizadas como prova aprovada. Revisão independente pendente.

## Troca efetiva no terminal — 2026-10-07

`node scripts/verify-pet-native.mjs --live-task-model` passou em 35 checks na build atual: `scratch/capy-visual-74b0a439-5162-480c-9a68-ddd6c6b55ea4/report.json`, `model-switch-receipt.json`, `model-sonnet-selected.png` e `model-switch-result.png`. O primeiro turno foi confirmado em Haiku; o seletor oficial foi aberto pelo botão da Capy, a linha Sonnet foi observada selecionada e a tecla `s` aplicou a escolha somente à sessão. Uma resposta assistant posterior trouxe o marcador próprio e `message.model=claude-sonnet-5-5` no mesmo UUID/pasta. Os bytes de settings.json permaneceram iguais; nenhuma ferramenta apareceu no histórico observado. A conta/cobrança por assinatura permaneceu a mesma. Fonte: https://code.claude.com/docs/en/model-config.

A captura final foi aberta e inspecionada. Título/lista e confirmação de saída agora identificam o modelo do registro da tarefa como `modelo inicial`; não o apresentam como o atual depois da troca no CLI. O terminal passa a usar altura proporcional à janela, limitada a 600px: a altura fixa anterior cortava o seletor e uma repetição da prova não conseguiu observar Sonnet selecionado. As assertions de visibilidade/seleção foram mantidas. A execução anterior em `scratch/capy-visual-b7ad6745-0fdb-41fd-b332-0f0ae368f372` passou em 33 checks antes de corrigir os rótulos; não é a validação final destes rótulos.

31 JS e builds frontend/desktop passaram. Backend Rust não mudou. Revisão independente da troca e prova de CLI externo continuam pendentes. A revisão independente dos controles de geração aprovou 42 checks nativos e 31 JS em `.checks/live-task-controls.verified.md`; não encerra C9 dos demais provedores/superfícies ou a revisão final do plano.

## Controles reais durante geração — 2026-10-07

`node scripts/verify-pet-native.mjs --live-task-controls` passou em 42 checks na build desktop atual. Evidências privadas: `scratch/capy-visual-8fc15b67-5448-40c1-9296-53d9ad33b263/report.json`, `interrupt-active-receipt.json`, `interrupt-result.txt`, `exit-active-receipt.json`, `exit-before-approval.txt` e `task-exit.json`. Cada resposta longa é ligada ao prompt no histórico do UUID/pasta próprios e ao PID/data de criação do CLI oficial. Duas amostras registram aumento do contador oficial de tokens de saída durante o turno; não inferem geração de um processo vivo. O CLI 2.1.292 recolhe o texto até interromper/terminar; a transcrição não mostrou os trechos em geração. A prova não simula o estado `working`.

Os diálogos reais do WebView2 identificaram UUID/pasta. Consentimento ausente foi rejeitado pelo backend. Cancelar interrupção e saída conservou o processo e a geração: os tokens continuaram aumentando. Aprovar interrupção enviou Esc e o CLI mostrou `Interrupted`; trechos numerados com o marcador próprio permaneceram na tela. Um novo turno gerou tokens antes de cancelar e depois aprovar a saída. O diálogo final identificou somente o terminal próprio; a aplicação e o PID/data de criação Claude observados encerraram. Nenhuma ferramenta foi registrada no histórico observado. Isso prova interrupção e saída do terminal integrado durante geração Claude por assinatura, sem afirmar pausa de processo no Windows, chat API interrompido ou controle de sessões externas.

A execução inicial aprovada de 38 checks está em `scratch/capy-visual-aafd7e24-e87b-4efa-9d80-937e4cf60d5f`; suas capturas `interrupt-result.png` e `exit-active.png` foram abertas e inspecionadas: sessão/controles legíveis, texto parcial preservado e indicação de geração do turno seguinte. A execução final acrescentou assertions de progresso após cancelar, preservação do marcador parcial e ausência de ferramentas. Duas tentativas iniciais falharam na visibilidade: a seção ficou em -0.25px por arredondamento da rolagem do WebView. A margem de rolagem 8px e a rolagem após montagem/foco mantiveram a assertion original; a posição passou a 7.75px. Outras tentativas falharam na codificação da leitura de tela, envio do Enter junto ao texto e observação de texto parcial recolhido; nenhuma dessas tentativas conta como prova aprovada.

31 testes JS e build frontend/desktop passaram. O backend não foi alterado; última suíte completa: 105 Rust aprovados, um opt-in ignorado. Revisão independente deste trecho e smoke final ainda pendentes. C9 de outros provedores/superfícies e revisão final do plano permanecem abertos.

## Retomada de 2026-10-07 — perfis incompatíveis

Build frontend/desktop passou; suíte do autor: 105 Rust aprovados e um live opt-in ignorado, 31 JS aprovados. `node scripts/verify-pet-native.mjs --profiles-corrupt` passou em 26 checks, usando somente fixture própria, sem chamadas de IA. Artefatos em `scratch/capy-visual-49fb7bce-4322-4a8f-b429-ef1839a027ba/report.json` e `profiles-corrupt.png`. A captura foi aberta e inspecionada: aviso legível no chat, sem overflow horizontal. Os bytes incompatíveis permaneceram intactos antes/depois de operação rejeitada e saída; cadastro de perfis bloqueado, controles API disponíveis, leituras vazias de histórico/tarefas e preparação de saída aprovadas. A saída normal encerrou somente a instância de teste. Smoke final passou em 20 checks, sem erros de frontend.

Verificador independente repetiu os cinco testes `profiles_` e 26 checks nativos em `scratch/capy-visual-57512f59-c2b4-48ce-afb9-b0468189f5aa/report.json`; relatório limitado em `.checks/profiles-preservation.verified.md`. Não prova login/envio real, histórico populado neste modo ou edição simultânea entre conferência de bytes e gravação. C10 completo e revisão final C1–C19 permanecem abertos.

Executável local compilado em 2026-10-06. Esta prova não encerra C1–C19 nem substitui o verificador independente final.

`node scripts/verify-pet-native.mjs` passou em 12 checks no WebView2 do executável, em processo próprio com preferências e cache isolados. Artefatos privados: `scratch/capy-visual-1e858f7d-bcc2-4038-ba44-f6a9b98b9b02/report.json` e PNGs.

Provas observadas:
- Painel e resumo sem overflow horizontal na dimensão nativa padrão.
- Teclado visível durante a demonstração de trabalho.
- Hover move o olhar; saída da mascote limpa o olhar e não mantém acompanhamento global.
- Clique reage e abre o resumo.
- Ocultar pausa animações; reabrir retira a pausa.
- Preferência de movimento reduzido retira animações e translação do olhar.
- Novo pedido acena e mostra badge; abrir o painel marca a atenção como vista.

As capturas `waiting.png`, `working.png`, `summary.png` e `panel.png` foram abertas e inspecionadas. Preservam a capivara frontal, cores e tipografia existentes; os cards e footer do resumo permanecem legíveis no scroll e o painel não corta conteúdo horizontalmente. A captura de espera é um frame, não prova todo o movimento do braço.

Lacunas: arraste real com mouse físico, áudio nativo, saudação na inicialização real, sono após 180s reais, comemoração por conclusão real do provedor e dados/cards de integrações reais. Relógio/sono/saudação/conclusão têm testes do controlador; isso não fecha as lacunas nativas.

`npm run verify:native` passou nos 20 checks de janelas, geometria, IPC e descoberta, sem erros de frontend, após corrigir a permissão `is_visible` restrita à janela `pet`. A primeira tentativa foi interceptada pela prévia antiga aberta; a seguinte encontrou a ACL ausente. Nenhuma dessas tentativas falhas foi contabilizada como aprovação.

As provas não iniciaram turnos de IA nem mudaram configurações/credenciais de contas externas.

## Chat — inicialização nativa parcial

Depois da inclusão do chat, a build desktop e o smoke nativo passaram novamente; `node scripts/verify-pet-native.mjs` passou em 16 checks. Artefatos: `scratch/capy-visual-44f1d760-52f3-49ef-ac22-08d47e5d7e8a/report.json` e PNGs. Os quatro checks novos verificam comando `list_chats` com histórico próprio vazio, criação bloqueada antes de verificar conta, campo de chave password sem autocomplete e ausência de overflow horizontal. A captura `chat.png` foi aberta e inspecionada: formulário, origem CLI, modelo e ações estão legíveis no painel nativo com scroll.

Não foi cadastrada chave real nem enviado turno nesta prova. Não comprova cadastro pela UI, conversa CLI/API, cobrança efetiva, histórico com conteúdo, transferências ou animações ligadas ao chat. A execução de teste própria foi encerrada e o executável atualizado foi reaberto em modo normal.

## Chat Claude — turno real e ligação à mascote

`node scripts/verify-pet-native.mjs --live-chat` passou em 21 checks em 2026-10-06. Artefatos próprios: `scratch/capy-visual-c4bbfd68-bf0b-4765-b41f-62a79f1623b6/report.json`. Antes do único turno curto, o CLI oficial confirmou login, identidade e cobrança de assinatura; a prova rejeita API antes de enviar. Criou UUID próprio, observou a mascote trabalhando e comemorando somente após resultado confirmado, comparou resposta e UUID, abriu a conversa pelo card e verificou o término da comemoração. Preferências, dados da Capy e cache WebView2 ficaram isolados. Nenhuma sessão de trabalho existente foi retomada.

O teste `chat_cli_live_subscription_keeps_exact_history_across_model_change`, executado explicitamente com `--ignored`, passou em dois turnos reais próprios: criou UUID com Sonnet, retomou o mesmo UUID com Haiku e recuperou um marcador aleatório enviado apenas na primeira mensagem. A primeira execução encontrou a perda de identidade ao forçar CLAUDE_CONFIG_DIR no padrão; foi corrigida no launcher/auth/PTY, mantendo pasta explícita nos demais perfis. A falha anterior ocorreu antes de qualquer turno e não conta como prova aprovada.

Essas provas fecham somente o caminho Claude por assinatura exercitado. API real, transferências, outros provedores/modelos, eventos de conclusão de tarefas interativas, arraste/áudio e demais lacunas do plano continuam abertos. Smoke nativo passou novamente em 20 checks sem erro de frontend.

## Transferência de chat — revisão e execução real parcial

Após a build desktop mais recente, `node scripts/verify-pet-native.mjs --live-chat --live-transfer` passou em 29 checks em 2026-10-06. Artefatos privados: `scratch/capy-visual-d5715ea3-0e05-4f56-aa14-6021b4e902a0/report.json` e `transfer-review.png`. A captura foi inspecionada: origem, destino, modelo, cobrança e campos de revisão estão legíveis; o formulário permite scroll sem overflow horizontal.

A prova confirmou um turno Claude próprio, preparação pela interface, aceno/badge para revisão pendente, checkbox de tamanho adequado e consentimento inicialmente desmarcado. Aprovação sem revisão foi rejeitada sem consumir o pedido. A aprovação pela interface criou outro UUID com Haiku na mesma conta por assinatura, enviou o resumo editado e recebeu resposta com o marcador. Repetir a aprovação foi rejeitado sem criar terceira conversa.

Isso substitui a lacuna de transferência apenas para esse caminho exercitado. API real, outras contas/provedores, mudança efetiva de cobrança, recuperação revisada de envio incerto, conclusão de tarefas interativas e o restante do escopo continuam pendentes. A suíte atual passou em 82 testes Rust (um teste opt-in ignorado nesta execução), 28 JS e build frontend; os três testes de transferência incluem recuperação sem reenvio e preservação da resposta concorrente de outra conversa. Não substitui o verificador independente final.

## Processo do chat — encerramento forçado e launcher real

`chat_process_job_kills_only_owned_tree_when_owner_is_terminated` passou com processos Windows reais próprios: o proprietário fixture foi encerrado à força, filho e neto encerraram, e outro processo com o mesmo executável permaneceu aberto. O launcher cria o processo diretamente no job privado, herdando somente I/O; falha de associação impede o spawn, sem fallback. Referência: https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute (JOB_LIST e HANDLE_LIST) e https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects (KILL_ON_JOB_CLOSE).

Após a build desktop, `node scripts/verify-pet-native.mjs --live-chat --live-transfer` passou novamente nos 29 checks com o launcher novo. Artefatos privados: `scratch/capy-visual-f58c027c-3270-49a8-bb23-2687d5e49ed8/report.json`. O Claude real por assinatura recebeu ambos os turnos próprios, incluindo resumo editado na nova conversa Haiku. A gravação antecipada da tentativa CLI não quebrou versão/nonce ou aprovação única.

Suíte atual: 86 Rust aprovados, um teste live opt-in ignorado nessa suíte padrão; 28 JS e builds frontend/desktop aprovados. `chat_cli_attempt_survives_interruption_without_assuming_result_or_resending` prova preservação de mensagem/nonce e tentativa após reinício, mas não prova recuperação revisada: essa interface ainda falta. Nenhuma sessão de trabalho externa foi encerrada nesta prova. Não substitui a revisão independente final do escopo completo.

Smoke nativo da mesma build passou em 20 checks, sem erros de frontend; executável local reaberto em modo normal.

## Recuperação revisada — interface e contexto Claude reais

`node scripts/verify-pet-native.mjs --live-recovery --live-transfer` passou em 38 checks após a build desktop, em 2026-10-06. Artefatos privados: `scratch/capy-visual-2180c47a-ca62-437c-b63f-a404d9540cd8/report.json`, `chat-recovery.png` e `recovery-layout.json`. A captura foi aberta e inspecionada: identidade/cobrança, incerteza, consentimento obrigatório, confirmação sem reenvio e envio bloqueado estão legíveis. A conferência mediu checkbox de 13 px, documento de 750 px dentro de viewport de 760 px e nenhuma caixa fora da largura.

A prova usou três turnos curtos próprios por assinatura. Depois do primeiro resultado confirmado, encerrou a instância própria e simulou sua perda no registro da Capy antes da gravação final, preservando o histórico real do CLI. O reinício marcou unknown sem reenviar ou comemorar, pediu atenção e abriu a conversa correta. A revisão conferiu UUID/pasta e exigiu checkbox inicialmente desmarcado; o backend rejeitou aprovação ausente. Confirmar não acrescentou mensagem, consumiu uma aprovação única e preservou o nonce incerto. A nova mensagem em Haiku retomou o UUID original e recuperou um marcador aleatório que só constava da primeira instrução. O aviso de resposta perdida permaneceu visível após o novo sucesso. Depois disso, a transferência editada para um novo UUID também passou, sem replay.

A primeira execução falhou no check de layout, antes de aprovar a recuperação (`scratch/capy-visual-62482e39-c89b-485b-b0a5-348f5a29b650/report.json`). O consentimento herdava a grade dos formulários; recebeu o mesmo layout flex dos consentimentos de transferência. Não foi reduzida nenhuma assertion nem contada a falha como aprovação.

Esta prova simula perda na persistência após resultado real; não prova uma falha de rede ou cancelamento durante geração. API real, sessões CLI antigas sem proteção comprovada, demais contas/provedores e o escopo completo continuam sem validação final. Suíte: 91 Rust aprovados, um opt-in ignorado; 29 JS aprovados; builds frontend/desktop e 20 checks de smoke aprovados. Revisão independente final continua pendente.

## Avisos persistentes na transferência e capacidade da continuidade

Em 2026-10-06, a build desktop passou e `node scripts/verify-pet-native.mjs --live-recovery --live-transfer` passou nos 40 checks em `scratch/capy-visual-d86c5056-5896-4eda-bf3b-bf86e061fe61/report.json`. A captura `transfer-review.png` foi aberta e inspecionada: o aviso de origem aparece legível antes dos campos editáveis, identifica a mensagem sem resposta confirmada e informa possível consumo e envio do aviso ao destino. O documento continua sem overflow horizontal.

A prova nativa substitui o estado editável por texto sem o aviso de resultado perdido. Após aprovar, o contexto da nova conversa conserva o aviso original e a orientação de não repetir automaticamente a mensagem. O resultado posterior bem-sucedido não apagou o registro anterior. São três turnos curtos em conversas próprias Claude por assinatura; a perda é simulada no registro local depois de um resultado real confirmado, como na prova anterior.

Suíte: 96 testes Rust aprovados, um opt-in ignorado; 30 JS aprovados; frontend e desktop build aprovados. Smoke nativo da mesma build passou em 20 checks, sem erros de frontend. Testes de armazenamento exercitaram 199 recuperações consecutivas, transferência após o limite de mensagens, reserva até 512 nonces, rejeição de replay e de metadados incompatíveis sem sobrescrever arquivos. Isso não prova API real, outras contas/provedores nem conclui os demais checks do plano. Revisão independente final permanece pendente.

## Confirmação de saída — diálogo nativo e terminais próprios

Em 2026-10-06, `node scripts/verify-pet-native.mjs --exit-review` passou em 27 checks na build desktop atual. Evidências privadas: `scratch/capy-visual-0d692bcb-a352-4f48-b518-80b072c8b7e8/report.json` e `exit-confirmation.json`. O diálogo real do WebView2 foi observado pelo evento Page.javascriptDialogOpening, com tipo confirm e texto contendo UUID, conta, modelo e cobrança da conversa própria. Foi cancelado por Page.handleJavaScriptDialog; nenhum window.confirm foi substituído. O cancelamento conservou aplicação/registro; nonce cancelado e envio alterado foram rejeitados. Um campo incompatível no histórico bloqueou a saída, mostrou o motivo e permaneceu intacto no arquivo. Uma aprovação fresca posterior pelo diálogo encerrou somente a instância de teste.

O runner preserva a checagem inicial de histórico vazio; depois reinicia a própria instância com um histórico fixture. Nenhum provedor é chamado neste modo: working é um estado de teste persistido. Não prova interrupção durante geração nem cobrança real. As execuções anteriores falharam na preparação do histórico (`scratch/capy-visual-976e8e57-e999-4e96-a2eb-1815db18d2e1`) e no motivo de erro exibido (`scratch/capy-visual-daf1a131-8a8d-4e36-8e60-caf79179718b`); a preparação foi dividida em etapas e a mensagem de conferência recebeu contexto. A assertion de inicialização vazia foi mantida.

`terminal_native_close_releases_only_owned_pseudoconsole` passou com ConPTY/PowerShell reais: verificou UUID ativo, liberação explícita de writer/master, saída do processo próprio, recusa de entrada/resize após fechar e preservação de outro PowerShell independente. A implementação instalada de portable-pty libera ClosePseudoConsole, mantendo a leitura em outra thread; contrato primário: https://learn.microsoft.com/en-us/windows/console/closepseudoconsole. A prova de job do chat continua sendo distinta e não equivale a esta liberação de terminal.

Os testes de revisão cobrem confirmação obrigatória, nonce único, substituição/cancelamento, mudança de UUID/versão/envio/conta/modelo/cobrança/lista, bloqueio de nova admissão depois de aprovar e prazo incluindo espera/conferência. Suíte atual: 101 Rust aprovados, um opt-in ignorado; 31 JS aprovados; frontend e desktop build aprovados. C9 completo, provas reais de tarefas/quotas/contas/API e revisão independente final continuam pendentes.

Na mesma build, `--live-recovery --live-transfer` passou novamente nos 40 checks em `scratch/capy-visual-e1c4b663-57a8-4fc4-8801-0d287d512b5a/report.json`. Três turnos próprios Claude por assinatura confirmaram que a admissão de chat e de transferência mantém retomada exata, revisão obrigatória, aviso de incerteza no destino e rejeição de replay. Não exercita a saída enquanto o modelo gera uma resposta.

Smoke nativo final da mesma build passou em 20 checks, sem erros de frontend. Nenhuma sessão ou processo externo foi encerrado pelas provas.

## Tarefa Claude no terminal integrado — formulário e processo reais

Em 2026-10-06, `node scripts/verify-pet-native.mjs --live-task` passou em 29 checks com uma instrução curta própria em Haiku por assinatura. Evidências privadas: `scratch/capy-visual-a3ed3a43-fe21-46f3-a2a9-9c79ee9be59e/report.json`, `task-receipt.json`, `task-exit.json`, `task-terminal.png` e `task-model-picker.png`. O formulário verificou conta/cobrança e conservou pasta, UUID, modelo, instrução e modo integrado. A resposta foi encontrada como mensagem assistant no JSONL desse UUID e projeto, com modelo Haiku; o eco da instrução no terminal não contou como resposta. Nenhuma chamada de ferramenta foi registrada no histórico observado. A resposta do modelo incluiu o marcador; esta prova não avalia sua obediência a responder somente o marcador.

Ao anexar, o terminal agora aparece na área visível antes de focar sua entrada. A prova mediu título/UUID e viewport visíveis sem rolagem manual; a captura foi aberta e inspecionada, com identidade, controles, instrução e resposta legíveis, sem overflow horizontal. O runner confirmou confiança somente na pasta própria, selecionando Yes explicitamente a partir de No, exit; não aprovou ferramentas. O seletor oficial de modelo abriu por Alt+P e fechou por Esc sem novo turno. A captura mostra a abertura do menu; não prova troca efetiva nem visibilidade de todas as opções em um único quadro.

A revisão de saída identificou somente o terminal próprio. O diálogo real exigiu aprovação; a aplicação e o processo Claude exato encerraram. `task-exit.json` preserva PID/data de criação observados e resultado nulo após fechar. Não é uma prova de parada enquanto o modelo gera: a resposta já constava do histórico, e hooks de Stop ainda podiam estar em execução.

As duas primeiras tentativas ficaram na confiança da pasta, com o terminal fora da área visível (`scratch/capy-visual-b20ad133-48ab-4491-b44e-a02066a449fb` e `scratch/capy-visual-13015c8c-2aa5-4187-8c20-f7b802b27623`). A terceira confirmou resposta, seletor e saída, mas o auxiliar de polling rejeitou a aplicação já encerrada antes de verificar o PID (`scratch/capy-visual-75332395-0553-432e-afbb-c225732c7e42`). A observação final foi separada desse auxiliar; a exigência de encerramento do processo foi mantida. Nenhuma execução com falha conta como runner aprovado.

31 testes JS, builds frontend/desktop e smoke nativo de 20 checks passaram nesta alteração; o backend Rust não mudou, e sua última suíte passou em 101 testes com um opt-in ignorado. CLI externo, troca real de modelo no terminal, pausa/parada durante geração, quotas/contas/API e revisão independente final permanecem abertos. Referência dos controles oficiais: https://code.claude.com/docs/en/interactive-mode.
