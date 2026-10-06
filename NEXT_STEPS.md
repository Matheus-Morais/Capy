# Retomada do Capy

Atualizado em 2026-10-06: esperas reais do Codex comprovadas pelo release e verificadas independentemente (C1–C7 PASS).

## Onde paramos

- Desktop Windows em Rust/Tauri 2 com capivara frontal, arraste, resumo, painel e bandeja. O usuário confirmou o arraste.
- Descoberta real de Claude Code e Codex, com validação de presença, projeto, identidade e ocultação persistente.
- Codex: leitura do daemon já aberto, sem iniciar ou retomar conversas. Estados working/waiting/idle/unknown; falhas descartam o estado anterior. Idle não significa conclusão.
- Pedidos reais são respondidos no agente de origem. Capy ainda não oferece resposta ou acesso ao terminal real.
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
