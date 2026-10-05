# Capy — companheiro de trabalho com IA no Windows

> Status: draft — identidade e intenção recuperadas; experiência e desenho técnico em descoberta.
> Este documento registra decisões existentes e propostas para discussão. Os itens abertos ainda não são um plano de implementação.

## Situation

- Produto novo, ainda não publicado, pensado pelo usuário na conversa do Claude Code de 2026-10-05 e retomado nesta sessão.
- Existe um protótipo visual de capivara aprovado pelo usuário com “Gostei”. Uma cópia está em `../prototypes/capivara.html`, relativa a este documento.
- O nome Capy e a capivara como mascote padrão foram definidos nesta conversa.
- O desenvolvimento do Token-Watch está em andamento em outra sessão; este documento aproveita suas ideias como referência.
- Stack escolhida pelo usuário em 2026-10-05: Rust + Tauri 2. Interface em TypeScript, HTML/CSS e mascote SVG/CSS, conforme proposta aceita.

## Problem

Quem trabalha com várias sessões de agentes precisa alternar entre terminais para descobrir quem está trabalhando, quem terminou, quem precisa de resposta e quais contas ainda têm cota.

A intenção registrada é reunir essas informações e ações em um produto com identidade própria. Ainda não medimos o tempo perdido nem a frequência de sessões bloqueadas. A primeira hipótese a validar é que os pedidos de intervenção são o momento de maior valor para o Capy.

## Success

- Funcionou se o usuário consegue identificar a sessão que precisa dele e responder ou chegar ao terminal correto pelo Capy.
- Sinal de problema: o usuário continua conferindo todos os terminais, recebe avisos repetidos ou deixa de confiar no estado mostrado.
- Revisão: após usar um protótipo com mais de uma sessão real, verificar esses comportamentos com o usuário.

## Boundary

Nesta rodada: promessa do produto, presença do mascote, jornada de atenção e relação entre sessões e cotas.

A stack Rust + Tauri está decidida. A implementação e os contratos de integração serão planejados a partir das decisões de experiência e das provas de viabilidade.

## Shape

Capy acompanha o trabalho de agentes no Windows com uma capivara animada e uma visão das sessões e das cotas. A experiência permite perceber que algo precisa de atenção, identificar o projeto e a sessão, e executar a ação pertinente. Clicar no mascote abre um resumo compacto, com acesso ao painel completo. A primeira experiência será focada em agentes que precisam de intervenção.

## Key decisions

1. **Capy será um produto novo.** A conversa original explicitou criar um novo projeto sem alterar o Token-Watch e o Terminal-Hub.
2. **Uma única capivara animada representa todas as sessões, em posição arrastável, com um resumo compacto ao clicar.** O usuário escolheu o animal, aprovou o protótipo anterior, definiu o nome Capy e escolheu poder posicionar o mascote na tela. As animações acontecem junto à posição escolhida. O resumo discrimina as sessões e oferece acesso ao painel completo.
3. **A intenção reúne três capacidades.** Sessões persistentes e acesso aos terminais, acompanhamento de cotas por conta e acompanhamento/interação com o estado dos agentes.
4. **O protótipo aprovado é a referência visual inicial; a pose de frente é o padrão escolhido.** Ele contém seis estados: parado/passeando, trabalhando, precisa de você, terminou, cota acabando e dormindo. Ele é uma exploração visual; não confirma que os estados estejam integrados a sessões reais.
5. **Rust + Tauri 2 são a stack escolhida.** O usuário confirmou essa opção nesta conversa. A interface usará TypeScript + HTML/CSS e a capivara SVG/CSS. Persistência, framework de interface e bibliotecas de terminal permanecem em aberto.
6. **Capy acompanha também sessões iniciadas fora dele.** O usuário confirmou essa abrangência nesta conversa. A descoberta e o acompanhamento de sessões externas pertencem à primeira versão; iniciar pelo Capy é uma opção de uso.
7. **As sessões dos agentes suportados são descobertas automaticamente, com opção de ocultar.** O usuário confirmou essa experiência nesta conversa. A descoberta deve permitir ao usuário reconhecer o projeto e o agente de cada sessão.
8. **A primeira versão acompanha Claude Code, Codex e Antigravity.** O usuário confirmou que nenhum outro agente é indispensável inicialmente. Os estados observáveis e as ações disponíveis serão demonstrados por integração antes de virarem promessas do produto.

## Work

