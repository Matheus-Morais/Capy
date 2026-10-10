# Plano de evolução da Capy — excelência no Windows

Data: 10/10/2026. Status: E1 concluída e verificada; fatia de respostas Claude E2 implementada experimentalmente na fonte, com QA real adicional em debug e GUI release; E2 integral e E3–E8 seguem abertas. A build corrigida passou os checks GUI e smoke da rodada. Estado reconciliado em [CAPACIDADES.md](CAPACIDADES.md), [resultados E2](.checks/evolution-e2-qa-results.md) e artefato/candidato em [ENTREGA_ATUAL.md](ENTREGA_ATUAL.md).

## 1. Objetivo e decisões

Entregar uma Capy que outra pessoa consiga instalar, configurar e usar diariamente no Windows para acompanhar agentes, responder a solicitações, consultar cotas e continuar trabalhos próprios.

Decisões confirmadas pelo usuário: **excelência no Windows** e **consolidar antes de expandir**. O trabalho tem dois marcos:

- **Marco A — consolidar:** concluir a experiência suportada com Claude Code, Codex e Antigravity e entregar um instalador validado.
- **Marco B — expandir:** acrescentar atividade detalhada, Cursor, Gemini CLI e acompanhamento do GitHub.

Manter Rust/Tauri, TypeScript, a identidade da capivara e as três superfícies existentes: mascote, resumo e painel. Reaproveitar integrações e proteções já implementadas.

