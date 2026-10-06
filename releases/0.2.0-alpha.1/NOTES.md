# Capy 0.2.0-alpha.1 — prévia local

Prévia do desenvolvimento em andamento. Nenhuma das seis etapas está declarada concluída.

- Antigravity CLI: presença por bloqueio, projeto por metadados locais e atividade por hooks opcionais. Compatível com os nomes de transcript e comandos observados no CLI 1.3.0.
- Codex: card com abertura da conversa pelo ID técnico via link registrado no Windows, com revalidação de presença/projeto antes da abertura. Suporta associação de executável e AppID empacotado.
- Configuração Antigravity reversível em scripts/antigravity-hooks.ps1; não foi habilitada globalmente no ambiente do usuário.

Validação: 32 testes Rust, 8 testes JS, build TypeScript/Vite e configuração Antigravity no Windows PowerShell 5.1. Layout do renderer conferido no navegador com dados sintéticos.

Pendências: prova CLI completa interrompida por RESOURCE_EXHAUSTED/429; sessão IDE ainda não comprovada; seleção da conversa após abertura do link ainda não comprovada no app; acesso às origens Claude/Antigravity, respostas, cotas, suporte Claude ampliado e QA visual desktop.

O relatório SHA256.json acompanhará o executável local depois da compilação. Publicação remota cancelada pelo usuário.
