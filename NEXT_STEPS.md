# Retomada do Capy

Salvo em 2026-10-05 a pedido do usuário: encerrar o trabalho de hoje e retomar amanhã.

## Onde paramos

- Desktop Windows em Rust/Tauri 2 com capivara frontal, arraste, resumo, painel e bandeja. O usuário confirmou o arraste.
- Descoberta real de Claude Code e Codex, com validação de presença, projeto, identidade e ocultação persistente.
- Codex: leitura do daemon já aberto, sem iniciar ou retomar conversas. Estados working/waiting/idle/unknown; falhas descartam o estado anterior. Idle não significa conclusão.
- Pedidos reais são respondidos no Codex. Capy ainda não oferece resposta ou acesso ao terminal real.
- Claude: presença funciona; atividade continua desconhecida. Antigravity e cotas reais seguem pendentes.
- O release atualizado foi reaberto ao encerrar a entrega: `src-tauri/target/release/capy.exe`.
- Últimos commits da entrega: `f4c9551` (atividade Codex), `e25aaaf` (verificação independente).
- Validação: 7 testes JS, 18 Rust e 20 verificações nativas passaram; revisão independente C1–C7 PASS.

## Próximo trabalho: atividade do Claude Code

1. Ler `.design/session-discovery.md`, `.design/codex-activity.md` e os relatórios em `.checks/` para preservar os contratos já comprovados.
2. Provar hooks numa sessão de teste própria e isolada, preservando os hooks/configurações existentes. Não usar a sessão externa do Prisma como sessão de teste.
3. Demonstrar eventos de início de trabalho, espera e término/continuação; verificar que uma intervenção desaparece quando deixa de ser válida.
4. Definir um coletor mínimo ligado à identidade da sessão/processo, com leitura limitada, ordenação e expiração. Guardar somente metadados necessários, sem prompts, respostas, histórico ou credenciais.
5. Após a prova, registrar contrato e checklist, implementar a integração e a habilitação reversível dos hooks sem sobrescrever os demais. Manter unknown quando a observação não for confiável.
6. Rodar testes e validação nativa, obter verificação independente e reabrir o release atualizado.

Não declarar respostas pelo Capy disponíveis com base apenas na observação de hooks.

## Depois, em ordem proposta

1. **Codex em espera real:** validar ambas as flags de intervenção e a remoção do pedido em sessão de teste própria. Hoje essas transições têm testes de contrato; o spike real demonstrou active/idle, e o release confirmou working. Testar também desconexão/reabertura e revisar limites de 64 sessões/1 MiB, hoje verificados estruturalmente.
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
- `.checks/session-discovery.verified.md` e `.checks/codex-activity.verified.md`: evidências e limitações.
- `README.md`: execução e comportamento atual.
