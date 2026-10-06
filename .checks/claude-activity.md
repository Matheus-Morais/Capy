# Atividade Claude — checklist

Profile: light. Base: 93988ca. Sources: .design/claude-activity.md (binding), NEXT_STEPS.md e .checks/claude-hooks-spike.md. Autorização na conversa em 2026-10-06.

## Plan

Observador Rust silencioso acionado antes da inicialização Tauri; arquivo único por sessão sob bloqueio exclusivo. Monitor e relatório enriquecem a descoberta existente. Script PowerShell habilita/remove grupos próprios preservando o restante do JSON.

## Checks

**C1** Estados da decisão 2, subagentes ignorados e Stop desconhecido mesmo em continuação. Proof: `claude_activity::tests::event_contract`.

**C2** Leitura exige sessão, cwd, PID e criação atuais; malformação, >64 KiB, idade >30s, futuro e bloqueio resultam unknown. Até 64 sessões enriquecidas. Proof: `claude_activity::tests::identity_expiry_and_limits`.

**C3** Espera substituída por working/unknown/idle, falha ou expiração; request/command ausentes. Proof: `claude_activity::tests::waiting_is_invalidated`.

**C4** Coletor lê no máximo 1 MiB, grava somente metadados permitidos; escrita ordenada ignora evento anterior e limpa no máximo 128 registros encerrados por escrita. Proof: `claude_activity::tests::bounded_metadata_and_order`; inspeção localizada de identidade ancestral; spike com release `scripts/verify-claude-hooks.mjs` exige metadados vinculados ao Claude real e nenhuma saída do observador.

**C5** Habilitar duas vezes é idempotente; remover preserva configurações/hooks alheios e invalida arquivos; JSON inválido não é alterado. Proof: `scripts/verify-claude-settings.ps1` em diretório isolado.

**C6** Release real demonstra working → waiting → working após negação → unknown em Stop com continuação; sessões terminadas desaparecem. Proof: `scripts/verify-claude-hooks.mjs` (processo próprio); `npm run verify:native`; `npm test` mantém cards sem respostas.

## Swept

Validação/falha/identidade: C1/C2/C4. Ordenação/concurrency: bloqueio, instante capturado na entrada, C4. Retry/idempotência: C5, scan sem cache. Ciclo de dados: decisão 4, C4/C5. Falha externa: unknown em C2/C3. Transições: C1/C3/C6. Observabilidade: diagnóstico e relatório, C6. Autorização: observação silenciosa, resposta somente no host de teste isolado; controles reais continuam recusados pelo contrato existente.

## Landing

| Decision | Literal shape | Alternative rejected |
|---|---|---|
| Stop concorrente | unknown | idle antes da decisão dos demais hooks |
| Longa ausência de eventos | TTL 30 s e unknown | manter espera antiga indefinidamente |
| Arquivo da sessão | lock exclusivo para leitura/escrita, truncate sob lock | JSON parcialmente legível |
| Hooks suportados | CLI Claude nativa Windows com ancestral claude.exe | aceitar qualquer emissor pelo ID da sessão |

## Handoff

Uma entrega: cerca de 30 KB existentes + 25 KB novos, cabe em um agente. Verificador independente após último commit; nenhuma delegação de construção.
