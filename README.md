# Capy

Mascote desktop para Windows que acompanha sessões de agentes, mostra pedidos de atenção e cotas e oferece chat e tarefas próprios. Feita com Rust/Tauri 2 e TypeScript; requer WebView2.

Consulte [capacidades e provas](CAPACIDADES.md), [executáveis e candidato para validação](ENTREGA_ATUAL.md), [retomada](NEXT_STEPS.md) e [plano de evolução](PLANO_EVOLUCAO.md). As provas citadas são históricas e não certificam automaticamente o executável disponível hoje.

## Usar

Clique na capivara para abrir o resumo; arraste para mover ou use as setas quando ela tiver foco. O resumo abre o painel completo. Na bandeja, abra/oculte a mascote e as janelas ou escolha Sair. Fechar uma janela a oculta; uma segunda abertura reutiliza a instância. Posição e sessões ocultas são preservadas.

O desktop inicia em **Sessões reais**. Claude Code e Codex têm descoberta e atividade; fontes experimentais podem mudar com versões dos agentes. Claude exige hooks opcionais para atividade recente. Codex observa o daemon existente e permite conectar respostas a perguntas/aprovações suportadas pelo pedido exato. A prova funcional dessas respostas existe; QA visual nativo dos cartões reais permanece aberto. O botão de origem Codex depende do protocolo registrado no Windows, e o despacho do link não prova seleção visual da conversa.

Antigravity CLI tem descoberta por lock/metadados e atividade por hooks, ainda experimental: houve working real, mas a prova completa terminou em 429. Presença/atividade da IDE permanecem sem prova real. **As cotas Antigravity são uma capacidade separada já validada na conta/export configurados**: consulta automática inicial e a cada minuto, sem abrir terminal. Isso não prova sessões IDE nem outras contas.

O painel oferece perfis, cotas/alertas, chat próprio Claude por assinatura, tarefas e terminal integrado/externo. Recuperação e transferência exigem revisão de contexto e destino. Há contratos de chat API OpenAI/Anthropic/Gemini, com cobrança API distinta da assinatura; envio API real e contas alternativas permanecem lacunas. Chat e tarefas próprios não retomam sessões externas Codex/Antigravity.

Estado desconhecido significa falta de evidência atual. Idle/Stop não comprovam conclusão. Cotas vencidas ou indisponíveis ficam sem percentual; Codex real foi observado, enquanto statusline/cotas Claude ainda precisam de prova real. Cenários Demo e preview de navegador usam dados simulados.

## Executar a partir do código

Requer Node/npm e toolchain Rust/Tauri para Windows.

```powershell
npm install
npm run desktop
```

Para compilar o executável local:

```powershell
npm run desktop:build
```

Saída: `src-tauri/target/release/capy.exe`. Esse comando usa `--no-bundle`; não gera instalador. Antes de recompilar ou testar o nativo, confira o caminho e encerre somente a instância deste checkout; reabra ao terminar.

Checks disponíveis: `npm test`, `cargo test --manifest-path src-tauri/Cargo.toml`, `npm run build` e `npm run verify:native`. Runners reais de agentes podem consumir inferência; seu alcance está nos documentos de prova.

## Hooks Claude

Após compilar, habilite a observação opcional:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/claude-hooks.ps1 -Action Enable
```

Reinicie as sessões Claude para carregar os hooks. Para remover, use o mesmo comando com `-Action Disable`. O script preserva hooks alheios e respeita `CLAUDE_CONFIG_DIR`; Codex respeita `CODEX_HOME`. O caminho do executável fica na configuração: remova antes de mover o checkout e habilite no novo caminho. Evidências Claude expiram em 30 s; respostas externas continuam na origem. A configuração Antigravity e suas limitações estão no [checklist específico](.checks/antigravity.md).

## Diagnóstico e demonstração

Sem abrir janelas: `capy.exe --discover-report "relatorio.json"`, `capy.exe --activity-report "relatorio.json"` ou `capy.exe --quota-report "relatorio.json"`. Relatórios contêm identidades e caminhos locais; mantenha-os privados.

Abra `prototypes/preview.html` para o protótipo simulado, com CSS/JavaScript na mesma pasta. `prototypes/capivara.html` preserva a arte original. [PRODUCT.md](PRODUCT.md), [.design/capy.md](.design/capy.md) e [DESIGN.md](DESIGN.md) registram as decisões do produto e da interface.
