# Verificação limitada — preservação de perfis

**Veredito: PASS no trecho de preservação de perfis de C10.** Não conclui C10 inteiro nem o plano multifuncional.

**Profile:** light. **Rodada:** 1, limitada. **Verificador:** subagente independente; autor != verificador. Nenhum código ou commit alterado pelo verificador. Sem injeção de falhas, conforme o perfil.

**Estado examinado:** diff local sobre `b60cbd666995284ddac937c64577312d84e095f3`, em 2026-10-07, nos arquivos `src-tauri/src/profiles.rs`, `src/accounts-ui.ts`, `src/chat-ui.ts`, `src/dashboard.ts`, `src/tasks-ui.ts` e `scripts/verify-pet-native.mjs`. As provas abaixo rodaram nesse estado local; não representam uma revisão integral de base..HEAD.

## Fontes e alcance

Lidos `.design/multifunction.md`, `.checks/multifunction.md` (inclusive “Preservação de perfis” e `Swept`) e o diff dos seis arquivos. A decisão exige validar a lista inteira, preservar arquivo incompatível/indisponível, bloquear operações após alteração externa, criar diretório novo exclusivamente sob o diretório próprio e manter inicialização de chat/API, histórico e saída apesar do erro de perfis. Nenhuma contradição concreta encontrada neste trecho.

## Evidência localizada

| Obrigação | Prova executada independentemente | Evidência | Resultado |
|---|---|---|---|
| Dados inválidos/campos desconhecidos/tamanho excessivo não são descartados nem sobrescritos; operações ficam bloqueadas | `profiles_invalid_metadata_blocks_operations_without_overwriting` | `src-tauri/src/profiles.rs:210–212`: `assert!(store.list().is_err())`, rejeição de `get/probe/add`, `assert_eq!(std::fs::read(&path).unwrap(),bytes)` e `assert!(!root.join("accounts").exists())` | PASS |
| IDs/pastas duplicados e mais de 64 entradas preservam a lista inteira | `profiles_duplicate_metadata_is_preserved` | `src-tauri/src/profiles.rs:217–223`: casos de mesmo ID, mesma pasta e 65 entradas; `assert!(store.list().is_err())` e comparação dos bytes originais | PASS |
| Edição/remoção externa bloqueia operações antes de criar pasta de login | `profiles_disk_changes_block_operations_before_login_directory_creation` | `src-tauri/src/profiles.rs:233–236`: rejeição de `list/add`, `assert_eq!(std::fs::read(&path).unwrap(),edited)` e `assert!(!root.join("accounts").exists())`, também após remoção do arquivo | PASS |
| Nova pasta fica sob o diretório próprio e não reutiliza pasta existente | `profiles_new_directory_rejects_outside_accounts_parent`; `profiles_isolate_new_login_and_preserve_existing`; inspeção da criação exclusiva | `src-tauri/src/profiles.rs:248`: `assert!(new_config_dir(&root,"fixture").is_err())` e nenhuma pasta no destino externo; `:261–264`: pastas distintas, nova pasta dentro da raiz, sem credencial copiada e segredo existente intacto. `:43` usa `std::fs::create_dir(&path)` | PASS |
| Erro de perfis aparece no painel, bloqueia cadastro e preserva bytes sem impedir controles API, leitura de histórico e saída | `node scripts/verify-pet-native.mjs --profiles-corrupt` | `scripts/verify-pet-native.mjs:133–141`: comparações exatas da fixture, controles desabilitados, invocação Tauri `add_profile` rejeitada, ausência de `accounts`, controles API disponíveis, aviso e `list_tasks/list_chats/prepare_exit_review` acessíveis. `:397–398`: saída real do processo e bytes intactos após saída | PASS |

## Execuções independentes

- `rtk proxy cargo test --manifest-path src-tauri/Cargo.toml profiles_`: exit 0; os cinco testes acima apareceram individualmente como `ok`; 5 passaram, 0 falharam, 101 filtrados.
- `node scripts/verify-pet-native.mjs --profiles-corrupt`: exit 0; 26 checks passaram, 0 falharam. Relatório: `scratch/capy-visual-57512f59-c2b4-48ce-afb9-b0468189f5aa/report.json`. Desses, dez são específicos da preservação/continuidade de inicialização/saída; os demais são checks gerais do runner. Sem chamadas de IA; fixture e processo próprios encerrados.

## Sweep e limites

Releitura das regras existentes relevantes: validação integral e guarda de bytes em `profiles.rs:26–62,78–93`; identidade atual antes de `probe` em `:65`; comandos de login e configuração de quotas passam por `store.get` em `main.rs:278–280`. Erro de perfis não propaga pela inicialização das tarefas (`tasks-ui.ts:123`); contas/API são consultadas separadamente (`chat-ui.ts:46`); as três inicializações são isoladas e `ui_ready` permanece alcançável (`dashboard.ts:143–147`). Listeners de saída são registrados antes da leitura de perfis em `tasks-ui.ts:115–119`. Nenhuma obrigação existente deste trecho foi removida.

Limites de amostragem e nível: a prova nativa usa histórico vazio e conta API não cadastrada, comprovando inicialização/controles/comandos, sem autenticação, cobrança, envio ou recuperação de histórico não vazio. Erro de leitura por ACL e todos os tipos de campo inválido não foram exercitados individualmente; o código compartilha a rejeição preservadora, mas a execução amostra JSON inválido, campo desconhecido, provedor inválido, limite, duplicidade e edição/remoção. Criação exclusiva é evidência de código; colisão no identificador novo não foi forçada. A comparação de bytes detecta mudança presente nas conferências; não houve prova de edição externa simultânea entre a última conferência e o rename, nem de troca simultânea de junction. Não se afirma exclusão transacional de escritores externos.

Sem tabelas `Coverage`/`Test policy` a recomputar sob este perfil; nenhuma enumeração visual de design foi exigida nesta revisão light. A prova confirma os controles e comandos registrados, não fidelidade visual completa.

## Evidência adicional do autor

O principal informou suíte completa com 105 testes Rust passando e um opt-in ignorado, 31 testes JS passando, build desktop release aprovado e runner `--profiles-corrupt` com 26 checks em `scratch/capy-visual-49fb7bce-4322-4a8f-b429-ef1839a027ba/report.json`. Esses resultados são do autor; não substituem as execuções independentes acima.

Login oficial real de perfil próprio, demais comportamentos de C10 e verificação final independente de todos os checks do plano continuam pendentes. Este PASS não encerra o escopo multifuncional.
