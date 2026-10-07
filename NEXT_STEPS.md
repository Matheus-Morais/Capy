# Retomada do Capy

Atualizado em 2026-10-06: prévia local 0.4.0-alpha.2 inclui respostas a perguntas/aprovações do Codex e animação expressiva do estado de espera. QA visual nativo dos cards reais continua pendente.

## Plano multifuncional em implementação

O escopo completo fica em `.design/multifunction.md` e `.checks/multifunction.md`; nenhuma entrega parcial encerra o objetivo. O checkout agora contém movimentos com memória, preferências/alertas de quota, perfis Claude, lançamento de tarefas, PTY integrado e fluxo de revisão de continuação. Isso ainda não é uma nova release publicada/empacotada.

Após as provas, a build local em `src-tauri/target/release/capy.exe` foi aberta. Antes de recompilar/testar, verificar o caminho do processo e fechar somente essa instância do repositório; a cópia empacotada anterior permanece em `releases/0.4.0-alpha.2/Capy.exe`.

Validação atual em 2026-10-06: 101 testes Rust e 31 JS passaram; um teste Rust live opt-in foi ignorado na suíte padrão. Builds frontend/desktop, 20 checks de smoke, 27 de saída revisada e 40 de recuperação/transferência Claude passaram. `.checks/multifunction.visual.md` registra o alcance e as lacunas. Resumos ligam ferramentas aos resultados, preservam instruções/trechos `.md` recebidos e avisam sobre histórico parcial. Revisões são de uso único; cobrança, identidade e fim de turno são revalidados. Tarefas longas ficam em registros individuais e revisões têm leitura/gravação compatíveis.

Ainda faltam: validar chat API com credenciais reais e outros provedores; provar saída durante geração real, statusline/contas/trocas e tarefas nos dois terminais em sessões próprias; referências das instruções carregadas automaticamente; gestão de origem/modelos dos demais provedores; eventos reais de conclusão de tarefas interativas; áudio, arraste e QA de cards reais; tratar os intervalos/lifecycle dos hooks, perfis incompatíveis e revisão final independente. Sugestões adicionais de personalidade continuam propostas abertas. Não declarar esses recursos concluídos com base somente nos testes de contrato.

Chat próprio: `credential_vault.rs` e `api_accounts.rs` guardam somente metadados no JSON e as chaves em entradas UUID próprias do Windows; `chat_api.rs` fixa os contratos OpenAI/Anthropic/Gemini, bloqueia redirecionamento/reenvio e distingue conclusão de resposta parcial/recusada; `chat_history.rs` persiste versão e nonce antes de enviar, impede concorrência/replay e recupera envios interrompidos como incertos. `chat_cli.rs` usa stdin UTF-8 e UUID exato para criar/retomar Claude, mantendo login oficial e verificando resultado de sucesso da própria sessão. Comandos/UI oferecem cadastro de chave, verificação de conta/cobrança, conversas e transferência revisada. Turnos e transferência reais por assinatura Claude foram exercitados, incluindo estado/conclusão na mascote. API real e recuperação revisada de envio incerto continuam pendentes; não confundir contratos fixture com consumo bem-sucedido do provedor.

Prévia local atual: [0.4.0-alpha.2](releases/0.4.0-alpha.2/NOTES.md), com respostas Codex por pedido exato e sinais animados por estado. Executável em `releases/0.4.0-alpha.2/Capy.exe`. A anterior [0.2.0-alpha.2](releases/0.2.0-alpha.2/NOTES.md) mantém as cotas reais da conta Codex conectada. O estado das seis entregas finais fica em `.checks/releases.md`; a 0.4.0 final e as demais etapas continuam abertas. Publicação remota foi dispensada pelo usuário. Os registros abaixo descrevem as entregas anteriores e seus limites.

## Onde paramos

