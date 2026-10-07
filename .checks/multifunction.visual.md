# Prova nativa parcial da Capy multifuncional

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
