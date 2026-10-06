# Antigravity — versão 0.2.0

Profile: light. Base: 6f3bac3. Sources: conversa (seis etapas), PRODUCT.md, `.design/session-discovery.md`, https://antigravity.google/docs/hooks, schema local de conversation_summaries.db observado em modo read-only.

## Landing

Reutiliza descoberta, Session, monitor e preferências. Nova integração isolada em `antigravity.rs`; novos hooks opcionais silenciosos para atividade e metadados de presença em IDE. CLI pode ser descoberto por bloqueio e metadados sem hooks. Registros de hook exigem ancestral nativo do Antigravity e PID/criação ainda vivos.

| Door | Literal shape | Alternative rejected |
| --- | --- | --- |
| Metadados CLI | rusqlite bundled, open read-only, SELECT workspace_uris e parent_conversation_id pelo ID bloqueado | histórico textual/protobuf mistura conteúdo privado e dados antigos |
| Atividade | hooks opt-in PreInvocation/PostToolUse/Stop, evidência TTL30s | inferir trabalho de timestamp/status persistido não demonstra atividade atual |
| Configuração | grupo capy-observer em config/hooks.json, preservando grupos alheios | sobrescrever hooks impediria regras existentes |

## Checks

**C1** — bloqueio mantido mais metadados válidos gera uma sessão CLI com ID estável e projeto; histórico desbloqueado e subagentes ficam fora.
Proof: Rust `antigravity::tests::held_presence_requires_workspace_and_main_session`.

**C2** — fonte ausente, JSON inválido, schema incompatível ou URI remota produzem diagnóstico, sem derrubar Claude/Codex.
Proof: Rust `antigravity::tests::invalid_metadata_is_diagnostic`.

**C3** — hook publicado por ancestral válido associa conversa/workspaces à identidade PID/criação; processo encerrado ou reutilizado fica fora.
Proof: Rust `antigravity::tests::hook_presence_requires_live_identity`.

**C4** — atividade working só com evidência recente; Stop não prova conclusão; evidência >30s volta a unknown.
Proof: Rust `antigravity::tests::activity_is_bounded_and_never_done`.

**C5** — habilitar é idempotente, remover preserva configuração alheia e invalida hooks carregados.
Proof: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-antigravity-settings.ps1`.

**C6** — sessão CLI própria aparece com projeto correto e working durante ação real, desaparece após encerramento; runner não retoma conversas externas.
Proof: `node scripts/verify-antigravity.mjs` — `cli_working`, `cli_removed`.

**C7** — sessão IDE própria produz presença/projeto e atividade pelo mesmo contrato, sem tratar histórico como aberta.
Proof: teste nativo de IDE própria a definir após spike; não declarar PASS com prova somente de CLI.

## Swept

- Validation: C1–C3, leitura limitada a 64 sessões e64KiB metadados.
- Failure/dependency: C2; locks não bloqueantes, SQLite busy timeout zero.
- Retry/idempotency: C5, hook escreve metadados sem duplicar sessão CLI.
- Authorization: leitura de fontes externas, hooks opt-in; nenhum pedido aceito pelo observador.
- Concurrency/ordering: revalidar lock ou PID antes de publicar; hook com bloqueio de arquivo.
- Lifecycle: C3/C4/C5; não manter estado ativo de processo encerrado.
- Transitions: C4; unknown não é idle ou done.
- Observability: diagnóstico por agente; evidências privadas em scratch.

## Handoff

Etapa inteira no agente principal. Verificador após tarefas concluídas. C7 ainda requer prova nomeável; a descoberta CLI pode ser implementada/verificada como primeiro slice, a versão inteira permanece aberta.

## Execução

- Slice 1: descoberta por presença/metadados implementada. C1 e C2 passaram em 27 testes Rust, nenhum falhou ou foi ignorado. Ainda não verificados independentemente; não equivalem à prova runtime C6/C7.
- Slice 2: coletor de hooks e configuração reversível (C3–C5), pendente.
- Slice 3: provas próprias CLI/IDE (C6/C7), verificador, versão e publicação, pendente.
