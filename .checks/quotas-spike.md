# Cotas reais — spike read-only

Em 2026-10-06, o daemon Codex existente respondeu a `account/read` com `refreshToken:false` antes e depois de `account/rateLimits/read`. Mesma conta nos dois resultados. Fonte oficial: https://learn.chatgpt.com/docs/app-server, seções conta e limites.

Resposta observada: account.type=chatgpt; campos type/email/planType; um bucket em rateLimitsByLimitId com janelas primary e secondary. Não há accountId no objeto account retornado. Não enviar tokens, refresh de credenciais, consumir reset, comprar créditos ou enviar e-mail para obter cotas.

Evidência privada em scratch/codex-quota-probe.json contém timestamp, hash de identidade local, tipo/campos e limites reais. O e-mail não foi gravado no artefato nem exposto no log. A prova fechou o proxy próprio depois da consulta; não retomou conversas.

Próxima implementação deve preservar bucket/conta conectada/fonte, usedPercent, duração e resetsAt em segundos; distinguir ausência de conta, conta sem identidade, ausência de saldo, incompatibilidade, atualização falha e dado expirado. Não inferir saldo de tokens consumidos. Mudança de conta durante leitura invalida a amostra. Associação entre sessão e cota só quando o provedor/conta puderem ser comprovados.

A etapa 0.5.0 permanece aberta: nenhuma cota foi ligada à interface e fontes Claude/Antigravity seguem sem prova. O 429 do Antigravity confirma limite atingido, mas não fornece saldo percentual nem todas as contas/provedores.
