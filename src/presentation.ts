import type { Session, Snapshot, QuotaRow, Intervention, Subscription } from './bridge';

export const escape = (s: string) => s.replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]!));
const eye = '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 3l18 18M10 5c5-1 9 3 11 7-1 2-2 3-4 4M7 7c-2 1-3 3-4 5 3 6 8 8 13 5m-7-7a3 3 0 0 0 4 4"/></svg>';

function interventionMarkup(request: Intervention): string {
  const b = request.body;
  const details = b.kind === 'question' ? (b.questions ?? []).map(q => `<fieldset class="intervention-question" data-question="${escape(q.id)}"><legend>${escape(q.header)} · ${escape(q.question)}</legend>${q.isOther
    ? `<label class="intervention-option"><input type="${q.isSecret ? 'password' : 'text'}" data-free-answer="${escape(q.id)}" maxlength="4096"${q.isSecret ? ' autocomplete="new-password"' : ''} aria-label="Sua resposta">${q.isSecret ? '<span class="sr-only">Resposta privada</span>' : ''}</label>`
    : q.options.map((o, i) => `<label class="intervention-option"><input type="radio" name="answer-${escape(request.nonce)}-${escape(q.id)}" value="${escape(o.label)}" data-question-answer="${escape(q.id)}"${i === 0 ? ' required' : ''}><span><strong>${escape(o.label)}</strong>${o.description ? `<small>${escape(o.description)}</small>` : ''}</span></label>`).join('')}</fieldset>`).join('')
    : `<p class="intervention-detail">${b.kind === 'command' ? `<strong>Comando</strong><code>${escape(b.command ?? '')}</code><strong>Diretório</strong><code>${escape(b.cwd ?? '')}</code>${b.reason ? `<span>${escape(b.reason)}</span>` : ''}` : `<strong>Alterações solicitadas</strong><pre>${escape(JSON.stringify(b.changes ?? [], null, 2))}</pre>${b.reason ? `<span>${escape(b.reason)}</span>` : ''}`}</p>`;
  const question = b.kind === 'question';
  return `<section class="intervention" data-request="${escape(request.nonce)}"><h3>${question ? 'Pergunta do Codex' : b.kind === 'command' ? 'Permissão para comando' : 'Permissão para alterações'}</h3>${details}${question ? '<button class="primary" data-action="submit-intervention">Enviar resposta</button>' : request.decisions.map((d, i) => `<button class="${i === 0 ? 'primary' : ''}" data-action="decide-intervention" data-decision="${escape(d)}">${escape(d === 'accept' ? 'Permitir' : d === 'decline' ? 'Negar' : d === 'cancel' ? 'Cancelar' : d)}</button>`).join('')}</section>`;
}

export function sessionRow(s: Session, real: boolean, requests: Intervention[] = [], subscription?: Subscription): string {
  const source = s.source_action;
  const buttons = real ? `${s.kind === 'codex' ? `<button data-action="connect-interventions" data-enabled="${subscription?.status === 'connected' ? 'false' : 'true'}"${subscription?.status === 'connecting' ? ' disabled aria-busy="true"' : ''}>${subscription?.status === 'connected' ? 'Desconectar respostas' : subscription?.status === 'connecting' ? 'Conectando…' : 'Conectar respostas'}</button>${subscription ? `<p class="source-reason">${escape(subscription.message)}</p>` : '<p class="source-reason">Conexão local explícita · este cartão Codex</p>'}` : ''}${source ? `<button data-action="open-source"${source.available ? '' : ' disabled'}>${escape(source.label)}</button>${source.reason ? `<p class="source-reason">${escape(source.reason)}</p>` : ''}` : ''}` : s.state === 'waiting' ? s.request === 'permission'
    ? '<button class="primary" data-action="allow">Permitir uma vez</button><button data-action="deny">Negar</button>'
    : '<button class="primary" data-action="answer" data-answer="Só balão">Só balão</button><button data-action="answer" data-answer="Balão e som">Balão e som</button>'
    : '<button data-action="terminal">Ver terminal simulado</button>';
  const states: Record<string, string> = { waiting: 'Precisa de você', working: 'Trabalhando', idle: 'Ociosa', done: 'Concluída' };
  const state = real && !['waiting', 'working', 'idle', 'unknown'].includes(s.state) ? 'unknown' : s.state;
  return `<article class="session ${escape(state)}" data-id="${escape(s.id)}"><div class="session-top"><span class="agent-mark ${escape(s.kind)}" aria-hidden="true">${escape(s.symbol)}</span><h2>${escape(s.project)}</h2><button class="icon-button" data-action="hide" aria-label="Ocultar sessão ${escape(s.project)}">${eye}</button></div><p class="session-meta">${escape(s.agent)} · ${escape(s.origin)}</p>${real ? `<p class="session-id">Sessão ${escape(s.id.split(':').slice(1).join(':') || s.id)}</p>` : ''}<p class="session-message">${escape(s.message)}</p>${!real && s.state === 'waiting' && s.command ? `<code class="request-command">${escape(s.command)}</code>` : ''}${buttons ? `<div class="actions">${buttons}</div>` : ''}${real ? requests.map(interventionMarkup).join('') : ''}<p class="state-line"><span aria-hidden="true"></span>${states[state] ?? 'Estado desconhecido'}</p></article>`;
}

