# Capy

Primeira base desktop para Windows em Rust + Tauri 2, com interface TypeScript, HTML/CSS e mascote SVG. Sessões e cotas continuam simuladas.

## Aplicativo desktop

- Instalar dependências: `npm install`.
- Desenvolvimento: `npm run desktop`.
- Compilar executável local: `npm run desktop:build`.
- Executável: `src-tauri/target/release/capy.exe` (requer o runtime WebView2).
- Verificação: `npm test`, `cargo test --manifest-path src-tauri/Cargo.toml`, `npm run verify:native`.

A capivara aparece sozinha, de frente, sobre a área de trabalho. Clique para abrir/fechar o resumo; arraste para mover ou use as setas quando estiver com foco. No resumo, abra o painel completo para experimentar os estados.

Na bandeja: clique esquerdo abre o resumo; clique direito oferece Mostrar Capy, Ocultar Capy, Resumo, Painel completo e Sair. Fechar uma janela a oculta. Use Sair para encerrar o aplicativo. Uma segunda abertura reutiliza a mesma instância.

A posição é salva ao ocultar, fechar uma janela ou sair normalmente, em `%APPDATA%/dev.capy.desktop/position.json`; a restauração verifica os monitores disponíveis. Pedidos, respostas e cotas são demonstrações locais; nenhuma permissão executa comandos reais.

## Protótipo HTML preservado

Abra `prototypes/preview.html` diretamente no navegador. Os arquivos CSS e JavaScript devem permanecer na mesma pasta.

- Arraste a capivara ou use as setas quando ela estiver com foco.
- Clique nela para abrir ou fechar o resumo compacto.
- Experimente permitir, negar, responder, ocultar e restaurar sessões.
- Use o painel completo e os cenários de demonstração.

Sessões, descoberta, respostas, terminais e cotas são simulados. A posição fica salva somente no navegador.

`prototypes/capivara.html` preserva a arte original aprovada. `PRODUCT.md` e `.design/capy.md` registram as decisões; `DESIGN.md` documenta a interface.
