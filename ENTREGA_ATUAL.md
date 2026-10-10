# Entrega atual — inventário e candidato local

Recompilação após QA em 09/10/2026. O candidato inclui o canal Claude experimental e a correção de logos bloqueados por CSP. O [QA adicional](.checks/evolution-e2-qa-results.md) e sua [revisão independente](.checks/evolution-e2-qa-review.md) registram as provas atuais. A [revisão E2 anterior](.checks/evolution-e2-review.md) e a [revisão E1](.checks/evolution-e1.verified.md) permanecem históricas. Referências: [plano](PLANO_EVOLUCAO.md) e [critérios E2](.checks/evolution-e2-claude.md).

## Candidato único para a próxima validação

Selecionado: **`src-tauri/target/release/capy.exe`**, recompilado nesta rodada. A seleção identifica o candidato de trabalho, não uma nova release funcionalmente validada.

| Campo | Valor atual |
| --- | --- |
| Versão Windows | `0.4.0-alpha.3` |
| Tamanho | 18439168 bytes |
| SHA-256 | `d9c73054082741aafd2fb022334c3fbbdd82660773cf8f10d6bfac35f0ad3165` |
| Comando | `npm run desktop:build`, executado por `rtk npm run desktop:build` |
| Resultado | Exit 0; TypeScript/Vite e Rust release compilados; seis warnings Rust |
| Fonte | Checkout alterado sobre `4290f30`; 170 entradas com SHA-256 |
| Correspondência | Manifesto anterior à build conferido novamente após a build; zero entradas alteradas |

Prova atual: [manifesto após QA](.checks/evolution-e2-qa-build.json), [log](.checks/evolution-e2-qa-build.log), [smoke nativo](.checks/evolution-e2-qa-smoke.json) e [interface real Claude](.checks/evolution-e2-qa-native-fixed.json). Zero entradas alteradas durante a build. O HEAD identifica a base Git, não um commit que contenha todas as alterações locais. O manifesto identifica os bytes das fontes inventariadas; não constitui pacote de reprodução completo de toolchain/dependências.

As provas anteriores e as bordas de desconexão/ausência/timeout usaram uma build debug separada com `intervention-proof`, hash `7ff2a235ee38bf01b67604d092193aa1e715f0c54012a8352a5fa03e82d866f0`. O QA adicional exerceu também os cartões da build release normal: permitir escrita e responder Alpha, com proveniência hook, logos carregados e zero erros observados após conexão CDP. Negação/pergunta foram exercitadas no release anterior. Claude foi lançado em modo print com host passivo; terminal interativo e demais lacunas do relatório permanecem abertos. O candidato normal não habilita o probe. Não foi criado instalador nem publicada release.

O candidato E2 anterior tinha hash `61b45972c37b1bac74b192e511fedd9e95d6c0bac5b655f860481c4ba3d32557`: [manifesto histórico](.checks/evolution-e2-build.json) e [log](.checks/evolution-e2-build.log). Sua prova funcional de interface precedeu a correção CSP.

O candidato E1 anterior tinha hash `06f9ef8f0bcd0381fc9d65e04e828b55c4a6711e777b43ab07c561f1b51df9fd`: [manifesto histórico](.checks/evolution-e1-build.json) e [log](.checks/evolution-e1-build.log).

`Capy.exe` na raiz permanece a cópia anterior e **não é o candidato atual**. Todos os executáveis em `releases/` também diferem do novo candidato. As notas de alpha.3 permanecem históricas e não comprovam a build nova, apesar da mesma versão declarada.

## Inventário dos executáveis anterior ao rebuild

Valores obtidos com `Get-Item` (Length e VersionInfo) e `Get-FileHash -Algorithm SHA256`, sem executar ou substituir os arquivos. Esta captura é histórica após qualquer rebuild e não deve ser tratada como hash atual do arquivo recompilado. FileVersion e ProductVersion retornaram o mesmo valor para cada linha abaixo; são metadados do executável, não prova de commit de build.

