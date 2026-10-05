# Capy

<!-- impeccable:product-schema 1 -->

## Platform

web

Este registro cobre a interface web de um produto desktop para Windows. O aplicativo será construído com Rust e Tauri, conforme escolha do usuário em 2026-10-05.

## Stack

Stack escolhida: Rust + Tauri 2 para o aplicativo desktop, TypeScript + HTML/CSS para a interface e SVG/CSS para a capivara. O protótipo atual ainda usa JavaScript puro; sua conversão para TypeScript fará parte da implementação. Framework de interface, persistência e bibliotecas de terminal ainda serão definidos conforme as necessidades demonstradas.

## Users

O usuário inicial é o idealizador do Capy, que trabalha no Windows com várias sessões de Claude Code, Codex e Antigravity abertas em terminais. O público de uma eventual distribuição ainda será definido.

## Product Purpose

Reunir o acompanhamento de sessões de agentes, a visibilidade de cotas por conta e a interação com pedidos de intervenção num produto com um mascote capivara. A primeira hipótese de valor é ajudar o usuário a identificar e tratar sessões esperando sua resposta.

## Operating Context

- Produto novo inspirado no Terminal-Hub, Token-Watch e Coucou.
- Sessões podem ser iniciadas pelo Capy ou externamente.
- O usuário confirmou descoberta automática das sessões suportadas, com opção de ocultar.
- Uma única capivara acompanha todas as sessões e pode ser arrastada para a posição desejada.
- Clicar nela abre um resumo compacto, com acesso ao painel completo.

## Capabilities and Constraints

- Agentes iniciais confirmados: Claude Code, Codex e Antigravity.
- As capacidades de descoberta, leitura do estado e resposta precisam ser demonstradas separadamente para cada agente.
- O projeto é independente dos aplicativos que servem de referência.
- O resumo deve permitir distinguir projeto, sessão e agente.
- A organização com pedidos de intervenção primeiro, demais sessões e cotas depois é uma proposta de experiência.
- Protótipos usam dados simulados claramente identificados. Uma interação de demonstração não representa uma integração disponível.

## Brand Commitments

- Nome escolhido: Capy.
- Mascote padrão: capivara animada.
- A referência visual aprovada está em `prototypes/capivara.html`. Preservar essa arte ao explorar a experiência ao redor dela.
- O protótipo original tem seis estados de animação: parado, trabalhando, precisa de você, terminou, cota acabando e dormindo.

## Evidence on Hand

- `.design/capy.md` registra as decisões e os pontos em aberto da descoberta.
- `prototypes/capivara.html` é uma cópia idêntica do protótipo aprovado em 2026-10-05 no Claude Code.
- A conversa original identificada é `a1957091-4b33-4223-9e07-a7eb68ad177d`.
- Em 2026-10-05 a leitura local identificou sessões abertas reais de Claude Code (Prisma) e Codex (Capy), com processo/bloqueio validado. A conexão de leitura ao daemon existente também comprovou estados runtime ativo e ocioso do Codex. `.design/codex-activity.md` delimita essa integração: espera é observada por flags, respostas continuam no Codex, ociosidade não prova conclusão. Claude, respostas pelo Capy, terminal, cotas e Antigravity seguem pendentes. `.design/session-discovery.md` registra a primeira etapa.

## Product Principles

- A experiência precisa reunir trabalho distribuído entre várias sessões numa única presença na tela.
- O usuário escolhe a posição do mascote e quais sessões acompanhar.
- O resumo compacto é a primeira superfície de interação; o painel completo aprofunda a visão.
- A demonstração visual preserva a identidade aprovada e mantém explícito o que é simulado.

## Current Exploration

Em 2026-10-05 o usuário sugeriu olhar de frente. O resumo em preview.html apresenta essa proposta em SVG; preview-side.html permite comparar com a pose lateral. O usuário aprovou a pose de frente como melhor que a lateral; ela será a pose padrão.