A referência é a [experiência do Coucou no Windows](https://github.com/Louis-CFM/coucou/blob/main/windows/README.md), consultada em 09/10/2026. O objetivo é qualidade de uso comparável no recorte escolhido; a análise documental não equivale a uma validação prática do concorrente nem promete paridade integral de funcionalidades.

Fora deste roteiro: celular, macOS, Linux, música, serviços financeiros, modelos locais, expansão do catálogo de roupas e ações de escrita no GitHub. Publicação remota, atualização automática e assinatura comercial do executável não são requisitos do primeiro instalador.

## 2. Base observada e evidências

| Área | Situação observada | Trabalho restante |
| --- | --- | --- |
| Claude e Codex | Descoberta e atividade implementadas | Fechar lacunas da auditoria na entrega selecionada |
| Intervenções Codex | Respostas por pedido exato já existem | Validar integração atual, expiração, desconexão e abertura da origem |
| Intervenções Claude | Canal experimental; 9 Rust targeted, 47 JS e fixture settings PASS; QA adicional provou allow/arquivo exato, desconexão/ausência e timeout real de 110 s no debug, e allow/deny/pergunta Alpha na GUI nativa do release corrigido | Perguntas/negação interativas, cancelamento/reinício reais; revalidar Codex e concluir Antigravity |
| Antigravity CLI | Descoberta por lock/metadados e atividade por hooks existem | Concluir prova interrompida por 429 e QA nativo |
| Antigravity IDE | Prova de presença/identidade permanece pendente | Investigar e validar separadamente do CLI |
| Cotas Antigravity | Coleta inicial/periódica da conta/export configurados já tem prova independente | Preservar a entrega e validar outras configurações quando disponíveis |
| Cotas Claude/Codex | Adaptadores existentes; Codex real já observado | Fechar statusline Claude, contas alternativas e limites de amostragem |
| Chat e continuidade | Chat próprio Claude/API, terminal, recuperação e transferência implementados | Consolidar provas reais; não pressupor retomada de sessões externas |
| Mascote e interface | Animações, skins, acessórios, resumo e painel existentes | Validar comportamento nativo, acessibilidade e consistência |
| Distribuição | Executáveis alpha e configuração NSIS existentes | Criar comando de empacotamento, lifecycle e validação de instalação |

Fontes locais: [retomada](NEXT_STEPS.md), [auditoria multifuncional](.checks/multifunction.final-audit.md), [prova Antigravity](.checks/antigravity.md) e [coleta automática de cotas Antigravity](.checks/agy-automatic.verified.md).

A revisão deste plano foi somente de leitura, com dois subagentes `gpt-6.1-sol`. Não foram reexecutados testes, builds ou provas reais para preparar o documento. Resultados anteriores mantêm seu alcance e sua versão original.

## 3. Etapas: o que fazer, como fazer e como concluir

### E1 — Reconciliar código, documentação e entrega

**Resultado:** uma base identificável para desenvolvimento e validação.

**Como fazer:**

- Inventariar alterações locais e separar o trabalho existente das mudanças deste roteiro, sem descartar arquivos do usuário.
- Montar matriz por agente: descoberta, atividade, aprovações, perguntas, origem, cotas, chat e tarefas. Classificar cada capacidade como validada, experimental, indisponível ou pendente de prova.
- Reconciliar README, roteiro de retomada e notas de versão. Separar cotas Antigravity, sessões CLI e sessões IDE.
- Escolher um artefato oficial por entrega e registrar versão, commit e SHA-256. Uma build de checkout alterado deve registrar também essa condição e a revisão das alterações utilizadas.
- Corrigir a ambiguidade entre executável de desenvolvimento, cópia na raiz e pacotes alpha. Não substituir uma evidência histórica por alegação de prova na build atual.

**Conclusão:** cada capacidade anunciada aponta para evidência ou limitação explícita; documentação e artefato selecionado descrevem a mesma entrega.

### E2 — Concluir sessões e intervenções

**Dependência:** E1. **Resultado:** agente trabalha, pede atenção, recebe a resposta correta e continua. Estado parcial: o canal Claude para PermissionRequest e AskUserQuestion foi implementado experimentalmente na fonte. 9 testes Rust `claude_intervention`, 47 testes JS e fixture de `verify-claude-settings` passaram. A prova print anterior demonstrou deny exato, rejeição de contexto alterado/replay, resposta `Alpha` e continuação via permission host passivo (sem `control_response`). QA adicional provou allow/arquivo exato, desconexão/ausência e timeout real de 110 s em build debug, além de allow/deny e pergunta Alpha pela GUI nativa. A build com correção CSP passou 10 checks GUI e smoke nativo de 20 checks. A prova debug não certifica o release. A rodada de [sessões](.checks/evolution-e2-sessions-results.md) passou 16 checks com duas sessões Write simultâneas e 10 checks de aprovação Write interativa em ConPTY. Perguntas/negação interativas e cancelamento/reinício reais seguem sem prova. Consulte [resultados E2](.checks/evolution-e2-qa-results.md), [.checks/evolution-e2-claude.md](.checks/evolution-e2-claude.md) e [.checks/evolution-e2-live.md](.checks/evolution-e2-live.md).

**Como fazer:**

- Declarar capacidades por agente e conexão, reaproveitando descoberta, snapshots e apresentação atuais.
- Revalidar respostas Codex: permissões, perguntas, expiração, desconexão e identidade exata do pedido.
- Completar a prova isolada Claude: `allow`, desconexão/ausência, timeout real de 110 s e GUI allow/deny/pergunta têm amostras registradas. Duas sessões Write concorrentes e aprovação Write no terminal interativo agora têm amostras reais. Ainda provar perguntas/negação interativas e cancelamento/reinício reais. A amostra debug não certifica o release nem os casos não exercitados.
- Manter respostas Claude opt-in por `Enable -Responses`, timeout de 120 s e marker dedicado; `Enable` sem essa opção e `Disable` removem o canal de resposta, preservando hooks alheios. Se o canal falhar ou não estiver disponível, orientar resposta na origem.
- Concluir a prova de Antigravity CLI. Investigar a IDE separadamente; lock, histórico, processo e quota não são evidências intercambiáveis de presença.
- Validar abertura da origem correta. Exibir indisponibilidade quando não for possível selecionar a sessão exata, sem anunciar sucesso apenas porque um dispatcher aceitou a chamada.
- Preservar estados desconhecidos e evidência vencida. Não inferir conclusão de `idle`, `Stop`, silêncio ou desaparecimento da sessão.

**Conclusão:** duas sessões do mesmo agente não recebem respostas cruzadas; pedido respondido, expirado ou alterado não é reutilizado; uma conexão perdida produz estado explícito e não reenvio automático. Fechar a Capy permite que o fluxo de aprovação prossiga na origem conforme o contrato do agente.

**Limite:** suporte completo de todos os agentes a todas as ações não é presumido. Uma restrição comprovada da fonte deve constar da matriz de suporte; não pode ser rotulada como função entregue.

### E3 — Consolidar contas, cotas e continuidade

**Dependência:** E1; validação conjunta com E2. **Resultado:** informação de conta confiável e continuidade dos trabalhos próprios.

**Como fazer:**

- Validar statusline Claude com conta real, preservando configurações e hooks alheios. Provar habilitação, renovação, remoção e configuração alterada externamente.
- Comparar cotas à fonte da conta correspondente, incluindo reset e indisponibilidade. Preservar a coleta automática Antigravity já comprovada; ampliar sua amostragem apenas quando necessário.
- Exercitar alertas existentes sem duplicar notificações da mesma conta/janela e sem aproveitar amostras vencidas.
- Consolidar chat Claude por assinatura, tarefas, terminal integrado/externo, interrupção, saída, recuperação e transferência já implementados.
- Preservar revisão do contexto, identidade e mudança de cobrança. Manter envio incerto sem reenvio automático.
- Separar provas de contratos API, envio HTTPS real e recuperação após falha real. Uma simulação de perda de registro não comprova uma queda de rede durante geração.

**Conclusão:** cada percentual corresponde a uma conta identificada; informação vencida deixa de parecer atual; transferência executa somente o destino revisado; reinício preserva a indicação de resultado incerto.

**Limite:** continuidade aqui significa chat/tarefa próprios suportados. Retomada de sessões externas Codex/Antigravity não é oferecida pelos contratos atuais e fica fora desta entrega. Segunda conta e APIs reais dependem de credenciais disponíveis, sem exigir contratação para concluir funções por assinatura.

### E4 — Fechar a experiência de configuração e uso

**Dependências:** E2 e E3. **Resultado:** primeira sessão utilizável sem edição manual de JSON.

**Como fazer:**

- Acrescentar configuração inicial com detecção dos agentes, estado da conexão, capacidades disponíveis e orientação para pendências.
- Levar habilitação/remoção das integrações à interface, mostrando alterações previstas e preservando backups e validações existentes.
- Organizar resumo com solicitações pendentes primeiro, depois sessões e cotas. Deixar detalhes extensos no painel.
- Diferenciar carregamento, nenhuma sessão, fonte indisponível, autenticação necessária e erro; preservar foco, seleção e rolagem durante atualizações.
- Validar teclado, foco visível, movimento reduzido, sons, arraste, skins e restauração entre monitores. Preservar a arte aprovada.
- Fazer a demonstração apresentar o fluxo completo, com dados simulados identificados e sem alterar sessões reais.

**Conclusão:** em perfil novo, instalar/conectar um agente e chegar à primeira sessão seguindo a interface e o guia. Pedidos continuam acessíveis quando a animação termina; atualização de dados não perde a seleção do usuário.

### E5 — Entregar o Marco A instalável

**Dependências:** E1–E4. **Resultado:** instalação, atualização manual e remoção reproduzíveis.

**Como fazer:**

- Criar comando específico de empacotamento NSIS. `desktop:build` usa `--no-bundle` e não produz instalador hoje.
- Configurar instalação por usuário e tratamento do WebView2; instalar executável e auxiliares em caminhos estáveis, independentes do checkout.
- Resolver caminhos com espaços antes de escolher a localização final: o script Antigravity atual os rejeita. Validar também usuário/caminho com acentos.
- Inventariar integrações pertencentes à Capy, inclusive por perfil. Migrar hooks que apontam para executáveis antigos: igualdade do comando absoluto não basta para reconhecer a instalação anterior.
- Tratar hooks Claude, hooks Antigravity e bridge/statusline de cotas no mesmo lifecycle. Restaurar configurações somente quando ainda pertencerem à Capy; preservar alterações externas e hooks alheios.
- Validar instalação limpa, reinstalação, atualização e desinstalação. Preservar dados pessoais na remoção; remover integrações executáveis próprias ou apresentar pendências concretas quando a configuração externa tiver mudado.
- Gerar checksum, notas de entrega e matriz de suporte; automatizar checks e geração de artefatos. Registrar claramente a condição de assinatura do pacote.

**Conclusão:** instalar em Windows limpo, conectar um agente, receber uma solicitação, reiniciar e atualizar preservando os dados. Desinstalar sem deixar integrações próprias conhecidas apontando para arquivos removidos. Toda evidência deve identificar o pacote exato exercitado.

### E6 — Mostrar atividade detalhada

**Dependência:** Marco A. **Resultado:** entender as ações observadas sem abrir cada terminal.

**Como fazer:**

- Começar por Claude. Definir a origem e a semântica de leitura, edição e execução a partir de eventos efetivamente fornecidos, distinguindo sessões externas de trabalhos próprios.
- Estender o contrato atual com eventos por sessão e detalhes de edição. Abrir diff somente quando a fonte fornecer dados suficientes; uma comparação genérica do diretório não atribui autoria ao agente.
- Adotar como limites iniciais 200 eventos e 20 diffs por sessão, com até 200 KiB por diff. Exibir metadados e indicação de truncamento quando exceder o limite.
- Manter esse conteúdo em memória e descartá-lo ao encerrar a sessão ou a Capy. Não acrescentar arquivo persistente de comandos/diffs nesta etapa.

**Conclusão:** edição observada aparece na sessão correta e abre seu diff; truncamento é visível; fonte sem detalhes continua utilizável; nenhum evento genérico provoca celebração de conclusão.

### E7 — Adicionar Cursor e Gemini CLI

**Dependências:** Marco A e contratos de E6. **Resultado:** ampliar cobertura usando as mesmas superfícies.

**Como fazer:**

- Implementar Cursor primeiro e Gemini CLI depois. Registrar documentação oficial, versões e contratos exercitados.
- Mapear eventos para identidade, presença e atividade comuns, reutilizando configuração, diagnóstico e apresentação.
- Entregar inicialmente presença e atividade. Habilitar perguntas/aprovações apenas depois de demonstrar canal de resposta e identidade do pedido.
- Exibir limitações específicas e acesso à origem quando a fonte não oferecer uma operação.

**Conclusão:** sessões simultâneas têm projeto e identidade corretos; desconexão remove a validade da evidência; cada controle corresponde à capacidade demonstrada da conexão.

### E8 — Acompanhar GitHub

**Dependências:** Marco A e E6; pode ocorrer sem esperar todos os agentes de E7. **Resultado:** consultar PRs, revisões solicitadas e CI dos repositórios escolhidos.

**Como fazer:**

- Criar conexão opcional com token informado nas configurações, armazenado no cofre Windows; consultar os contratos oficiais antes de implementar.
- Permitir seleção explícita de repositórios. Consultar em segundo plano a cada dois minutos, com atualização manual e respeito aos limites retornados pelo serviço.
- Mostrar última atualização e distinguir resultado vazio, autenticação inválida, indisponibilidade e limite de consultas.
- Abrir PRs e execuções no navegador. Não adicionar merge, comentário ou reexecução de CI nesta entrega.

**Conclusão:** PR e execução aparecem com links corretos; revogar token produz erro de conexão sem apagar preferências; nenhum segredo aparece em configuração ou diagnóstico.

## 4. Contratos, compatibilidade e operação

- Estender a ponte Rust/TypeScript com capacidades por agente, disponibilidade da conexão e eventos vinculados à sessão. Dados GitHub permanecem separados das sessões de agentes.
- Reutilizar os mecanismos existentes de nonce, revisão e revalidação. A generalização para Claude não pode remover as garantias atuais de Codex.
- Manter credenciais e chamadas externas no backend Rust. Diagnósticos devem permitir investigar conexão e versão sem expor chaves ou conteúdo completo das conversas.
- Versionar alterações persistentes e manter leitura dos dados anteriores. Arquivo inválido ou alterado externamente deve ser preservado e gerar erro explícito.
- Tratar eventos duplicados/fora de ordem por identidade do pedido/evento; não depender da ordem de chegada para autorizar ações.
- Falhas de leitura podem ser recuperadas nas consultas seguintes; envios com resultado incerto não são repetidos automaticamente.
- Registrar a versão dos agentes exercitada. Uma atualização incompatível deve produzir indisponibilidade reconhecível, nunca confirmação fictícia.

## 5. Validação e critérios de entrega

| Grupo | Cenários obrigatórios |
| --- | --- |
| Identidade | Duas sessões do mesmo agente, projetos distintos, sessão encerrada e identificador obsoleto |
| Intervenções | Permitir, negar, responder, clique duplicado, expiração, reconexão e perda de conexão após envio |
| Cotas | Conta diferente, reset, informação vencida, indisponibilidade e alerta repetido |
| Continuidade | Interrupção, reinício, envio incerto, transferência cancelada e mudança de cobrança |
| Configuração | JSON inválido, hooks alheios, habilitação repetida, migração de caminho e edição externa |
| Interface | Vazia, carregando, com erro, muitas sessões, navegação por teclado e movimento reduzido |
| Windows | Escalas 100%/150%/200%, segundo monitor removido, bandeja, paths com espaços/acentos |
| Instalador | Instalação limpa, reinstalação, atualização e remoção com hooks e statusline habilitados |
| Expansão | Evento duplicado/fora de ordem, diff truncado, token revogado e limite de consultas |

Usar testes JavaScript/Rust para contratos e regressões; runners nativos para integração com janelas; sessões próprias para provas reais de agentes. Simulação, prova de contrato e integração real devem aparecer separadamente no relatório.

Comandos existentes: `npm test`, `cargo test --manifest-path src-tauri/Cargo.toml`, `npm run build`, `npm run desktop:build` e `npm run verify:native`. Usar RTK quando adequado. Executar checks relevantes por mudança e os gates do marco; não repetir consumo de provedor sem necessidade.

Antes de recompilar ou testar o nativo, conferir o caminho do processo e encerrar somente a instância própria prevista pelo runner. Não usar conversas externas do usuário como fixtures.

Cada conclusão deve apontar para comando, resultado, versão/artefato, cenário exercitado e limitação de amostragem. Handler registrado, fixture aprovada ou screenshot isolado não prova o fluxo completo.

### Cobertura das obrigações transversais

| Dimensão | Tratamento |
| --- | --- |
| Validação | E2: identidade/pedido; E3: conta/fonte; E5: propriedade das configurações |
| Falhas | E2: desconexão; E3: envio incerto; E4: estado de erro acionável |
| Idempotência e repetição | Proteções existentes preservadas; E2 valida clique duplicado; E5 valida reinstalação |
| Autorização | Capacidade real e revisão existentes; GitHub somente leitura em E8 |
| Concorrência e ordem | Pedidos revalidados, eventos identificados e configurações externas preservadas |
| Ciclo de dados | E3: histórico; E5: migração/remoção; E6: conteúdo transitório limitado |
| Dependências externas | Matriz por versão e funções experimentais quando sem prova |
| Transições de estado | E2/E6 distinguem espera, atividade, desconhecido e conclusão explícita |
| Observabilidade | E1 vincula evidência à entrega; E4/E8 mostram erro e atualização; diagnósticos sem segredos |

## 6. Acompanhamento, dependências e execução com subagentes

| Item | Estado atual | Evidência de conclusão |
| --- | --- | --- |
| E1 — Base reconciliada | Concluída: documentação/proveniência; revisão independente C1–C6 PASS | [.checks/evolution-e1.verified.md](.checks/evolution-e1.verified.md) |
| E2 — Sessões/intervenções | Parcial: canal Claude experimental; E2 integral aberta | [Critérios](.checks/evolution-e2-claude.md), [amostra real parcial](.checks/evolution-e2-live.md), [revisão](.checks/evolution-e2-review.md) |
| E3 — Cotas/continuidade | Planejado; reaproveita implementação e provas existentes | Pendente |
| E4 — Experiência | Planejado | Pendente |
| E5 — Instalador / Marco A | Planejado | Pendente |
| E6 — Atividade detalhada | Planejado | Pendente |
| E7 — Cursor/Gemini | Planejado | Pendente |
| E8 — GitHub / Marco B | Planejado | Pendente |

Ordem: **E1 → E2/E3 → E4 → E5 → E6 → E7/E8**. E2 continua aberta apesar da fatia Claude já implementada: faltam completar a prova Claude real e QA nativo, revalidar Codex e concluir Antigravity CLI/IDE. O candidato E2 foi recompilado, com proveniência em ENTREGA_ATUAL; não há nova release empacotada/publicada. O paralelismo exige arquivos e responsabilidades separados; dependências de integração continuam sequenciais.

O agente principal atua como orquestrador. Usar subagentes `gpt-6.1-sol`, conforme solicitado, com contexto mínimo, escopo delimitado, arquivos sob responsabilidade e critério de entrega. Não duplicar pesquisas nem permitir edições concorrentes no mesmo arquivo. Revisão independente deve concentrar-se nas mudanças e evidências relevantes; testes nativos que compartilhem processos/configurações devem ser sequenciais.

### Dependências externas e decisões técnicas a comprovar

| Dependência | Impacto | Comportamento até resolver |
| --- | --- | --- |
| Canal Claude para cada tipo de intervenção | Bloqueia habilitar esse tipo de resposta em E2 | Responder na origem; não anunciar suporte |
| Fonte confiável Antigravity IDE | Bloqueia declarar presença/atividade IDE validada | Manter experimental, distinguindo CLI e cotas |
| Segunda conta e credenciais API | Bloqueia provas reais correspondentes de E3 | Preservar resultado local e limitação; não exigir contratação |
| Acesso Cursor/Gemini | Bloqueia prova real do adaptador correspondente | Não marcar integração validada com fixtures |
| Token/repositório GitHub | Bloqueia prova real de E8 | Manter conexão opcional e registrar pendência |

Os limites numéricos de E6 e o intervalo de E8 são defaults propostos por este plano. Não são requisitos herdados nem medições de desempenho. Protocolos externos ainda não comprovados exigem a etapa de investigação indicada; o implementador não deve inventar uma API para fechar o checklist.

Ausência de credencial bloqueia somente a prova dependente. Não marcar o marco integral como concluído enquanto uma obrigação incluída estiver aberta; é possível entregar uma prévia com alcance explícito. Obrigações antigas fora deste recorte continuam pendentes em seus documentos originais.

O roteiro termina quando os dois marcos estiverem entregues com evidências, suporte documentado e limitações identificadas. Publicação remota permanece uma ação posterior à preparação dos artefatos.
