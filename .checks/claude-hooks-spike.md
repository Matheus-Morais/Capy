# Prova isolada de hooks Claude

Executada em 2026-10-06, Claude Code 2.1.291, diretório scratch/claude-hooks-spike (ignorado). Nenhuma sessão externa retomada ou usada. Configurações existentes não foram editadas; hooks de teste via `--settings`, fontes user/project/local desabilitadas apenas no processo de teste.

Sessão nova com Write como única ferramenta, modo manual, entrada/saída stream-json e host de permissão stdio. O host recebeu `can_use_tool`, aguardou três segundos e negou a escrita. Um hook Stop separado bloqueou somente a primeira parada, exigindo continuação sem ferramentas.

Sequência observada: SessionStart → UserPromptSubmit → PreToolUse(Write) → PermissionRequest(Write) → host em espera → negação → PostToolBatch → Stop(stop_hook_active=false) → Stop(stop_hook_active=true) → SessionEnd. Resultado success, um controle de permissão, exit 0. Nenhuma escrita de probe.txt autorizada.

Conclusões: PermissionRequest precede a espera real; PostToolBatch limpa a espera após negação nesta versão; Stop não confirma idle porque pode ser seguido de continuação sem outro UserPromptSubmit. PermissionDenied e PostToolUseFailure não apareceram nessa negação pelo host. O observador não pode depender deles exclusivamente.

Limites: fluxo SDK/print, não diálogo de terminal interativo. idle_prompt, perguntas AskUserQuestion, cancelamento, eventos de subagentes e paralelismo não foram demonstrados nesta prova. Registros de teste guardam apenas campos selecionados; os dados locais não são fixtures versionadas.
