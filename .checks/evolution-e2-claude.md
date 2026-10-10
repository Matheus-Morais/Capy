# E2 — canal de respostas Claude

Sources: `PLANO_EVOLUCAO.md` E2; `.checks/evolution-e2-research.md`; documentação oficial https://code.claude.com/docs/en/hooks e https://code.claude.com/docs/en/agent-sdk/user-input consultada em 09/10/2026.

Profile: light. Estado: fatia Claude implementada experimentalmente; revisão independente sem defeito substantivo confirmado, C7 parcial. E2 integral permanece aberta até provas reais restantes e demais integrações. Evidências: [revisão](evolution-e2-review.md), [amostra real](evolution-e2-live.md), [build](evolution-e2-build.json) e [smoke](evolution-e2-native-smoke.json).

## Out of scope

Permissões persistentes/Always, rede sandboxed, subagentes Claude, elicitações MCP, mudança de conta, retomada externa, alterações em configurações reais sem opção explícita, fechamento das lacunas Codex/Antigravity por fixtures.

## Landing

Canal Claude separado do observador e do serviço Codex. Reutiliza Context/View para apresentação, sem alterar o registro Codex. Endpoint TCP efêmero somente loopback, descoberta em pasta própria de configuração, token UUID aleatório; payload de ferramentas somente em memória. Configuração de respostas exige `-Responses` explícito; observação atual continua com prazo de dois segundos.

| Decisão | Forma | Alternativa rejeitada |
| --- | --- | --- |
| Contexto Claude identificável | generation começa `claude:`, sessionId `claude:<uuid>`, threadId UUID, turnId nonce da invocação, itemId tool_use_id quando existe ou nonce | Inventar ID de ferramenta em PermissionRequest |
| Canal vivo e limitado | 64 pedidos, frame 64 KiB, prazo interno 110 s, hook externo 120 s | Registro em disco de decisões sem conexão viva |
| Falha de entrega | Consumir tentativa uma vez; erro de entrega exige conferir origem | Reenviar automaticamente uma aprovação |
| Compatibilidade UI | Campos opcionais multiSelect/allowFreeText e body claudePermission | Modificar semântica de isOther Codex |

## Checks

### S1 — protocolo e canal síncrono

**C1** — PermissionRequest aceita somente decisão explícita suportada e produz JSON oficial; AskUserQuestion preserva perguntas e fornece respostas completas, com seleção única/múltipla/texto livre; payload inválido não concede permissão.
Proof: testes Rust nomeados `claude_intervention_protocol_*`, com relatório por caso.

**C2** — Duas conexões simultâneas recebem respostas próprias; contexto trocado, geração diferente e replay são rejeitados.
Proof: testes Rust nomeados `claude_intervention_service_*` de loopback real, com assertions de contexto e saída.

**C3** — Conexão encerrada, prazo vencido, identidade inválida, reset/demo e stop invalidam pedidos; app ausente retorna controle silenciosamente ao Claude sem decisão.
Proof: testes `claude_intervention_service_*` e revisão do caminho collect/cleanup; limites de simulação/processo explicitados no relatório.

### S2 — integração e configuração

**C4** — Configuração isolada Enable -Responses instala dois canais síncronos com timeout120 (PermissionRequest e PreToolUse matcher AskUserQuestion); observer2s continua intocado; repetição é idempotente e remoção/downgrade preservam hooks alheios.
Proof: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-claude-settings.ps1` com casos explícitos.

**C5** — Cartões Claude exibem ferramenta/entrada escapadas e perguntas com escolhas/texto livre; submissão só aceita contexto Claude visível em modo real, sem enfraquecer Codex.
Proof: testes JS nomeados `Claude interventions ...` e testes Codex existentes em `tests/presentation.test.mjs`; `npm run build` para contrato TypeScript.

**C6** — Aplicação registra serviço, incorpora pedidos ativos ao snapshot, encaminha resposta por provedor e invalida ao entrar em demonstração/fechar. Identidade PID/criação/ancestral e sessão/projeto são revalidadas antes da resposta.
Proof: inspeção independente main/monitor/validador e testes do serviço. Essa prova não substitui interação nativa real.

### S3 — prova externa e transparência

**C7** — Sessão Claude própria real recebe aprovação e pergunta pelo hook no Windows; modos print e interativo, concorrência/cancelamento/timeout têm resultados classificados individualmente.
Proof: `scripts/verify-claude-interventions.mjs --temporary-optin --passive-permission-host` com `CAPY_TEST_CAPY_EXE` apontando à build debug `intervention-proof`. Print passou deny/pergunta/continuação, contexto e replay; uso interativo, allow, concorrência, cancelamento e timeout reais permanecem pendentes. Não substituir essas provas por loopback.

**C8** — Documentação classifica canal como experimental se C7 incompleto e explica opção -Responses, reinício de sessões e retorno à origem em falha.
Proof: revisão independente README/CAPACIDADES/NEXT_STEPS/plano.

## Swept

- validation: C1/C2/C6, payload/contexto/processo/projeto.
- failure modes: C3, sem decisão em falha, incerteza de entrega explícita.
- idempotency and retry: C2, nonce e tentativa única.
- authorization: C4, opção explícita; C6 modo real e sessão visível.
- concurrency and ordering: C2/C3, conexão de cada invocação e limites.
- data lifecycle: endpoint sem payload, pedidos transitórios, cleanup C3.
- external-dependency failure: C3/C7/C8, fonte ausente ou incompatível continua na origem.
- state transitions: pendente → resposta tentada ou invalidado; sem retorno automático a pendente.
- observability: documentação C8 e evidências externas separadas C7.

## Handoff

Uma fatia Claude dividida por arquivos: parser, serviço e configuração em autores separados; root integra main/monitor/UI. Leitura estimada <100k tokens por autor, abaixo de 150k. Verificador fresco depois da integração. Não modificar alterações de produto preexistentes fora das linhas necessárias; não incluir essas alterações em commits por acidente.
