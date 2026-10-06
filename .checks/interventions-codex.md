# Respostas reais Codex — parte da etapa 0.4.0

Profile: light. Feature base: f5434a1.
Sources: conversa (executar todas as etapas), PRODUCT.md e .design/capy.md (pedido exato e rejeição de expirados), .checks/interventions-spike.md (transporte comprovado), https://learn.chatgpt.com/docs/app-server e schema do executável local (requestApproval/requestUserInput/serverRequest/resolved), interface existente em DESIGN.md.

## Boundary

Este slice implementa perguntas e aprovações de comandos/arquivos do Codex pela conexão inscrita. Claude e Antigravity permanecem no escopo das etapas gerais, com transportes/provas próprios ainda por implementar. Não abrir novas conversas, iniciar turnos, salvar regras, aceitarForSession, alterar autenticação ou executar terminal pelo Capy. Pedidos de MCP, rede gerenciada, permissões adicionais ou stdin sem representação completa permanecem na origem. QA visual nativo não é substituído por fixture de navegador.

## Landing

Reutilizar descoberta de sessão/identidade, estado real/demo e apresentação dos cards. Observação permanece read-only por padrão; inscrição para respostas exige ação explícita no card e é somente em memória. Um worker mantém o cliente e recebe respostas por canal; não reutilizar proxy transitório de atividade/cotas.

| Door | Literal shape | Alternative rejected |
| --- | --- | --- |
| Inscrição | thread/resume {threadId,excludeTurns:true}, sem overrides e sem turn/start; somente sessão Codex atual validada | resposta de um proxy read-only não recebeu a solicitação na prova |
| Identidade do pedido | nonce local + geração de conexão + threadId + turnId + itemId + ID RPC nativo; fonte revalidada ao enviar | somente sessionId permite responder ao próximo pedido por engano |
| Ciclo | pending → submitting → resolved; resolved/turn completed/interrupted/disconnect/account change invalida; nenhuma persistência | reaproveitar callback resolvido ou mantê-lo depois de reconectar mistura solicitações |
| Decisões | accept/decline/cancel por ocorrência, filtradas pelas availableDecisions; perguntas retornam answers por ID | aceitar por sessão/política cria permissões que o usuário não aprovou no card |

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

**C6** — card real exibe pergunta/opções ou contexto da aprovação escapados, controla ações suportadas por request nonce, preserva preenchimento enquanto o mesmo pedido atualiza, bloqueia duplo envio e remove controles ao expirar. Demo permanece isolado.
Proof: JS `real interventions preserve exact request identity and discard stale controls`.

**C7** — executável release recebe pergunta própria após inscrição, entrega a resposta pelo mesmo serviço usado pelo comando Tauri, observa resolved e rejeita resposta repetida/expirada sem resolver pergunta seguinte. A aprovação própria é negada e o arquivo não é criado.
Proof: `node scripts/verify-codex-interventions.mjs` — named proofs `question_delivered`, `expired_rejected`, `next_request_preserved`, `approval_declined`, `denied_write_absent`.

## Swept

- Validation: C1/C2/C3, limites de entrada, identidade e fonte.
- Failure/dependency: C4, falha remove pedidos e mostra motivo; proxy próprio recolhido, daemon compartilhado preservado.
- Retry/idempotency: C3, nenhuma repetição automática de resposta após falha ambígua.
- Authorization: C2/C3/C5, inscrição por ação explícita e decisão por pedido; sem alteração de regras/contas.
- Concurrency/ordering: C3/C4, worker serial, geração e estados; resultado no Snapshot somente pelo monitor principal.
- Lifecycle: C4, só memória; Capy reinicia sem inscrições anteriores; sessões ocultas não permitem responder.
- Transitions: C3/C4, submitting não significa entregue; desconhecido não significa concluído.
- Observability: C6, motivo e contexto no card; sem perguntas/respostas/comandos em logs ou artefatos públicos.

## Handoff

S1 inteiro (~25k leitura) no principal. Os campos de protocolo do spike já foram comprovados, mas nenhum C1–C7 está fechado por essa investigação. Verificador independente fresh obrigatório após o último commit da feature inteira; range f5434a1..HEAD. As etapas gerais e releases finais permanecem abertas até suas provas completas.

## Estado da implementação

Primeiro checkpoint: parser/contexto/geração de resposta em src-tauri/src/interventions.rs e provas C1/C2 passaram na camada própria (2 testes nomeados). O módulo ainda não está conectado ao transporte persistente, ao Snapshot ou ao frontend: nenhum controle novo real foi habilitado e nenhuma release nova foi gerada a partir desse checkpoint. Próximo trabalho obrigatório: registro de callbacks por geração e ciclo C3/C4, transporte C5, cards C6, runner release C7 e verificação independente da feature completa. Não usar os probes da investigação como substitutos desses gates.
