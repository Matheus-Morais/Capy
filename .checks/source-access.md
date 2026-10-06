# Acesso à origem — etapa 0.3.0 aberta

Profile: light. Base: 839e6a2.

Sources: conversa (acesso à sessão correta), `.design/capy.md` (Sessões), PRODUCT.md, DESIGN.md e `.impeccable/surfaces/src-style-css.md` (interface existente), https://learn.chatgpt.com/docs/reference/commands (link codex://threads/ID), Microsoft ShellExecuteW e AssocQueryStringW.

## Landing

Uma ação opcional na Session identifica acesso disponível. O backend gera o destino por ID validado, nunca recebe uma URL da interface. O comando revalida a descoberta antes de entregar o link ao Windows. Contrato compartilhado pelo resumo e painel.

| Door | Literal shape | Alternative rejected |
| --- | --- | --- |
| Acesso Codex | codex://threads/UUID via associação do Windows; source_action opcional com disponibilidade e motivo | retomar uma CLI concorrente não prova acesso à janela original |
| Registro Windows | consultar executável ou AppID empacotado; AppX não precisa expor executável à associação | consulta somente por executável rejeita a instalação real OpenAI.Codex |

## Checks do slice Codex

**C1** — ação envia exatamente codex://threads/UUID da sessão real presente, sem prompt, caminho, criação ou retomada.
Proof: Rust `source_access::tests::opens_only_the_current_codex_identity`.

**C2** — ID malformado, sessão encerrada, projeto alterado, demonstração ou agente sem destino comprovado rejeitam a abertura antes do dispatcher.
Proof: Rust `source_access::tests::stale_or_unsupported_requests_never_dispatch`.

**C3** — associação por executável ou AppID empacotado é reconhecida; ausente fica indisponível, erro do Windows é retornado, sem simular sucesso.
Proof: Rust `source_access::tests::missing_handler_and_dispatch_errors_are_visible`.

**C4** — card real mostra a ação somente quando o backend a fornece; indisponível tem motivo visível; escapa conteúdo; demonstração preserva terminal simulado.
Proof: JS `source access is driven by backend availability and escapes its reason`.

## Swept

- Validation: C1/C2.
- Failure/dependency: C3, verificação do protocolo registrado.
- Idempotency: abrir destino existente não envia mensagem; interface desabilita clique durante chamada.
- Authorization: somente identidade real na descoberta; não aceitar URLs, comandos ou caminhos externos.
- Concurrency: revalida sessão e projeto antes da abertura; fechamento posterior ainda é uma corrida do aplicativo de destino.
- Lifecycle: não persiste destinos; deriva por ciclo da descoberta.
- Transitions: C3, erro permanece erro.
- Observability: motivo e erro no card/aviso existente; sem conteúdo de conversa em logs.

## Handoff

Slice inteiro no principal; verificador independente após commit do slice. Etapa inteira NÃO se fecha com esse slice.

## Evidência ainda exigida pela etapa completa

- Abrir a conversa própria no app e comprovar seleção do ID correto, sem nova conversa.
- Destinos de Claude e Antigravity para alcançar a sessão original, com prova real própria por agente.
- Release local 0.3.0 somente após fechar toda a etapa; 0.2.0 permanece aberta no checklist Antigravity.

## Execução

- 32 testes Rust e 8 testes JS passaram, nenhum ignorado; build TypeScript/Vite passou.
- Descoberta real confirmou source_action.available=true nesta instalação AppX após consultar AppID; consulta somente por executável falhava.
- Layout do renderer confirmado no Chrome em fixture sintética de 380px com ação disponível e indisponível; imagem privada em scratch/source-access-preview.png. Não comprova a seleção de uma conversa no aplicativo nativo.
- Detector rodou uma vez; avisos de tokens/paleta são anteriores à edição. O novo motivo usa cor, tamanho e espaçamento existentes. A sidecar de DESIGN.md já estava desatualizada; não foi alterada.
- Prévia 0.2.0-alpha.1 reunirá o trabalho em andamento; não substitui as releases finais por etapa nem fecha as provas pendentes.
