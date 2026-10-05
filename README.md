# Capy

Aplicativo desktop para Windows em Rust + Tauri 2, com interface TypeScript, HTML/CSS e mascote SVG. Descobre sessões abertas de Claude Code e Codex e acompanha a atividade do Codex pelo daemon local existente.

## Aplicativo desktop

- Instalar dependências: `npm install`.
- Desenvolvimento: `npm run desktop`.
- Compilar executável local: `npm run desktop:build`.
- Executável: `src-tauri/target/release/capy.exe` (requer o runtime WebView2).
- Verificação: `npm test`, `cargo test --manifest-path src-tauri/Cargo.toml`, `npm run verify:native`.

A capivara aparece sozinha, de frente, sobre a área de trabalho. Clique para abrir/fechar o resumo; arraste para mover ou use as setas quando estiver com foco. No resumo, abra o painel completo para experimentar os estados.

Na bandeja: clique esquerdo abre o resumo; clique direito oferece Mostrar Capy, Ocultar Capy, Resumo, Painel completo e Sair. Fechar uma janela a oculta. Use Sair para encerrar o aplicativo. Uma segunda abertura reutiliza a mesma instância.

A posição é salva ao ocultar, fechar uma janela ou sair normalmente, em `%APPDATA%/dev.capy.desktop/position.json`; a restauração verifica os monitores disponíveis.

O desktop inicia em **Sessões reais** e atualiza a descoberta a cada cinco segundos. Claude Code é identificado pelo registro local de sessão e pela identidade de criação do processo; Codex, pelo bloqueio mantido pela conversa e seus metadados. Históricos encerrados e subagentes Codex ficam fora da lista. As fontes são experimentais e podem mudar com atualizações dos agentes. `CLAUDE_CONFIG_DIR` e `CODEX_HOME` são respeitados; os caminhos padrão usam o perfil do Windows.

O Codex pode mostrar **Trabalhando**, **Precisa de você** ou **Ociosa**. Ociosa significa ausência de turno ativo, não conclusão de tarefa. A leitura usa o daemon já aberto, sem iniciar ou retomar conversas, com prazo de cinco segundos por ciclo. Sessões fora desse daemon ou falhas de conexão mostram **Estado desconhecido**; um estado anterior não é conservado após falha. Pedidos devem ser respondidos no Codex, sem botões de resposta ou terminal no Capy. A consulta solicita somente o resumo (`includeTurns: false`), usa identidade/projeto/estado e descarta os demais campos; nenhum conteúdo de conversa ou credencial é persistido.

Claude Code ainda mostra atividade desconhecida. Antigravity tem integração pendente, indicada no diagnóstico. Ocultações persistem em `hidden-sessions.json`, junto à posição; restaurar limpa essa preferência. Nenhuma configuração dos agentes é alterada.

O painel permite voltar aos cenários **Demo**. Pedidos, respostas e cotas nesses cenários são demonstrações locais; nenhuma permissão executa comandos reais. Cotas não conectadas aparecem sem porcentagens no modo real. O preview de navegador permanece simulado.

Diagnóstico sem abrir janelas: `capy.exe --discover-report "caminho-do-relatorio.json"` (presença) ou `capy.exe --activity-report "caminho-do-relatorio.json"` (presença e estado atual do Codex). O relatório contém os identificadores e projetos locais descobertos; não o publique como fixture.

## Protótipo HTML preservado

Abra `prototypes/preview.html` diretamente no navegador. Os arquivos CSS e JavaScript devem permanecer na mesma pasta.

- Arraste a capivara ou use as setas quando ela estiver com foco.
- Clique nela para abrir ou fechar o resumo compacto.
- Experimente permitir, negar, responder, ocultar e restaurar sessões.
- Use o painel completo e os cenários de demonstração.

Sessões, descoberta, respostas, terminais e cotas são simulados. A posição fica salva somente no navegador.

`prototypes/capivara.html` preserva a arte original aprovada. `PRODUCT.md` e `.design/capy.md` registram as decisões; `DESIGN.md` documenta a interface.