| Slice | Entrega | Status |
|---|---|---|
| [Atenção](#atenção) | Identificação e tratamento de uma sessão que precisa do usuário | open |
| [Sessões](#sessões) | Relação entre projeto, agente, terminal e conversa persistente | open |
| [Cotas](#cotas) | Recursos disponíveis por provedor e conta | open |
| [Mascote](#mascote) | Capivara com estados compreensíveis e presença discreta | design |

Ordem proposta da descoberta: Atenção → Sessões → Cotas → Mascote integrado. A exploração visual do mascote já existe e pode continuar em paralelo.

### Atenção

**Delivers:** uma sessão esperando intervenção fica identificável e acessível. **Status: open.**

| Situação | Resultado proposto |
|---|---|
| Agente espera permissão | Mostrar projeto, sessão e ação solicitada antes de aprovar ou negar |
| Agente espera uma resposta | Mostrar a pergunta; oferecer acesso ao terminal quando o provedor não permitir responder pelo Capy |
| Várias sessões precisam do usuário | Mostrar o total e permitir escolher qual tratar |
| Pedido deixou de ser válido | Atualizar a apresentação e impedir uma resposta ao pedido antigo |
| Estado não pode ser confirmado | Informar que o estado é desconhecido; não apresentar a sessão como concluída |

Em aberto: quais ações poderão ser respondidas diretamente nos agentes da decisão 8. Uma prova de viabilidade deve verificar a leitura do estado e a entrega da resposta numa sessão real, incluindo um pedido que expira antes da resposta.

### Sessões

**Delivers:** o usuário reconhece e acessa cada conversa pelo seu projeto e agente. **Status: open.**

| Situação | Resultado proposto |
|---|---|
| Sessão externa descoberta automaticamente | Identificar projeto, agente e origem; mostrar quais informações e ações estão disponíveis |
| Mesma sessão encontrada por mais de uma origem | Apresentar uma única sessão, sem duplicar avisos |
| Usuário oculta uma sessão | Retirar a sessão da apresentação principal e permitir restaurá-la; o trabalho segue no terminal de origem |
| Sessão trabalhando | Identificar projeto e agente, com acesso ao terminal correspondente |
| Sessão finalizada | Mostrar a conclusão até que o usuário a reconheça |
| Sessão desconectada | Diferenciar falta de conexão de ausência de trabalho |
| Capy reabre | Definir quais conversas e processos podem ser encontrados ou retomados |

A decisão 6 exige uma jornada de descoberta para sessões externas. Encontrar uma conversa salva não prova que ela ainda está ativa; seu estado e a disponibilidade do terminal precisam ser demonstrados antes de apresentar uma ação como executável.

A decisão 7 define a descoberta e a opção de ocultar. Como padrão proposto, sessões ocultas ficam fora dos avisos agregados do mascote até serem restauradas; essa preferência permanece ao reabrir o Capy.

Uma prova de viabilidade precisa demonstrar, para cada agente da decisão 8, a identificação de uma sessão iniciada externamente, sua associação ao projeto, a distinção entre histórico e atividade atual e os estados observáveis. A forma de habilitar integrações nas sessões externas e o alcance das ações diretas ainda não estão decididos.

### Cotas

**Delivers:** visibilidade de cota por provedor e conta junto ao trabalho em andamento. **Status: open.**

| Situação | Resultado proposto |
|---|---|
| Cota disponível | Mostrar o saldo e a conta a que ele pertence |
| Cota baixa | Avisar sem substituir a informação de atividade da sessão |
| Dados antigos ou indisponíveis | Mostrar a falta de atualização, sem apresentar um saldo antigo como atual |
| Conta ou provedor mudou | Manter a associação entre saldo e identidade da conta |

Em aberto: fornecedores da primeira versão, quais cotas serão exibidas e como a informação aparece no resumo e no painel.

### Mascote

**Delivers:** a identidade visual da decisão 4 ganha significado na experiência real. **Status: design.**

A decisão 2 define uma capivara principal para todas as sessões. Como apresentação proposta, ela mostra a quantidade de sessões que precisam do usuário, discriminadas no resumo. Animações recebem texto complementar, e atividade e cota aparecem como informações separadas. A animação de “cota acabando” do protótipo deve ser revista à luz dessa separação.

A decisão 2 define a posição arrastável. Como padrões propostos, a posição é lembrada entre aberturas e ajustada para permanecer acessível quando a configuração dos monitores muda; o balão aparece junto ao mascote dentro da área visível.

O desenho deve resolver: presença durante trabalho normal, chamada de atenção, múltiplos pedidos, conclusão, conexão desconhecida e preferência por movimento reduzido. A aparência do resumo e do painel completo permanece aberta.

A decisão 2 define a abertura do resumo compacto ao clicar no mascote. A organização proposta coloca pedidos de intervenção primeiro, seguidos das demais sessões e das cotas; os itens levam à ação correspondente ou ao terminal, conforme as capacidades da integração.

O protótipo do resumo permitirá discutir a hierarquia das sessões e a apresentação dos pedidos de intervenção. As ações demonstradas com dados simulados não provam compatibilidade com os agentes reais.

## Sources

- Conversa original do Claude Code: `C:/Users/MOBILTEC/.claude/projects/C--PProjetos/a1957091-4b33-4223-9e07-a7eb68ad177d.jsonl` — declarações do usuário em 2026-10-05 sobre reunir três ideias num novo produto, escolher capivara e aprovar o protótipo.
- Memória original: `C:/Users/MOBILTEC/.claude/projects/C--PProjetos/memory/project-novo-produto-pet-capivara.md` — contexto recuperado, referência visual e recomendações técnicas ainda abertas.
- Protótipo recuperado: `C:/PProjetos/Capy/prototypes/capivara.html` — arte SVG e animações CSS aprovadas na conversa original.
- Roadmap do Terminal-Hub: `C:/Users/MOBILTEC/.claude/projects/C--Users-MOBILTEC/memory/project-terminal-hub-roadmap.md` — referência para sessões, estado e integração de cotas.
- [Coucou](https://louis-cfm.github.io/coucou/) — referência para um companheiro visual que acompanha agentes e permite interação.
- Esta conversa — nome Capy, mascote único em posição arrastável com resumo compacto ao clicar, proposta inicial de foco em atenção e confirmação de acompanhamento de sessões externas com descoberta automática e opção de ocultar, inicialmente para Claude Code, Codex e Antigravity.
