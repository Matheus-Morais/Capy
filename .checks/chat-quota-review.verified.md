# Revisão automática de quota do chat próprio

**Veredito: PASS no grupo local C13, passos 1–3. O objetivo multifuncional e a prova real de automação por quota continuam abertos.**

Data: 2026-10-07. Verificador independente do autor. Range: `5678462..3360249` (`a62ef75`, `de97787`, `3360249`). Fontes: `.design/multifunction.md` integral, critérios C13 de `.checks/multifunction.md`, skill tlc-spec-driven e referências validate/coding-principles.

O usuário pediu parada em ponto seguro. Este registro contém somente resultados já concluídos. A auditoria geral C1–C19 começou por leitura, mas não terminou e não produziu `.checks/multifunction.final-audit.md`. Nenhum teste, build ou chamada de provedor foi iniciado após a solicitação de parada.

## Critérios e assertions inspecionados

| Obrigação | Evidência e resultado esperado | Resultado local |
| --- | --- | --- |
| Cache de identidade/conta/cobrança exato, até 120 s | `src-tauri/src/profiles.rs:220` compara account/billing/profileId; `:221` aceita 121000 para observação 1000; `:222` recusa passado/futuro e 121001; `:224` recusa login, conta e billing inválidos; `:227` recusa alteração externa do perfil. | PASS |
| Gatilhos 5h e semanal independentes | `src-tauri/src/chat_routing.rs:62` exige nenhuma revisão abaixo do limiar para 300/10080 minutos; `:63–64` exige revisão no limiar, destino q, modelo sonnet e versão exata. | PASS |
| Working e unknown aguardam fim do turno | `src-tauri/src/chat_routing.rs:74` exige waitingTurn, UUID exato e nenhuma revisão; `:75` conserva espera após recuperação unknown; `:78` exige versão do turno terminado. | PASS |
| Cadeia ordenada, destinos suportados e esgotamento | `src-tauri/src/chat_routing.rs:93` exige r/haiku após q esgotado; `:95` exige exhausted, UUID exato, aviso e nenhuma revisão sem alternativa. Candidates são somente targets CLI válidos em `:4–6`. | PASS |
| Dado antigo, futuro, sem conta e drift de origem não acionam | `src-tauri/src/chat_routing.rs:85` exige revisões vazias nos cinco modos; `:88` exige unavailable e nenhuma revisão após troca da conta. | PASS |
| Preparar somente revisão, sem envio/destino | `src-tauri/src/chat_transfer.rs:220–222` exige uma conversa, completed, duas mensagens, activeNonce ausente e usedNonces intactos; `src-tauri/src/chat_routing.rs:65` confirma cardinalidade/nonces intactos. | PASS |
| Revisão atual e manual preservadas | `src-tauri/src/chat_transfer.rs:224` exige None e bytes intactos para revisão existente; `:229–230` exige os mesmos bytes e nonce manual após tentativa automática. | PASS |
| Cancelamento durável e nova origem elegível | `src-tauri/src/chat_transfer.rs:225–227` exige revisão ausente, supressão após reload e aprovação recusada; `:234` exige novo nonce/versão depois de outro turno. | PASS |
| Revalidação sob trava e registros incompatíveis preservados | `src-tauri/src/chat_transfer.rs:136–145` lê/revalida sob ids.lock; `:245` rejeita revision/account/billing/model/state divergentes; `:249–252` rejeita snapshot antigo, working e unknown; `:254` exige erro e bytes futuros intactos. | PASS |
| Nova revisão sem alteração da versão e edição preservada por nonce | `scripts/verify-pet-native.mjs:183` exige versão 2 e título automático; `:188` exige draft intacto após snapshot. Controller real em `src/chat-transfer-ui.ts:34–40` foi também exercitado isoladamente para UUID/versão/nonce. | PASS no fluxo sintético |
| Consentimento e billing novos | `tests/chat-transfer-view.test.mjs:9–18` exige títulos manual/automático, reviewed required, billingConfirmed required para mudança de cobrança e nenhuma checkbox checked. `src-tauri/src/chat_transfer.rs:161–164` mantém verificações de consentimento, nonce, targets e confirmação de cobrança. | PASS local |

## Integração inspecionada

`src-tauri/src/monitor.rs:32–36` inclui perfis de origem de chats configurados no probe; `:106` chama evaluate com chat_targets e quotas atuais. `:119`, `:144` e `:156–158` incluem as revisões no snapshot e em sua comparação antes de emitir demo-updated. `src/chat-ui.ts:117` encaminha snapshots reais ao controller. `src/chat-transfer-ui.ts:36–40` seleciona UUID/versão exatos e conserva campos quando o nonce não muda. A aprovação Tauri em `src-tauri/src/chat_commands.rs:168–179` revalida ambas as identidades antes do Store e do envio. Não foi encontrado defeito concreto no range examinado.

## Gates executados pelo verificador

Em cópia isolada do commit 3360249, com target de debug próprio:

