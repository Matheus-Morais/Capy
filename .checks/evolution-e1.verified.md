# Evolução E1 — verificação independente

**Verdict: PASS — C1–C6 comprovados no escopo documental/proveniência.**
**Profile:** light. **Round:** 1 — full.
**Diff range:** `f349f706b57063d6bc53510724046537797508bb..de3f53920aafc49c2ca86eb93b186d5af8c19931`.
**Verifier:** subagente fresco, independente dos autores; única escrita neste relatório.

## Fontes e método

Lidos checklist `.checks/evolution-e1.md`, E1 em `PLANO_EVOLUCAO.md:41–53`, matriz, README, inventário, retomada, manifesto e log. Confronto focado com código e relatórios históricos citados; nenhuma contradição localizada. Perfil light não requer enumeração UI, Coverage/Test policy ou fault injection. Não houve build, testes de aplicação, UI ou chamadas de IA nesta revisão.

## Checks

| Check | Evidência e prova independente | Resultado |
|---|---|---|
| C1 | `CAPACIDADES.md:7–16` tem quatro variantes × oito dimensões. Grupos A/B: `:20–22`, relatórios Claude `:3,19–24`, descoberta `:18–24`, espera Codex `:22–28`; TTL em `claude_activity.rs:12` e `antigravity.rs:14`. C: `:24`, checklist Antigravity `:60–61` preserva CLI parcial/C7 aberto. D: `:26`, relatório round3 `:3,23–29` distingue PASS funcional de FAIL visual, registry `:15,24` restringe Codex e `interventions.rs:109` implementa requestUserInput. E: `:28`, `source_access.rs:13,47,107` restringe link/handler/despacho. F/G: `:30–32`, relatório agy `:3,19–23,31`, código `antigravity_quotas.rs:22,73` usa 60/120 s e `agy_usage.rs:14` consulta /usage. H: `:34`, `chat_cli.rs:26`, `tasks.rs:45,81,98`, auditoria multifuncional `:64,70–71` conserva amostras e lacunas API/contas/origem. | PASS |
| C2 | `README.md:11–17` descreve respostas Codex e chat/tarefas próprios e separa CLI, IDE e cotas Antigravity; API real/contas alternativas explicitamente abertas. Conferido contra C1 e fontes acima. Busca focada nas afirmações de integração não encontrou contradição nova no README. | PASS |
| C3 | Repetidos Get-ChildItem/Get-Item/Get-FileHash SHA256: sete executáveis, tamanhos/hashes iguais às linhas aplicáveis de `ENTREGA_ATUAL.md:12–13,29–35`. O target atual difere legitimamente da captura histórica `:29`; o hash histórico é igual ao da cópia raiz conservada `:30`. Não há commit inventado: `:19,25,37,41`. | PASS |
| C4 | `ENTREGA_ATUAL.md:7,19–21,41,59,63–65` indica um único candidato, diferencia históricos e checkout alterado e não declara release validada. HEAD observado nesta revisão = de3f53920aafc49c2ca86eb93b186d5af8c19931 (commit documental); inspectionHead é f349f706… intencionalmente histórico. Status mantém 11 fontes rastreadas alteradas e não rastreados de produto; 167 hashes conferem apesar da mudança documental do HEAD. | PASS |
| C5 | `NEXT_STEPS.md:5–9`, `PLANO_EVOLUCAO.md:3,210–218` apontam documentos e mantêm E1 aguardando revisão/E2–E8 planejados. Test-Path de todos os links Markdown relativos desses cinco documentos: zero ausentes. E1 registra vínculo fonte/build sem atribuir provas funcionais antigas ao candidato (`ENTREGA_ATUAL.md:65`). | PASS |
| C6 | Recalculados TODOS os 167 SHA256 de `.checks/evolution-e1-build.json:9–677`: zero divergências. Output atual = 18238464 bytes, SHA256 06f9ef8f0bcd0381fc9d65e04e828b55c4a6711e777b43ab07c561f1b51df9fd, igual a `:682–683`. Manifesto `:679–681` registra exitCode 0/sourceDrift vazio. Log `:1–2,20,56–58` registra TypeScript/Vite, quatro warnings Rust, Finished release e Built application no caminho correto. Não reexecutado, por escopo autorizado: revisão independente dos bytes atuais e registro da execução. | PASS |

## Inventário recalculado

| Executável | Bytes | SHA256 |
|---|---:|---|
| Capy.exe | 18238464 | E0A1D0F3B1E6317B56D50FAEA77B10A4D46E1B135234F3250E7EE2787AC272C6 |
| releases/0.2.0-alpha.1/Capy.exe | 11293184 | 1198AA929E6D539200DC3C5F849CCE54EA739BBB6BBD8E1724239FEFA6EA8593 |
| releases/0.2.0-alpha.2/Capy.exe | 11423744 | 177076E818DEDD3C1AFEE1E7E81200C78A0A4E14DD4E6E5B7839A4E395F835AE |
| releases/0.4.0-alpha.1/Capy.exe | 11889664 | 7610F95B3DE8B9F041CACCF6A181F0398D8020750C042FCCC9A7AA1019BAEDD3 |
| releases/0.4.0-alpha.2/Capy.exe | 11889152 | 4F47B10027630A8CBBA0D0E671DA4C38661EA525EF1D290AA3BD2D8746CFA7BD |
| releases/0.4.0-alpha.3/Capy.exe | 18162176 | 53F6B578CD8A9F97BDB14B83D437A1CD104E859273E695B14E9342E175BD278D |
| src-tauri/target/release/capy.exe | 18238464 | 06F9EF8F0BCD0381FC9D65E04E828B55C4A6711E777B43AB07C561F1B51DF9FD |

## Gates e limites

Diff documental do range: `git diff --check f349f706…de3f539` passou, sem saída. O escopo staged documental foi registrado pelo orquestrador como aprovado; agora já está commitado. `git diff --check` global aponta apenas `src/quota-board.ts:142: new blank line at EOF`, preexistente e fora desta entrega; preservado.

Proveniência é conferência dos bytes inventariados e do registro de build; não reprodução hermética de toolchain/dependências, nem validação funcional/nativa do candidato. Captura anterior do target não pode ser reobtida após rebuild; sua correspondência com a cópia raiz foi recalculada. Provas históricas foram inspecionadas como documentos, não reexecutadas nem promovidas ao hash novo. Conversa vinculante não foi fornecida integralmente ao verificador; este relatório prova os seis checks materiais disponíveis, sem presumir decisões ausentes. Nenhuma lacuna bloqueante encontrada no escopo E1.
