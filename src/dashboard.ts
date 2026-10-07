import './style.css';
import { action, desktopCommand, native, showError, snapshot, subscribe, respondIntervention, setInterventionSubscription, type Snapshot } from './bridge';
import { escape, sessionRow, renderQuotas } from './presentation';
import { pendingIntervention, interventionContext, captureInterventionAnswers, restoreInterventionAnswers } from './interventions-ui';
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
  document.getElementById('subtitle')!.textContent = waiting ? `${waiting} pedidos precisam de você` : `${visible.length} sessões acompanhadas`;
  sessions.innerHTML = visible.length ? visible.map(s => sessionRow(s, realMode, data.interventions.filter(r => r.sessionId === s.id), data.subscriptions.find(sub => sub.sessionId === s.id),data.chatTransfers?.find(review=>s.id===`chat:${review.sourceId}`))).join('') : realMode
    ? '<div class="empty"><h2>Nenhuma sessão visível.</h2><p>Abra uma sessão de Claude Code ou Codex. A lista é atualizada automaticamente a cada 5 segundos; sessões ocultas podem ser restauradas abaixo.</p></div>'
    : '<div class="empty"><h2>Tudo tranquilo por aqui.</h2><p>Restaure as sessões ocultas ou escolha outro cenário no painel completo.</p></div>';
  restoreInterventionAnswers(sessions.querySelectorAll<HTMLInputElement>('[data-question-answer],[data-free-answer]'), retained);
  for (const row of sessions.querySelectorAll<HTMLElement>('[data-id]')) {
    if (opening.has(row.dataset.id!)) {
      const button = row.querySelector<HTMLButtonElement>('[data-action="open-source"]');
      if (button) { button.disabled = true; button.setAttribute('aria-busy', 'true'); }
    }
    const requestElement = row.querySelector<HTMLElement>('[data-request]');
    if (requestElement && submitting.has(requestElement.dataset.request!)) requestElement.querySelectorAll<HTMLButtonElement>('button').forEach(b => { b.disabled = true; b.setAttribute('aria-busy', 'true'); });
  }
  document.querySelector<HTMLElement>('.simulation')!.textContent = realMode ? 'Sessões reais · descoberta local experimental' : 'Demonstração · sessões, respostas e cotas simuladas';
  cancelQuotaExpiry?.();
  cancelQuotaExpiry = renderQuotas(document.getElementById('quotaRows')!, realMode, data.quotas ?? []);
  document.querySelector<HTMLElement>('#quotaTitle span')!.textContent = realMode ? 'Fonte e atualização' : 'Simulação';
  const integrations = document.getElementById('integrations')!;
  integrations.hidden = !realMode;
  integrations.innerHTML = '<h2>Integrações locais</h2>' + (data.integrations.length ? data.integrations.map(i => `<p><strong>${escape(i.agent)}</strong><br>${escape(i.message)}</p>`).join('') : '<p>Buscando sessões locais…</p>');
  const footer = document.querySelector<HTMLElement>('.sheet-footer > span');
  if (footer) footer.textContent = realMode ? 'Atualiza a cada 5 s' : '3 agentes no exemplo';
  const restore = document.getElementById('restore') as HTMLButtonElement;
  const hidden = data.sessions.filter(s => s.hidden).length;
  restore.hidden = !hidden;
  restore.textContent = `${hidden} ${hidden === 1 ? 'sessão oculta' : 'sessões ocultas'} · Restaurar`;
  document.querySelectorAll<HTMLButtonElement>('[data-scenario]').forEach(b => b.setAttribute('aria-pressed', String(b.dataset.scenario === data.scenario)));
  const motion = document.getElementById('reduceMotion') as HTMLInputElement | null;
  if (motion) motion.checked = data.reduceMotion;
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
      notice.textContent = 'Este pedido expirou. Confira o estado atualizado do Codex.';
      return;
    }
    let response: Record<string, unknown>;
    if (button.dataset.action === 'decide-intervention') response = { decision: button.dataset.decision };
    else {
      const answers: Record<string, string> = {};
      for (const q of request.body.questions ?? []) {
        const free = requestElement.querySelector<HTMLInputElement>(`[data-free-answer="${CSS.escape(q.id)}"]`);
        const choice = requestElement.querySelector<HTMLInputElement>(`[data-question-answer="${CSS.escape(q.id)}"]:checked`);
        const value = free?.value.trim() ?? choice?.value;
        if (!value) { notice.textContent = `Responda à pergunta “${q.header}” antes de enviar.`; (free ?? requestElement.querySelector<HTMLInputElement>(`[data-question-answer="${CSS.escape(q.id)}"]`))?.focus(); return; }
        answers[q.id] = value;
      }
      response = { answers };
    }
    submitting.add(nonce!);
    requestElement.querySelectorAll<HTMLButtonElement>('button').forEach(b => b.disabled = true);
    const requestNonce = request.nonce;
    void respondIntervention(interventionContext(request), response).then(() => {
      notice.textContent = 'Pedido resolvido no Codex.';
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
  await desktopCommand('ui_ready');
}
void start().catch(showError);