- `cargo test --manifest-path <scratch>/src-tauri/Cargo.toml chat_automatic_review -- --nocapture`: 2 aprovados, zero falhas, 133 filtrados.
- Mesmo comando com `chat_routing`: 4 aprovados, zero falhas, 131 filtrados.
- Mesmo comando com `profiles_chat_targets`: 1 aprovado, zero falhas, 134 filtrados.
- `node --test tests/chat-transfer-view.test.mjs`: 2 aprovados, zero falhas.
- Controller copiado de produção, com bridge/presentation substituídos por stubs somente na scratch: baseline aprovado para UUID errado, versão antiga, revisão nova, mesmo nonce com edição, novo nonce e lista vazia.

Os 133 Rust/2 live opt-in ignorados, JS34 e desktop build são resultados declarados do autor, não execuções deste verificador. Nenhum teste live nem provedor foi chamado nesta revisão.

## Sensor de discriminação

Baseline verde antes das mutações. Cada mutação Rust alterou somente fonte de produção copiada e foi restaurada em finally. Todos os seis processos terminaram com código 101 por assertion de comportamento, sem timeout ou erro de compilação.

| Mutação isolada | Assertion que a matou | Resultado |
| --- | --- | --- |
| Remover igualdade de versão/target/state/model na revalidação automática | `src-tauri/src/chat_transfer.rs:245`, caso revision exige None | KILLED |
| Remover supressão de revisão já existente na mesma origem | `src-tauri/src/chat_transfer.rs:224`, prepare automático exige None | KILLED |
| Persistir dismissed=false ao cancelar | `src-tauri/src/chat_transfer.rs:225`, active_transfer_review exige None | KILLED |
| Forçar ended=true no roteamento | `src-tauri/src/chat_routing.rs:74`, quantidade de statuses exige 1 (recebeu 0) | KILLED |
| Retirar expiração de 120 s do cache | `src-tauri/src/profiles.rs:222`, chat_targets(121001) exige vazio | KILLED |
| Retirar correspondência da identidade atual da origem | `src-tauri/src/chat_routing.rs:88`, quantidade de statuses exige 1 (recebeu 0) | KILLED |
| Retirar retorno para mesmo nonce no controller | Proof isolada exige objective='edited' após sync do mesmo nonce | KILLED |
| Retirar correspondência de sourceId no controller | Proof isolada exige view.hidden=true para UUID alheio | KILLED |
| Retirar correspondência de sourceRevision no controller | Proof isolada exige view.hidden=true para versão antiga | KILLED |

Sensor: **9 injetados, 9 mortos, zero sobreviventes**. Os três sensores UI provam o controller com dependências simuladas; não substituem WebView/Tauri. A mutação de preservação morreu no caso de revisão existente antes da assertion manual; a preservação manual foi conferida por assertion e baseline, sem alegar que aquele mutante chegou ao ramo manual.

## Prova nativa e limites

Relatório existente `scratch/capy-visual-283d457a-c311-4474-bebc-b87f2d23c411/report.json` foi lido: passed=true, 22 checks aprovados. Assertions do runner foram inspecionadas; `automatic-chat-review.png` foi aberta pelo verificador e mostra título, origem/destino e resumo editado legíveis.

O runner escreve revisão e histórico **sintéticos próprios**. Ele comprova recepção da revisão pelo monitor/UI sem mudança de versão, consentimento, edição, cancelamento persistido e rejeição da aprovação cancelada. `scripts/verify-pet-native.mjs:195–197` exige dismissed=true, uma conversa, versão 2, duas mensagens, activeNonce null, revisão null e aprovação expirada. Não comprova quota real, identidade/login real da alternativa nem decisão disparada por percentual observado de um provedor. A tentativa anterior a2d57e79 falhou por seleção antes da descoberta e não conta como prova aprovada.

## Estado preservado e parada

Porcelain antes dos sensores e último porcelain terminalmente observado depois de todos os sensores foram idênticos: somente os quatro arquivos preexistentes `.checks/interventions-codex.verified.md`, `.checks/interventions-codex.round2.verified.md`, `.checks/interventions-codex.round3.verified.md` e `releases/0.4.0-alpha.1/Capy.exe.sha256`, todos untracked. Nenhuma fonte/teste real foi editada; nenhum stash, commit, release build, login ou parada de aplicativo ocorreu.

Session6263 terminou com exit 0 depois de registrar o sexto mutante; session46696 terminou com exit 0 para o baseline. Os scripts de sensores não ficaram ativos. A scratch restaurada permanece em `C:/Users/MOBILTEC/AppData/Local/Temp/capy-chat-quota-sensor-183d692d-384e-465b-ba49-9316e05c902a`. A remoção recursiva foi rejeitada pela política automática; não houve remoção parcial observada. Nenhuma autorização adicional foi solicitada. A sessão de comando18376, somente Resolve-Path/status, pode ainda precisar ser recolhida pelo orquestrador; não executa testes, builds ou mutações.

Lacunas genuínas do grupo local: nenhuma identificada. Lacunas de prova mantidas abertas: automação por quota real Claude, cadeia entre contas reais e demais provedores conforme capacidades validadas. C13 inteiro, C1–C19 e o objetivo original não foram marcados completos. A auditoria geral ficou interrompida antes do relatório; suas leituras parciais não constituem aprovação geral.