| Caminho relativo | FileVersion / ProductVersion | Tamanho (bytes) | SHA-256 |
| --- | --- | ---: | --- |
| `src-tauri/target/release/capy.exe` | `0.4.0-alpha.3` | 18238464 | `E0A1D0F3B1E6317B56D50FAEA77B10A4D46E1B135234F3250E7EE2787AC272C6` |
| `Capy.exe` | `0.4.0-alpha.3` | 18238464 | `E0A1D0F3B1E6317B56D50FAEA77B10A4D46E1B135234F3250E7EE2787AC272C6` |
| `releases/0.2.0-alpha.1/Capy.exe` | `0.2.0-alpha.1` | 11293184 | `1198AA929E6D539200DC3C5F849CCE54EA739BBB6BBD8E1724239FEFA6EA8593` |
| `releases/0.2.0-alpha.2/Capy.exe` | `0.2.0-alpha.2` | 11423744 | `177076E818DEDD3C1AFEE1E7E81200C78A0A4E14DD4E6E5B7839A4E395F835AE` |
| `releases/0.4.0-alpha.1/Capy.exe` | `0.4.0-alpha.1` | 11889664 | `7610F95B3DE8B9F041CACCF6A181F0398D8020750C042FCCC9A7AA1019BAEDD3` |
| `releases/0.4.0-alpha.2/Capy.exe` | `0.4.0-alpha.2` | 11889152 | `4F47B10027630A8CBBA0D0E671DA4C38661EA525EF1D290AA3BD2D8746CFA7BD` |
| `releases/0.4.0-alpha.3/Capy.exe` | `0.4.0-alpha.3` | 18162176 | `53F6B578CD8A9F97BDB14B83D437A1CD104E859273E695B14E9342E175BD278D` |

Versão declarada na fonte inspecionada: `0.4.0-alpha.3` em `package.json`, `src-tauri/Cargo.toml` e `src-tauri/tauri.conf.json`. Sua coincidência com os recursos Windows não estabelece correspondência fonte/binário.

## Estado da fonte na inspeção

HEAD observado por `rtk git rev-parse HEAD`: `f349f706b57063d6bc53510724046537797508bb`. **Este é o HEAD da inspeção, não o commit de build.**

Antes da criação deste documento, `rtk git diff --stat` registrou 11 arquivos rastreados alterados, 752 inserções e 39 exclusões:

| Arquivo | Linhas afetadas no diff --stat |
| --- | ---: |
| `panel.html` | 27 |
| `prototypes/preview.css` | 14 |
| `prototypes/preview.js` | 2 |
| `src-tauri/src/discovery.rs` | 23 |
| `src-tauri/src/main.rs` | 44 |
| `src-tauri/tauri.conf.json` | 2 |
| `src/dashboard.ts` | 62 |
| `src/presentation.ts` | 9 |
| `src/quota-board.ts` | 58 |
| `src/style.css` | 460 |
| `summary.html` | 90 |

`rtk git status --short` também mostrou não rastreados: `.checks/evolution-e1.md`, `Capy.exe`, `PLANO_EVOLUCAO.md`, `assets/logos/`, `prototypes/logos/` e `prototypes/showcase-summary.html`. A captura antecede este documento e pode anteceder edições documentais concorrentes de E1. Os três primeiros incluem artefato existente e documentos desta preparação; a listagem não atribui autoria às alterações. Nenhum desses arquivos foi descartado por esta tarefa.

## Próximo gate: validação funcional do candidato

O vínculo entre as fontes inventariadas e a compilação foi registrado nesta rodada. Nenhum arquivo de produção foi editado por E1; as alterações de produto já presentes no checkout foram compiladas. Nenhuma instância Capy estava aberta antes da compilação; não foi necessário encerrar ou reabrir um aplicativo.

O próximo gate de E2–E5 é executar os cenários pertinentes neste hash e vincular as novas evidências a ele. E1 não executou suítes de aplicação, smoke nativo ou chamadas de IA. Não houve promoção de pacote histórico, instalador ou publicação; somente o executável de trabalho foi recompilado. Evidências anteriores mantêm o alcance e o artefato originalmente exercitados.

## Próximos passos de distribuição — E5

- Criar comando NSIS: `desktop:build` contém `--no-bundle` e hoje gera somente executável local. Configurar instalação por usuário e WebView2.
- Migrar hooks de caminhos anteriores e incluir bridge/statusline de cotas no lifecycle por perfil. Atualização deve preservar dados; remoção deve restaurar somente configurações ainda pertencentes à Capy, preservando alterações externas e hooks alheios.
- Resolver a rejeição de caminhos com espaços em `scripts/antigravity-hooks.ps1` antes de definir o caminho instalado; validar também acentos. Provar instalação, atualização e desinstalação no pacote exato, com hooks e statusline habilitados.