Recuperação de chat: revisão persistida por versão/nonce e identidade/cobrança revalidadas, sem reenvio na confirmação. Uma anotação ligada à mensagem preserva a incerteza depois de novos sucessos. Claude retoma somente histórico do UUID/pasta conferidos, sem processo ativo; sem histórico, o envio fica bloqueado e o caminho é transferência revisada. CLI antigo sem proteção comprovada não é presumido encerrado. `--live-recovery --live-transfer` passou em 38 checks com contexto real Claude por assinatura após simular perda no registro próprio, retomada em Haiku e transferência posterior. Captura inspecionada após corrigir layout de consentimento. Suíte: 91 Rust aprovados (um opt-in ignorado), 29 JS, frontend/desktop e smoke de 20 checks aprovados. Ainda faltam API real, demais provas e revisão final; a simulação não comprova falha de rede durante geração.

Continuidade validada nesta build: avisos de resultados perdidos permanecem no contexto da transferência mesmo depois de sucesso posterior ou edição do resumo. O aviso de origem é separado dos campos editáveis. Todos os nonces antigos são preservados, com capacidade de 512 e espaço reservado para recuperação/transferência; testes exercitam 199 recuperações e o limite de capacidade. Suíte: 96 Rust aprovados (um opt-in ignorado), 30 JS, frontend/desktop, 40 checks nativos de recuperação/transferência e smoke de 20 checks aprovados. Captura inspecionada; evidências em `.checks/multifunction.visual.md`.

Saída normal agora prepara lista exata de chats em envio e terminais integrados, com conta/modelo/cobrança e aprovação única de 60s. Mudança de envio/lista exige nova revisão; após aprovar, novos trabalhos não são admitidos. Erro de leitura bloqueia a saída e é mostrado no painel. ConPTYs próprios são liberados explicitamente. `--exit-review` passou em 27 checks com diálogo real e histórico fixture, sem consumo de IA; teste ConPTY real preservou um PowerShell independente. Suíte: 101 Rust aprovados (um opt-in ignorado), 31 JS, builds frontend/desktop aprovados. Recuperação/transferência Claude reais passaram novamente nos 40 checks em `scratch/capy-visual-e1c4b663-57a8-4fc4-8801-0d287d512b5a/report.json`.

Tarefa integrada foi exercitada pelo formulário no Claude real: Haiku por assinatura, pasta própria, UUID/instrução preservados e resposta assistant no histórico exato. `--live-task` passou em 29 checks, incluindo terminal visível automaticamente, seletor oficial aberto/fechado e saída revisada com encerramento do PID próprio. Captura inspecionada; artefatos em `scratch/capy-visual-a3ed3a43-fe21-46f3-a2a9-9c79ee9be59e`. 31 JS e builds frontend/desktop passaram; smoke nativo passou em 20 checks. O backend Rust não mudou neste trecho; última suíte: 101 aprovados e um opt-in ignorado. A resposta precedeu a saída; não prova interrupção durante geração nem troca efetiva de modelo.

Próximos pontos do escopo: provar saída durante geração real e controles de pausa/parada; provar CLI externo, quotas/contas, troca de modelo no terminal e demais movimentos no nativo; conferir preservação de perfis incompatíveis, continuação de fontes API sem chave antiga disponível e lifecycle dos hooks. A escolha do provedor para validar API real foi solicitada; a chave deve ser cadastrada no aplicativo, nunca na conversa. As sugestões adicionais de personalidade continuam abertas e não foram implementadas.

Ciclo de vida do chat: `chat_process.rs` cria o CLI diretamente num job privado Windows com encerramento dos processos associados ao fechar o proprietário, herdando somente os três handles de I/O. A prova encerrou à força um proprietário fixture e confirmou a saída de filho/neto, preservando outro processo com mesmo executável. A tentativa CLI é gravada antes do spawn; reinício conserva mensagem/nonce e estado incerto, sem inferir resposta ou reenviar. Suíte atual: 86 Rust aprovados (um opt-in ignorado), 28 JS, frontend e desktop build aprovados. O launcher real passou novamente nos 29 checks nativos com dois turnos próprios Claude por assinatura e transferência revisada para Haiku. Recuperação revisada de envios incertos é o próximo trecho, e não está entregue por essa proteção.

