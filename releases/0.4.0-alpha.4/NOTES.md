# Capy 0.4.0-alpha.4

Versão focada no aprimoramento da governança e visualização de cotas (Anti-Flicker Cache padrão Token-Watch, falas proativas da mascote e hover peek), além do refinamento das animações da Capy trabalhando no laptop.

## Principais Novidades

### 1. Governança de Cotas & Anti-Flicker Cache (Padrão Token-Watch)
- **Fim do travessão `—` repentino por inatividade**: O Capy agora mantém um **Anti-Flicker Cache** com persistência local (`localStorage`), espelhando a arquitetura comprovada do Token-Watch (`_last_good_snapshots`).
- **Indicação clara de estado**: Quando uma fonte fica temporariamente quieta (ex: sem prompts no Claude nos últimos 2 minutos), o Capy continua exibindo o último saldo confirmado com um ícone sutil de relógio (`92% 🕒`) e tooltip informativo (`Última leitura há X min · cache Token-Watch`).
- **Alerta no elo mais fraco**: No Antigravity, a leitura multi-janelas prioriza o saldo mais restritivo ativo (ex: cota 3P / modelos de terceiros em 14% ativa alerta visual imediato).

### 2. Alertas de Cota Proativos Falados pela Capy
- **Dev Buddy comunicativa**: Quando qualquer cota monitorada atinge patamar baixo (≤ 20%) ou crítico (≤ 10%), a Capy fala proativamente na tela através de um balão de diálogo dinâmico (`showSpeechBubble`), sem exigir nenhum clique do usuário.
- **Feedback audiovisual orgânico**: A mascote mexe as orelhinhas (`g-ear-twitch`) e emite sinal sonoro suave quando o áudio está habilitado.
- **Cooldown anti-spam**: Intervalo inteligente de 10 minutos por cota para manter o foco e evitar repetições desnecessárias.

### 3. Resumo Automático ao Passar o Mouse (Hover Peek)
- **Fala imediata no balão**: Ao posicionar o cursor sobre a Capy, ela solta instantaneamente uma fala com o resumo das cotas (`Claude: 92% 🕒 · AGY: 14% (Outros modelos) 🐾`).
- **Abertura suave do painel lateral**: Se mantido o hover por 250ms, a janela do resumo rápido se abre ao lado da Capy via comando nativo (`peek_summary`), **sem roubar o foco do teclado**.
- **Auto-hide com tolerância**: Afastar o mouse recolhe a janela automaticamente, a menos que o usuário tenha clicado para fixá-la (pinned).

### 4. Digitação com Patinhas Visíveis sobre o Laptop (`s-working`)
- **Correção da ordem de camadas SVG**: O grupo `<g class="typing-paws">` foi posicionado acima do `<g class="laptop">`, garantindo que as patinhas fiquem 100% visíveis enquanto a Capy trabalha no notebook.
- **Animação alternada de teclas**: Deslocamento de 7px para cima e para baixo (`capy-type-key`) simulando digitação veloz no teclado.
- **Micro-fidgets em trabalho**: Piscadas de olhos, respiração e olhinhos atentos funcionam continuamente mesmo durante o modo de foco.
- **Atalho de Pausa para Café (<kbd>X</kbd>)**: Permite alternar manualmente entre foco e pausa com caneca de café e balão animado.

### 5. Otimização de Performance e Isolamento de Hooks
- Hooks do Claude Code repontados para `%LOCALAPPDATA%\Capy\capy.exe`, eliminando contenção de arquivos durante compilações e garantindo execução ultrarrápida (< 20ms).
- Sincronização do `bridge.json` do statusline do Claude com o executável oficial instalado.

## Validação e Qualidade
- 51 testes JavaScript/TypeScript aprovados (`npm test`, 100% verde).
- 154 testes Rust aprovados (`cargo test`, 100% verde).
- Compilação limpa do frontend (`vite build`) e binário nativo (`tauri build`).
