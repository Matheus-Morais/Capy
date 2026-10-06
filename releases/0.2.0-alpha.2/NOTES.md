# Capy 0.2.0-alpha.2 — prévia local com cotas Codex

Inclui a prévia anterior e conecta cotas reais da conta que o daemon Codex informou. Exibe bucket, janela principal/secundária, saldo percentual disponível, consulta e renovação. Atualiza a cada minuto, sem enviar conteúdo de conversas, forçar renovação de autenticação, consumir resets ou alterar créditos.

A identidade é conferida antes/depois da leitura; notificação de mudança de conta invalida a amostra. Saldo desaparece ao atingir reset, expirar após 120 s ou falhar atualização. Interface também vence o dado por temporizador, sem depender da próxima atualização do backend. Leitura de cotas tem worker próprio; publicação de observações continua no monitor principal.

Limites: a fonte identifica a conta conectada por e-mail e plano, sem accountId; não associa cotas a sessões, organizações ou outras contas. Claude/Antigravity continuam sem saldo real conectado. A etapa de cotas por todos os provedores/contas não está concluída. As demais etapas e provas de IDE/acesso ao app/respostas/Claude ampliado/QA desktop permanecem abertas.

Executável: `Capy.exe`, ProductVersion `0.2.0-alpha.2`, compilado do commit `f5979cd`; tamanho e hash em `SHA256.json`. Build, 35 testes Rust, 9 testes JS e 20 verificações automáticas nativas passaram. O smoke comprova abertura/ciclo das janelas, sem substituir QA visual nativo. Publicação remota cancelada pelo usuário; esta é uma prévia local, sem substituir as releases finais por etapa.

Verificação independente: C1–C5 PASS contra `f5979cd`, incluindo leitura de cotas pelo executável release e comparação com o daemon real. Relatório: `.checks/quotas-codex.verified.md`. Perfil light e limites de amostragem explicitados no relatório.
