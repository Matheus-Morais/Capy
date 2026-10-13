import './style.css';
import { action, desktopCommand, emitEvent, native, showError, snapshot, subscribe, respondIntervention, setInterventionSubscription, type Snapshot } from './bridge';
import { escape, sessionRow } from './presentation';
import { renderQuotaBoard, renderSummaryQuotas } from './quota-board';
import { pendingIntervention, interventionContext, captureInterventionAnswers, restoreInterventionAnswers, questionAnswer } from './interventions-ui';
import { runInterventionSubscriptionClick } from './intervention-subscription';
import { renderPreferences } from './settings-ui';
import { initializeAccounts } from './accounts-ui';
import { renderHandoffs } from './handoff-ui';
import { shouldCloseOnEscape } from './keyboard';

const sessions = document.getElementById('sessions')!;
const notice = document.getElementById('notice')!;
let realMode = false;
const opening = new Set<string>();
const submitting = new Set<string>();
let latestSnapshot: Snapshot | undefined;
let cancelQuotaExpiry: (() => void) | undefined;
function render(data: Snapshot) {
  latestSnapshot = data;
  renderPreferences(data);
  renderHandoffs(data);
  const retained = captureInterventionAnswers(sessions.querySelectorAll<HTMLInputElement>('[data-question-answer],[data-free-answer]'), document.activeElement);
  if (realMode !== (data.scenario === 'real')) notice.textContent = '';
  realMode = data.scenario === 'real';
  const visible = data.sessions.filter(s => !s.hidden).sort((a,b) => Number(b.state === 'waiting') - Number(a.state === 'waiting'));
  const waiting = visible.filter(s => s.state === 'waiting').length + (data.chatTransfers??[]).length;
  const subtitle = document.getElementById('subtitle')!;
  const pillClass = waiting ? 'waiting' : visible.length ? 'working' : 'idle';
  const pillText = waiting ? `${waiting} pendente${waiting > 1 ? 's' : ''}` : visible.length ? `${visible.length} ativo${visible.length > 1 ? 's' : ''}` : 'Tudo zen';
  subtitle.className = `zen-status-pill ${pillClass}`;
  subtitle.innerHTML = `<span class="pulse-dot"></span><span>${pillText}</span>`;
  const isSummary = document.body.dataset.surface === 'summary';
  const emptyMarkup = isSummary
    ? (realMode
      ? '<div class="zen-empty-state"><p class="zen-empty-title">🌿 Tudo tranquilo por aqui</p><p class="zen-empty-desc">Nenhuma sessão ativa no momento.<br>A Capy monitora Claude Code, Codex e Antigravity automaticamente.</p></div>'
      : '<div class="zen-empty-state"><p class="zen-empty-title">🌿 Tudo tranquilo por aqui</p><p class="zen-empty-desc">Nenhum agente em execução no momento.<br>A Capy está descansando em segundo plano.</p></div>')
    : (realMode
      ? '<div class="empty"><h2>Nenhuma sessão visível.</h2><p>Abra uma sessão de Claude Code ou Codex. A lista é atualizada automaticamente a cada 5 segundos; sessões ocultas podem ser restauradas abaixo.</p></div>'
      : '<div class="empty"><h2>Tudo tranquilo por aqui.</h2><p>Restaure as sessões ocultas ou escolha outro cenário no painel completo.</p></div>');
  sessions.innerHTML = visible.length ? visible.map(s => sessionRow(s, realMode, data.interventions.filter(r => r.sessionId === s.id), data.subscriptions.find(sub => sub.sessionId === s.id),data.chatTransfers?.find(review=>s.id===`chat:${review.sourceId}`))).join('') : emptyMarkup;
  restoreInterventionAnswers(sessions.querySelectorAll<HTMLInputElement>('[data-question-answer],[data-free-answer]'), retained);
  for (const row of sessions.querySelectorAll<HTMLElement>('[data-id]')) {
    if (opening.has(row.dataset.id!)) {
      const button = row.querySelector<HTMLButtonElement>('[data-action="open-source"]');
      if (button) { button.disabled = true; button.setAttribute('aria-busy', 'true'); }
    }
    const requestElement = row.querySelector<HTMLElement>('[data-request]');
    if (requestElement && submitting.has(requestElement.dataset.request!)) requestElement.querySelectorAll<HTMLButtonElement>('button').forEach(b => { b.disabled = true; b.setAttribute('aria-busy', 'true'); });
  }
  const sim = document.querySelector<HTMLElement>('.simulation');
  if (sim) sim.textContent = realMode ? 'Sessões reais · descoberta local experimental' : 'Demonstração · sessões, respostas e cotas simuladas';
  cancelQuotaExpiry?.();
  const quotaRowsEl = document.getElementById('quotaRows')!;
  if (isSummary) {
    cancelQuotaExpiry = renderSummaryQuotas(quotaRowsEl, realMode, data.quotas ?? []);
    const agentsCount = document.getElementById('agentsCount');
    if (agentsCount) agentsCount.textContent = `${visible.length} ativo${visible.length === 1 ? '' : 's'}`;
    const quotaStatus = document.getElementById('quotaStatusLabel');
    if (quotaStatus) quotaStatus.textContent = realMode ? 'Ao vivo' : 'Renovam hoje';
    const footerSync = document.getElementById('footerSync');
    if (footerSync) footerSync.textContent = realMode ? '● Auto-sync ativo' : '● Simulação ativa';
  } else {
    cancelQuotaExpiry = renderQuotaBoard(quotaRowsEl, realMode, data.quotas ?? []);
    const quotaTitleSpan = document.querySelector<HTMLElement>('#quotaTitle span');
    if (quotaTitleSpan) quotaTitleSpan.textContent = realMode ? 'Fonte e atualização' : 'Simulação';
    const refreshBtn = document.getElementById('refreshQuotas');
    if (refreshBtn) refreshBtn.hidden = !realMode;
  }
  const integrations = document.getElementById('integrations')!;
  if (integrations) {
    if (isSummary) {
      integrations.hidden = true;
      integrations.innerHTML = '';
    } else {
      integrations.hidden = !realMode;
      integrations.innerHTML = '<h2>Integrações locais</h2>' + (data.integrations.length ? data.integrations.map(i => `<p><strong>${escape(i.agent)}</strong><br>${escape(i.message)}</p>`).join('') : '<p>Buscando sessões locais…</p>');
    }
  }
  const footer = document.querySelector<HTMLElement>('.sheet-footer > span');
  if (footer) footer.textContent = realMode ? 'Atualiza a cada 5 s' : '3 agentes no exemplo';
  const restore = document.getElementById('restore') as HTMLButtonElement;
  const hidden = data.sessions.filter(s => s.hidden).length;
  restore.hidden = !hidden;
  restore.textContent = `${hidden} ${hidden === 1 ? 'sessão oculta' : 'sessões ocultas'} · Restaurar`;
  document.querySelectorAll<HTMLButtonElement>('[data-scenario]').forEach(b => b.setAttribute('aria-pressed', String(b.dataset.scenario === data.scenario)));
  const motion = document.getElementById('reduceMotion') as HTMLInputElement | null;
  if (motion) motion.checked = data.reduceMotion;
  if (isSummary) {
    requestAnimationFrame(() => {
      const sheet = document.querySelector<HTMLElement>('.sheet');
      if (sheet) {
        const height = Math.ceil(sheet.getBoundingClientRect().height) + 2;
        if (height > 0) void desktopCommand('resize_summary', { height, width: 360 }).catch(() => {});
      }
    });
  }
}
sessions.addEventListener('click', event => {
  const button = (event.target as Element).closest<HTMLButtonElement>('[data-action]');
  if (!button) return;
  const id = button.closest<HTMLElement>('[data-id]')!.dataset.id!;
  if (button.dataset.action === 'open-source') {
    if (opening.has(id) || button.disabled) return;
    opening.add(id);
    button.disabled = true;
    button.setAttribute('aria-busy', 'true');
    void desktopCommand('open_source', { id }).then(() => {
      notice.textContent = 'Abertura solicitada para a conversa selecionada.';
    }).catch(showError).finally(() => {
      opening.delete(id);
      void snapshot().then(render).catch(showError);
    });
    return;
  }
  if (button.dataset.action === 'connect-interventions') {
    const operation = runInterventionSubscriptionClick(button, id, setInterventionSubscription);
    if (!operation) return;
    const enabled = button.dataset.enabled === 'true';
    void operation.then(() => {
      notice.textContent = enabled ? 'Conexão de respostas iniciada para este cartão Codex.' : 'Conexão de respostas encerrada.';
    }).catch(showError).finally(() => void snapshot().then(render).catch(showError));
    return;
  }
  if (button.dataset.action === 'submit-intervention' || button.dataset.action === 'decide-intervention') {
    const requestElement = button.closest<HTMLElement>('[data-request]');
    const nonce = requestElement?.dataset.request;
    const request = nonce && pendingIntervention(latestSnapshot, nonce);
    if (!request || request.status !== 'pending' || !requestElement || submitting.has(nonce!)) {
      notice.textContent = 'Este pedido expirou. Confira o estado atualizado do agente.';
      return;
    }
    let response: Record<string, unknown>;
    if (button.dataset.action === 'decide-intervention') response = { decision: button.dataset.decision };
    else {
      const answers: Record<string, string | string[]> = {};
      for (const q of request.body.questions ?? []) {
        const free = requestElement.querySelector<HTMLInputElement>(`[data-free-answer="${CSS.escape(q.id)}"]`);
        const selected = [...requestElement.querySelectorAll<HTMLInputElement>(`[data-question-answer="${CSS.escape(q.id)}"]:checked`)].map(input => input.value);
        const value = questionAnswer(q, free?.value ?? '', selected);
        if (value === undefined || value.length === 0) { notice.textContent = `Responda à pergunta “${q.header}” antes de enviar.`; (free ?? requestElement.querySelector<HTMLInputElement>(`[data-question-answer="${CSS.escape(q.id)}"]`))?.focus(); return; }
        answers[q.id] = value;
      }
      response = { answers };
    }
    submitting.add(nonce!);
    requestElement.querySelectorAll<HTMLButtonElement>('button').forEach(b => b.disabled = true);
    const requestNonce = request.nonce;
    void respondIntervention(interventionContext(request), response).then(() => {
      notice.textContent = 'Pedido resolvido no agente.';
    }).catch(showError).finally(() => {
      submitting.delete(requestNonce);
      void snapshot().then(render).catch(showError);
    });
    return;
  }
  if (button.dataset.action === 'terminal') { notice.textContent = 'Terminal simulado. Esta versão não abre terminais reais.'; return; }
  const kind = button.dataset.action!;
  void action(kind, id, button.dataset.answer ?? '').then(() => {
    notice.textContent = kind === 'hide' ? 'Sessão oculta. Você pode restaurá-la abaixo.' : 'Resposta simulada enviada.';
    const target = [...document.querySelectorAll<HTMLElement>('[data-id]')].find(row => row.dataset.id === id)?.querySelector<HTMLButtonElement>('button');
    (target ?? (document.getElementById('restore')?.hidden ? document.getElementById('close') : document.getElementById('restore')))?.focus();
  }).catch(showError);
});
document.getElementById('restore')!.addEventListener('click', () => void action('restore').catch(showError));
document.getElementById('close')!.addEventListener('click', () => void desktopCommand('hide_window', { label: document.body.dataset.surface }).catch(showError));
document.getElementById('openPanel')?.addEventListener('click', () => void desktopCommand('show_panel').catch(showError));
document.getElementById('refreshQuotas')!.addEventListener('click', event => {
  const button = event.currentTarget as HTMLButtonElement;
  button.disabled = true;
  void desktopCommand('refresh_quotas').then(() => {
    notice.textContent = 'Atualização solicitada. As fontes serão consultadas em segundo plano. Antigravity consulta as cotas automaticamente; Claude depende de uma nova observação do statusline.';
  }).catch(showError).finally(() => setTimeout(() => { button.disabled = false; },15_000));
});
document.querySelectorAll<HTMLButtonElement>('[data-quota-section]').forEach(button => {
  button.addEventListener('click', () => {
    if (document.body.dataset.surface !== 'panel') {
      void desktopCommand('show_panel').catch(showError);
      return;
    }
    const section = document.getElementById(button.dataset.quotaSection!);
    if (!section) { notice.textContent = 'Esta configuração está disponível no aplicativo desktop.'; return; }
    for (let parent: HTMLElement | null = section; parent; parent = parent.parentElement) {
      if (parent instanceof HTMLDetailsElement) parent.open = true;
    }
    section.scrollIntoView({block:'start'});
    const target = section.querySelector<HTMLElement>('button,input,select,textarea,summary') ?? section;
    if (target === section) target.tabIndex = -1;
    target.focus({preventScroll:true});
  });
});
document.querySelectorAll<HTMLButtonElement>('[data-scenario]').forEach(b => {
  if (b.dataset.scenario === 'real') b.hidden = !native;
  b.addEventListener('click', () => { notice.textContent = ''; void action('scenario', '', b.dataset.scenario!).catch(showError); });
});
document.getElementById('reduceMotion')?.addEventListener('change', event => void action('motion', '', String((event.target as HTMLInputElement).checked)).catch(showError));
document.addEventListener('keydown', event => {
  const target=event.target instanceof Element?event.target:null;
  if(shouldCloseOnEscape(event.key,{
    defaultPrevented:event.defaultPrevented,composing:event.isComposing,
    terminal:!!target?.closest('#terminalViewport'),
    editing:!!target?.closest('input,textarea,select,[contenteditable="true"]'),
    dialog:!!target?.closest('dialog,[role="dialog"]'),
  }))void desktopCommand('hide_window', { label: document.body.dataset.surface }).catch(showError);
});
async function start() {
  await subscribe(render);render(await snapshot());
  if(document.body.dataset.surface==='panel'){
    try{await (await import('./tasks-ui')).initializeTasks();}catch(error){showError(error);}
    try{await (await import('./chat-ui')).initializeChat();}catch(error){showError(error);}
  }
  try{await initializeAccounts();}catch(error){showError(error);}
  if (document.body.dataset.surface === 'summary') {
    document.addEventListener('pointerenter', () => { void emitEvent('summary-mouse-enter'); });
    document.addEventListener('pointerleave', () => { void emitEvent('summary-mouse-leave'); });
  }
  await desktopCommand('ui_ready');
}
void start().catch(showError);
