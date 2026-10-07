# Capy multifuncional — plano aprovado

Fonte: conversa e plano aprovado em 2026-10-06. Este documento preserva o escopo completo; entregas parciais não encerram o trabalho.

## Ordem

Sessões → pedidos de atenção → quotas/modelos/contas/IA. A (acompanhar) primeiro, B (iniciar tarefas) e C (abrir/trocar para a origem) posteriormente. Claude primeiro para contas e troca; integrações Codex existentes continuam funcionando. Codex e Antigravity recebem a gestão de contas após validação própria.

## Comportamento aprovado

- Início de tarefas com pasta, provedor, conta, modelo e instrução; CLI externo padrão e terminal integrado como alternativa.
- Contas existentes e login oficial em perfil isolado; credenciais ficam com CLI/Windows, nunca senhas no formulário Capy.
- Chat próprio com fluxo CLI compatível/assinatura ou API, identificando cobrança em cada conversa.
- Troca manual na mesma sessão quando o provedor suportar; caso contrário, nova sessão com resumo revisável.
- Alertas por conta, janela e limiar; padrões 50/60/70/80/90%, editáveis/desativáveis, novos limiares permitidos. Alertar uma vez por limiar/janela, agrupar saltos no maior limiar.
- Gatilho automático por percentual configurável, independente para 5h e semanal; cadeia ordenada por conta. Dados antigos ou sem identidade não acionam troca.
- Esperar o turno terminar; preparar resumo e pedir revisão a cada transferência. Nunca transferir uma resposta em andamento. Sem alternativa, avisar.
- Resumo completo com objetivo, decisões, estado, arquivos alterados, testes e próximos passos; citar os caminhos/seções dos planos e .md que guiaram o desenvolvimento, incluindo suas regras relevantes.
- Cada transferência exige aprovação única; mudanças de cobrança exigem confirmação identificando destino e tipo. Nenhuma autorização permanente implícita.
- Pausa/parada somente pelos controles oficiais disponíveis, mostrando a sessão exata e pedindo confirmação.
- Conclusão somente por evento explícito do provedor; Stop, silêncio e desaparecimento não comprovam conclusão.

## Movimento

Cumprimento ao abrir; olhar apenas durante hover; reação ao clique; teclado em trabalho; comemoração finita em conclusão confirmada; aceno imediato ao novo pedido e lembrete discreto a cada 30s até visto; sono após 180s sem trabalho/pedido. Interação acorda a mascote. Sons opcionais desligados por padrão. Reduzir movimento preserva postura, badge e estado, retirando movimento espacial. Pausar animações quando a janela estiver escondida.

Foco: braçinho que acena como pedido de atenção e cumprimento. Continuidade: olhos/teclado/sono refletem estados. Feedback: clique e conclusão finitos. Orçamento: SVG existente, transform/opacity, sem biblioteca de animações; um temporizador de 1s para ciclo de vida, sem seguimento global de cursor.

### Retorno sobre personalidade — propostas abertas

O usuário reiterou: seguir a ordem; A inicialmente, B/C depois; sem acompanhar o cursor fora do hover. Pediu sugestões adicionais a partir de cumprimento, hover, clique, comemoração, teclado, sono e aceno para pedidos.

Sugestões para discutir, ainda não aprovadas e sem alterar o escopo implementado:
- Ao acordar: abrir os olhos, espreguiçar e voltar à postura do estado atual.
- Durante raciocínio observado: mão no queixo; teclado somente quando houver evidência de atividade de programação.
- Aproximação de limite: olhar uma pequena ampulheta, uma vez por alerta; conservar prioridade dos pedidos de atenção.
- Falha confirmada: expressão confusa e um balão discreto com acesso à sessão e ao erro.
- Durante transferência aprovada: gesto curto de passagem de bastão, somente depois de confirmar a operação real.
- Ociosidade antes do sono: pequenos gestos, como bocejar, ajeitar-se ou cheirar uma folhinha; proposta de intervalos variáveis de 45–90s, sem deslocamento da mascote.

Questão aberta: permitir gestos espontâneos discretos durante ociosidade ou restringir reações a eventos e interações? A recomendação é permitir os discretos, interrompidos por trabalho/pedidos e desativados por reduzir movimento.

## Interfaces

Reutilizar snapshot/eventos Tauri e adaptadores existentes. Acrescentar preferências persistidas, perfis de conta e recursos declarados por provedor. Não oferecer operações cuja integração não exista. Toda continuação mantém identidade da sessão/pedido/conta e revalida antes da execução. UI de demonstração continua explicitamente separada.
