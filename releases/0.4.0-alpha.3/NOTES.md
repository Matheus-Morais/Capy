# Capy 0.4.0-alpha.3

Versão com a evolução visual e comportamental completa da mascote Capy, mantendo a galeria histórica de versões (V1 a V5) e preservando o showcase de animações interativo no repositório.

## Novidades Visuais e Comportamentais

1. **Estado Ocioso Vivo e Desperto (`s-idle`)**:
   - A Capy não dorme mais de olhos fechados com ZZZ assim que para.
   - Enquanto ociosa (`s-idle`), ela permanece acordada e atenta: pisca naturalmente a cada 3,8s, respira suavemente com balanço corporal, olha em volta, mexe as orelhinhas, fareja o ambiente com o focinho e balança levemente a laranja na cabeça.
   - O sono profundo (`s-sleeping`) com ZZZ e olhinhos fechados só é ativado após 3 minutos de inatividade real ou pelo cenário demo.

2. **Novos Movimentos Dinâmicos (Atalhos de Teclado & Sintetizador de Áudio)**:
   - 💃 **Dancinha & Samba** (<kbd>T</kbd>): Passinho ritmado, balanço de patinhas, orelhas vibrando e notas musicais flutuando com arpejo cintilante.
   - 🐾 **Andadinha no Lugar** (<kbd>A</kbd>): Waddle fofo pantaneiro alternando as patinhas com som borbulhante.
   - 🤸 **Giro & Pulo 360°** (<kbd>R</kbd>): Salto acrobático com giro completo de 360° da capivara e spin da laranjinha com fanfarra de comemoração.
   - 🧘 **Super Alongamento** (<kbd>E</kbd>): Esquiçada zen profunda com respiração ronronante e suspiro pantaneiro.
   - 🍑 **Reboladinho Rápido** (<kbd>G</kbd>): Wiggle acelerado com coraçõezinhos e chilreio sonoro.
   - 🎵 **Batidinha de Patinha** (<kbd>Z</kbd>): Tap ritmado de patinha no chão com efeito sonoro de percussão/crunch.

3. **Guarda-Roupa & Acessórios Personalizáveis**:
   - Bonés e chapéus (<kbd>1</kbd>): Streetwear, Chapéu de Palha, Cartola Chic, Gorro de Inverno, Coroa de Flores.
   - Roupas e acessórios (<kbd>2</kbd>): Cachecol, Gravata Borboleta, Moletom Capy Dev, Capa de Chuva, Óculos Escuros.
   - Fantasias completas (<kbd>3</kbd>): Mago/Bruxo, Dino-Capy, Pirata, Detetive Sherlock, Rei Capivara.
   - Tecla <kbd>0</kbd> despir todos os itens.
   - Configurações persistidas no `localStorage` e integradas ao painel de configurações.

4. **Amiguinhos do Pantanal & Clima Dinâmico**:
   - Mini-pets (<kbd>4</kbd>): Bem-te-vi cantando no ombro, Borboleta voando, Patinho amarelo e Tartaruguinha.
   - Clima dinâmico (<kbd>5</kbd>): Céu estrelado com vaga-lumes à noite, raios de sol, chuva pantaneira ou automático pelo horário real do sistema.
   - Dev Buddy (<kbd>J</kbd>): Dicas úteis de desenvolvimento e programação.
   - Bem-estar e hidratação (<kbd>L</kbd>): Lembrete de beber água com garrafinha animada.

5. **Arquivo Showcase Preservado**:
   - O arquivo [showcase.html](file:///C:/PProjetos/Capy/prototypes/showcase.html) permanece disponível na íntegra no repositório com o catálogo completo de todas as 5 versões, movimentos, acessórios e sintetizador sonoro.

## Validação

- `npm test`: 34 testes unitários e de integração aprovados (100% verde).
- `npm run verify:native`: Validação nativa completa aprovada (`passed=True`, `uiErrors={}`).
- `npm run verify:pet-native`: 16 verificações nativas no WebView2/Tauri aprovadas (`checks=16`, `error=null`).
- Teste de layout: Sem overflow horizontal no painel nativo nem na visualização de tarefas.
- Suporte a Movimento Reduzido (`prefers-reduced-motion` e botão no painel) verificado e preservado.
