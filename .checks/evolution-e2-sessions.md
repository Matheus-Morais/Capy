# E2 — terminal interativo e sessões simultâneas

Fontes: conversa (usuário pediu testar lacunas); PLANO_EVOLUCAO.md; QA anterior em evolution-e2-qa-results.md. Perfil tlc-implement: light. Base: 4774f5c.

Escopo: release local atual, sessões Claude próprias, interface nativa e hooks reais. Fora: Codex/Antigravity, instalador, matriz completa de DPI, cancelamento/reinício (amostras posteriores). Não modificar sessões externas ou settings globais.

## Checks

- C1: duas sessões Claude reais distintas ficam com pedidos Write pendentes no mesmo serviço ao mesmo tempo. Prova: verify-claude-concurrent.mjs, snapshot conjunto e identidades UUID/origens distintas.
- C2: responder B negar antes de A permitir pela interface entrega decisões somente às sessões respectivas. Prova: mesmo runner, metadata hook/reject B e hook/accept A, arquivo B ausente e A com conteúdo exato, continuação de ambas.
- C3: Claude sem `-p`, executado em terminal interativo real, emite pedido Write próprio visível no Capy. Prova: verify-claude-interactive.mjs, argv, transcript de terminal e snapshot/screenshot real.
- C4: decisão pela interface no cenário interativo retorna pelo hook e Claude continua; arquivo corresponde à decisão. Prova: mesmo runner, stdout bruto do hook, transcript posterior e arquivo próprio.
- C5: hash do release e 170 fontes permanecem iguais ao manifesto anterior; settings globais/marker preservados; processos próprios encerrados e app normal inicialmente aberto restaurado. Prova: evolution-e2-sessions-environment.json antes/depois.
- C6: revisão independente identifica evidências e limites por critério. Prova: evolution-e2-sessions.verified.md.

## Landing e execução

Reutilizar runners nativos/CDP anteriores. Nenhuma porta irreversível: somente scripts de QA e evidências. Providers executados sequencialmente pelo orquestrador devido ao marker compartilhado; simultaneidade C1 ocorre dentro de um único runner com um único app. Duas fatias pequenas (runner de concorrência e runner interativo), sem handoff adicional.

## Swept

Validação: UUID, origem e conteúdo exatos C1–C4. Falhas externas: registrar FAIL e preservar logs sem transformar fallback em aprovação. Retry/idempotência: sem retry de ferramenta solicitado. Autorização: hooks e modo manual existentes. Concorrência/ordem: C1/C2. Ciclo de dados: scratch próprio, C5. Transições: pendente → decisão → continuação C2/C4. Observabilidade: logs, snapshots, screenshots e revisão C6. Sem alteração de produção prevista.
