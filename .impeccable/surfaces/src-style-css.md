---
version: 1
slug: "src-style-css"
primary_target: "src/style.css"
related_targets: ["index.html","summary.html","panel.html","src/pet.ts","src/dashboard.ts"]
---

# Capy — base desktop

Modo: Operate. Refinamento da experiência já aprovada, agora com janelas nativas Tauri. Referências: prototypes/front-pet.svg e resumo compacto de prototypes/preview.html.

THESIS: uma capivara frontal acompanha o trabalho; clique abre um resumo que identifica sessões e pedidos sem ocupar a área de trabalho inteira.

OWN-WORLD: preservar SVG frontal aprovado, caramelo, aveia, castanho, oliva e âmbar; Segoe UI, linhas de sessão e divisórias suaves. Transparência apenas na janela pequena do mascote.

STORY: capivara sozinha, clique abre resumo, pedidos simulados mudam os estados, painel aprofunda a visão. Bandeja mostra, oculta e encerra.

FIRST VIEWPORT: pet 200×180, summary 380×620 e panel 760×680 logical px. Só pet aparece inicialmente. O resumo mantém pendências primeiro e rodapé de acesso ao painel; o painel usa lista à esquerda e cotas/cenários à direita.

FORM: brief-pinned-compact-companion; identidade e pose escolhidas pelo usuário, stack Rust/Tauri aprovada. A integração nativa altera presença e navegação de janelas, mantendo a linguagem visual.

FINISH: build e testes, inicialização real de WebViews e bandeja, inspeção visual da interface compilada, revisor independente. Gestos físicos e composição nativa requerem confirmação manual se o ambiente não expuser automação de apps Windows.
