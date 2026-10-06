# Respostas reais Codex — parte da etapa 0.4.0

Profile: light. Feature base: f5434a1.
Sources: conversa (executar todas as etapas), PRODUCT.md e .design/capy.md (pedido exato e rejeição de expirados), .checks/interventions-spike.md (transporte comprovado), https://learn.chatgpt.com/docs/app-server e schema do executável local (requestApproval/requestUserInput/serverRequest/resolved), interface existente em DESIGN.md.

## Boundary

Este slice implementa perguntas e aprovações de comandos/arquivos do Codex pela conexão inscrita. Claude e Antigravity permanecem no escopo das etapas gerais, com transportes/provas próprios ainda por implementar. Não abrir novas conversas, iniciar turnos, salvar regras, aceitarForSession, alterar autenticação ou executar terminal pelo Capy. Pedidos de MCP, rede gerenciada, permissões adicionais ou stdin sem representação completa permanecem na origem. QA visual nativo não é substituído por fixture de navegador.

## Landing

Reutilizar descoberta de sessão/identidade, estado real/demo e apresentação dos cards. Observação permanece read-only por padrão; inscrição para respostas exige ação explícita no card e é somente em memória. Um worker mantém o cliente e recebe respostas por canal; não reutilizar proxy transitório de atividade/cotas.

| Door | Literal shape | Alternative rejected |
| --- | --- | --- |
| Inscrição | initialize + account/read sem refresh de token para conferir modo de autenticação; thread/resume {threadId,excludeTurns:true}, sem overrides e sem turn/start; somente sessão Codex atual validada | resposta de um proxy read-only não recebeu a solicitação na prova |
| Identidade do pedido | nonce local + geração de conexão + threadId + turnId + itemId + ID RPC nativo; fonte revalidada ao enviar | somente sessionId permite responder ao próximo pedido por engano |
| Ciclo | pending → submitting → resolved; resolved/turn completed/interrupted/disconnect/account change invalida; nenhuma persistência | reaproveitar callback resolvido ou mantê-lo depois de reconectar mistura solicitações |
| Decisões | accept/decline/cancel por ocorrência, filtradas pelas availableDecisions; perguntas retornam answers por ID | aceitar por sessão/política cria permissões que o usuário não aprovou no card |
| Transporte persistente | actor serial sobre stdio, leitor limitado a quatro chunks de 64 KiB, polling de 200 ms, RPCs de 8 s e supervisor de 20 s para recolher o proxy próprio se inicialização/ator bloquear; inscrição perdida exige reconectar | um mutex sobre leitura bloqueante impede responder enquanto o daemon está quieto; repetir automaticamente pode duplicar uma resposta ambígua |
| Confirmação observada | serverRequest/resolved confirma resolução do pedido; a UI diz Pedido resolvido no Codex e a prova real confere a resposta consumida pela conversa própria | o evento também pode representar resposta na origem ou limpeza por interrupção; sozinho não identifica quem respondeu |

## Checks

### S1 — Resposta no pedido exato · ~15 arquivos · ~100 KB · ~25k leitura

**C1** — parser reconhece somente os três métodos suportados, valida thread/turn/item/ID RPC e perguntas (1–4, opções 2–4, texto até 4096 bytes); payload acima de 64 KiB e formatos desconhecidos não habilitam resposta.
Proof: Rust `interventions::tests::request_contract_and_bounds`.

**C2** — aprovação mostra comando/cwd ou mudanças de arquivos antes da decisão; rede gerenciada, permissões adicionais e stdin sem contexto suportado não habilitam aceitação; decisões de sessão/política são rejeitadas.
Proof: Rust `interventions::tests::approval_context_and_decisions`.

**C3** — nonce errado, thread/turn/item/geração distintos, sessão encerrada/trocada/oculta e modo demo não enviam nada; uma resposta válida muda pending para submitting e duplo envio não repete a mensagem.
Proof: Rust `interventions::tests::exact_request_and_single_submission`.