Continuação mais recente: transferência de chat com sete campos revisáveis, identidade/cobrança revalidadas, aprovação única persistida e journal de recuperação sem reenvio automático. Revisões pendentes acionam aceno e badge. A build desktop passou; o runner nativo `--live-chat --live-transfer` passou em 29 checks, incluindo rejeição sem revisão, criação de novo UUID com Haiku na mesma conta Claude por assinatura, envio do resumo editado e rejeição do replay. Captura de revisão inspecionada; evidências em `.checks/multifunction.visual.md`. A suíte passou em 82 testes Rust (um opt-in ignorado) e 28 JS. Ainda faltam recuperação revisada de envio incerto, provas reais de quotas/contas/tarefas, API/outros provedores e verificação final independente. A troca na mesma sessão Sonnet → Haiku foi provada separadamente no teste opt-in anterior; não equivale à transferência com nova conversa.

A build local foi recompilada e reaberta após a inclusão do chat. Smoke nativo passou sem erros de frontend; o runner visual passou em 16 checks, incluindo quatro sobre inicialização/controles do chat. Prova e lacunas estão em `.checks/multifunction.visual.md`. Isso ainda não é release empacotada ou prova de conversa real.

- Desktop Windows em Rust/Tauri 2 com capivara frontal, arraste, resumo, painel e bandeja. O usuário confirmou o arraste.
- Descoberta real de Claude Code e Codex, com validação de presença, projeto, identidade e ocultação persistente.
- Codex: leitura do daemon já aberto, sem iniciar ou retomar conversas. Estados working/waiting/idle/unknown; falhas descartam o estado anterior. Idle não significa conclusão.
- Capy 0.4.0-alpha.2 permite responder a perguntas e aprovações suportadas do Codex pelo pedido exato; animação de espera sinaliza pedidos pendentes. Claude/Antigravity seguem sem esses controles e nenhum terminal real é aberto.
- Claude: hooks silenciosos opcionais, identidade por sessão/PID/criação/projeto e ancestral Claude nativo. Evidência expira em 30 s; Stop é unknown porque pode continuar. Sessões sem hooks ou sem evidência recente permanecem unknown. Antigravity e cotas reais seguem pendentes.
- Release compilado e reaberto: `src-tauri/target/release/capy.exe`. Hooks habilitados na configuração local; sessões Claude já abertas precisam ser reiniciadas pelo usuário para carregar a habilitação.
- Commits desta etapa: `8626c8a` (contrato e prova), `9d0e5ff` (integração).
- Validação: 7 testes JS, 22 Rust e 20 verificações nativas passaram; prova release própria demonstrou working → waiting → working → unknown com continuação e remoção da sessão encerrada. Verificação independente C1–C6 PASS; evidências em `.checks/claude-activity.verified.md`.
- Codex: runner próprio comprovou approval e user input reais → waiting no release → working após resposta na origem. Um novo proxy observou a espera e o arquivamento removeu a sessão do relatório. Escrita negada não criou arquivo. 25 testes Rust passaram, incluindo recuperação após falha, exatamente 64 leituras para 65 IDs e fronteira de 1 MiB. Sem mudanças no comportamento de produção.

## Espera Codex entregue

Prova opt-in: `node scripts/verify-codex-waiting.mjs`. Requer daemon aberto e release compilado; usa a conta conectada para dois turnos em uma conversa própria. Cria projeto exclusivo em `scratch`, nega a escrita, responde à pergunta na origem e arquiva somente a conversa criada. Evidências locais permanecem em `scratch`; checklist em `.checks/codex-waiting.md` e verificação independente em `.checks/codex-waiting.verified.md` (C1–C7 PASS; runner real, 25 Rust, 7 JS e build repetidos pelo verificador).

Limites: prova via cliente App Server, não diálogo visual; falha e recuperação do daemon são comprovadas por contrato, sem encerrar o daemon compartilhado. Reabertura de proxy foi comprovada de verdade. `thread/unsubscribe` mantém conversas carregadas por até 30 minutos; remoção imediata foi provada via arquivamento, não desconexão. Fonte: https://learn.chatgpt.com/docs/app-server.