export function petState(data: Snapshot): string {
  const followed = data.sessions.filter(s => !s.hidden);
  if (followed.some(s => s.state === 'waiting')) return 'waiting';
  if (followed.some(s => s.state === 'working')) return 'working';
  if (followed.some(s => s.state === 'unknown' || s.state === 'idle')) return 'idle';
  return followed.length ? 'done' : 'sleeping';
}

export function quotaRows(real: boolean, quotas: QuotaRow[] = [], now = Date.now()): string {
  if (real) {
    if (!quotas.length) return '<p class="quota-unavailable">Cotas reais ainda não estão conectadas.</p>';
    return quotas.map(q => {
      const w = q.window;
      const age = q.observedAt === null ? -1 : now - q.observedAt;
      const valid = q.state === 'fresh' && q.account && w && Number.isFinite(w.usedPercent) && w.usedPercent >= 0 && w.usedPercent <= 100
        && Number.isFinite(w.windowDurationMins) && w.windowDurationMins > 0 && Number.isFinite(w.resetsAt)
        && Number.isFinite(age) && age >= 0 && age <= 120_000 && w.resetsAt * 1000 > now && w.resetsAt * 1000 <= 8.64e15;
      const expired = q.state === 'stale' || (q.state === 'fresh' && w && !valid);
      const title = `${escape(q.provider)}${q.account ? ` · ${expired ? 'Última conta observada: ' : ''}${escape(q.account)}` : ''}`;
      const period = q.period === 'primary' ? 'Janela principal' : q.period === 'secondary' ? 'Janela secundária' : '';
      const meta = [q.bucket ? escape(q.bucket) : '', period, w && valid ? `${w.windowDurationMins} min` : ''].filter(Boolean).join(' · ');
      const remaining = valid && w ? Number((100 - w.usedPercent).toFixed(1)) : null;
      const balance = remaining !== null ? `<div class="quota-row"><span>Disponível</span><div class="quota-track" role="meter" aria-label="Cota disponível" aria-valuemin="0" aria-valuemax="100" aria-valuenow="${remaining}"><span style="width:${remaining}%"></span></div><strong>${remaining}%</strong></div><p class="quota-meta">Renova em ${escape(new Date(w!.resetsAt * 1000).toLocaleString('pt-BR', {dateStyle:'short',timeStyle:'short'}))}</p>`
        : `<p class="quota-unavailable">${q.state === 'stale' || (q.state === 'fresh' && w) ? 'Dado expirado · saldo indisponível.' : 'Saldo indisponível.'}</p>`;
      const observed = q.observedAt !== null && Number.isFinite(q.observedAt) && Math.abs(q.observedAt) <= 8.64e15
        ? `<p class="quota-meta">Consultado em ${escape(new Date(q.observedAt).toLocaleString('pt-BR', {dateStyle:'short',timeStyle:'short'}))}</p>` : '';
      return `<section class="quota-source"><h3>${title}</h3>${meta ? `<p class="quota-meta">${meta}</p>` : ''}${balance}${observed}<p class="quota-meta">${escape(q.message)}</p></section>`;
    }).join('');
  }
  return [{label:'Claude · Conta 1',remaining:38},{label:'Codex',remaining:72},{label:'Antigravity',remaining:61}].map(q => `<div class="quota-row"><span>${q.label}</span><div class="quota-track" aria-hidden="true"><span style="width:${q.remaining}%"></span></div><strong>${q.remaining}%</strong></div>`).join('');
}

export function renderQuotas(element: HTMLElement, real: boolean, quotas: QuotaRow[], now = Date.now()): () => void {
  let timer: ReturnType<typeof setTimeout> | undefined;
  function draw(time: number) {
    element.innerHTML = quotaRows(real, quotas, time);
    const deadlines = real ? quotas.filter(q => q.state === 'fresh' && q.window && q.observedAt !== null)
      .map(q => Math.min(q.observedAt! + 120_001, q.window!.resetsAt * 1000)).filter(d => Number.isFinite(d) && d > time) : [];
    if (deadlines.length) timer = setTimeout(() => draw(Date.now()), Math.min(120_001, Math.max(1, Math.min(...deadlines) - time)));
  }
  draw(now);
  return () => { if (timer !== undefined) clearTimeout(timer); };
}