**C4** — resolved, fim/interrupção de turno, desconexão ou mudança de conta remove os pedidos; nova conexão não herda callbacks. Envio apenas não é rotulado como confirmação de entrega: aguardar serverRequest/resolved.
Proof: Rust `interventions::tests::resolution_and_connection_lifecycle`.

**C5** — inscrição do cliente manda initialize/initialized e thread/resume com excludeTurns:true sem alterações de config; mantém callback da conexão, e envia somente a resposta escolhida ao ID exato. Não envia thread/start, turn/start ou política.
Proof: Rust `interventions::tests::subscription_and_response_protocol`.

**C6** — card Codex real exibe pergunta/opções ou contexto da aprovação escapados, assina somente por ação explícita em cada cartão, preserva preenchimento/foco enquanto a mesma identidade pending atualiza, bloqueia duplo envio e remove controles ao expirar/ocultar. Respostas secretas usam campo password. Demo permanece isolado.
Proof: JS `real interventions preserve exact request identity and discard stale controls`; JS `intervention submission resolves only the current visible pending Codex identity`.

**C7** — executável release de prova recebe pergunta própria após inscrição, entrega a resposta pelo mesmo Service usado pelo comando Tauri, observa resolved e rejeita resposta expirada sem resolver a pergunta seguinte. A aprovação de alteração do arquivo próprio é negada e o arquivo não é criado. O harness de stdin está atrás da feature Cargo `intervention-proof`, limitado a `scratch/codex-capy-*`; o executável de entrega é compilado sem essa feature.
Proof: `node scripts/verify-codex-interventions.mjs` — PASS em `question_delivered`, `expired_rejected`, `next_request_preserved`, `approval_declined`, `denied_write_absent`; prova repetida no release após a asserção confirmar `item/fileChange/requestApproval` e o caminho do arquivo esperado.

## Swept

- Validation: C1/C2/C3, limites de entrada, identidade e fonte.
- Failure/dependency: C4, falha remove pedidos e mostra motivo; proxy próprio recolhido, daemon compartilhado preservado.
- Retry/idempotency: C3, nenhuma repetição automática de resposta após falha ambígua.
- Authorization: C2/C3/C5, inscrição por ação explícita e decisão por pedido; sem alteração de regras/contas.
- Concurrency/ordering: C3/C4, worker serial, geração e estados; resultado no Snapshot somente pelo monitor principal.
- Lifecycle: C4, só memória; Capy reinicia sem inscrições anteriores; sessões ocultas não permitem responder.
- Transitions: C3/C4, submitting não significa entregue; desconhecido não significa concluído.
- Account lifecycle: C4/C5, `account/read` observa o modo sem renovar token; atualização inicial é comparada, pedidos ficam inválidos em qualquer `account/updated` e a conexão fecha se o modo de autenticação mudar.
- Observability: C6, motivo e contexto no card; sem perguntas/respostas/comandos em logs ou artefatos públicos.

## Handoff

S1 inteiro (~25k leitura) no principal. Os campos de protocolo do spike já foram comprovados, mas nenhum C1–C7 está fechado por essa investigação. Verificador independente fresh obrigatório após o último commit da feature inteira; range f5434a1..HEAD. As etapas gerais e releases finais permanecem abertas até suas provas completas.

## Estado da implementação

C1–C7 implementados. O C7 passou duas vezes no release Windows de prova usando o mesmo `Service` dos comandos Tauri; a última execução confirmou especificamente `item/fileChange/requestApproval` e o caminho criado apenas no pedido. O harness de stdin está disponível somente com a feature `intervention-proof` e valida uma pasta em `scratch/codex-capy-*`; o executável de entrega será reconstruído sem essa feature. A prova local guarda somente cinco booleanos em `scratch/codex-interventions-proof/proof.json`.

Validação do código atual: 40 testes Rust, 11 JS, TypeScript/Vite e `npm run verify:native` passaram. Próximos gates obrigatórios: commit final da feature, verificador independente fresh sobre `f5434a1..HEAD`, build release sem `intervention-proof`, smoke nativo, QA visual da interface de intervenção real e conferência do preview local. As seis etapas finais permanecem abertas até as suas provas completas.