## Atividade Claude entregue

Habilitação: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/claude-hooks.ps1 -Action Enable`; remoção: `-Action Disable`. Preserva hooks alheios; remoção invalida registros e bloqueia hooks ainda carregados até nova habilitação. Reiniciar sessões Claude para carregar a configuração. Remover antes de mover o repositório, pois o hook usa caminho absoluto do release.

Provas isoladas: `scripts/verify-claude-settings.ps1` e `node scripts/verify-claude-hooks.mjs`. Esta última inicia somente uma sessão própria, com teto de US$ 1, nega sua única escrita e testa continuação via Stop. Não usar a sessão externa do Prisma como sessão de teste.

Limites: prova real usa print/SDK, não diálogo interativo; idle_prompt, perguntas, elicitações e subagentes não têm prova real completa. TTL implica unknown em trabalho/espera longa. Registrar somente metadados mínimos; não inferir ociosidade de Stop.

Não declarar respostas pelo Capy disponíveis com base apenas na observação de hooks.

## Próximo trabalho, em ordem proposta

1. **Codex em espera real — entregue e verificado:** ambas as flags e a remoção da espera passaram no release em sessão própria; reabertura de proxy e fronteiras de 64 sessões/1 MiB também. Interrupção/reinício real do daemon compartilhado permanece sem prova; recuperação após falha tem teste de contrato. Verificação independente C1–C7 PASS.
2. **Antigravity:** provar presença e projeto na CLI/IDE. Diretórios de histórico não comprovam sessão aberta; evitar interpretar conversas protobuf para inferir atividade.
3. **Intervenção e terminal:** investigar como abrir a origem correta e, quando suportado, responder ao pedido exato. Provar identidade, pedido expirado e entrega real antes de habilitar controles.
4. **Cotas por conta/provedor:** encontrar fonte confiável, associar à conta correta e tratar informação antiga/indisponível. Sem porcentagens inventadas.
5. **Validação visual nativa:** conferir cards reais, badge do mascote, ocultação/restauração e ciclo de abertura/fechamento. A revisão atual da apresentação usa funções/testes e smoke; não equivale a QA visual nativo.

## Cuidados para retomar

- Conferir `git status` antes de editar. Usar RTK conforme `C:/Users/MOBILTEC/.codex/RTK.md`.
- O Capy pode estar aberto. Antes de rebuild ou `npm run verify:native`, encerrar somente o processo cujo caminho corresponde ao release deste repositório e aguardar sua saída. Reabrir ao terminar.
- Comandos: `npm test`, `cargo test --manifest-path src-tauri/Cargo.toml`, `npm run build`, `npm run desktop:build`, `npm run verify:native`.
- Diagnósticos: `capy.exe --discover-report <arquivo>` e `capy.exe --activity-report <arquivo>`.
- `.checks/discovery-local.json` e `.checks/activity-local.json` são privados e ignorados pelo Git; não publicar identificadores/caminhos locais como fixtures.
- O monitor consulta o daemon existente por WebSocket sobre stdio do proxy. JSONL direto nesse proxy não funcionou; iniciar outro app-server não permite observar as sessões externas existentes.
- Manter a arte aprovada e a distinção explícita entre demonstração e integração real.

## Referências locais

- `.design/capy.md`: decisões do produto.
- `.design/session-discovery.md`: presença e integrações pendentes.
- `.design/codex-activity.md`: contrato de atividade Codex.
- `.design/claude-activity.md`: contrato de atividade Claude.
- `.checks/claude-hooks-spike.md` e `.checks/claude-activity.md`: prova e checklist da etapa Claude.
- `.checks/claude-activity.verified.md`: verificação independente e limitações.
- `.checks/session-discovery.verified.md` e `.checks/codex-activity.verified.md`: evidências e limitações.
- `README.md`: execução e comportamento atual.
