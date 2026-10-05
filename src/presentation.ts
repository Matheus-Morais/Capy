import type { Session, Snapshot } from './bridge';

export const escape = (s: string) => s.replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]!));
const eye = '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 3l18 18M10 5c5-1 9 3 11 7-1 2-2 3-4 4M7 7c-2 1-3 3-4 5 3 6 8 8 13 5m-7-7a3 3 0 0 0 4 4"/></svg>';

export function sessionRow(s: Session, real: boolean): string {
  const buttons = real ? '' : s.state === 'waiting' ? s.request === 'permission'
    ? '<button class="primary" data-action="allow">Permitir uma vez</button><button data-action="deny">Negar</button>'
    : '<button class="primary" data-action="answer" data-answer="Só balão">Só balão</button><button data-action="answer" data-answer="Balão e som">Balão e som</button>'
    : '<button data-action="terminal">Ver terminal simulado</button>';
  const states: Record<string, string> = { waiting: 'Precisa de você', working: 'Trabalhando', idle: 'Ociosa', done: 'Concluída' };
  const state = real && !['waiting', 'working', 'idle', 'unknown'].includes(s.state) ? 'unknown' : s.state;
  return `<article class="session ${escape(state)}" data-id="${escape(s.id)}"><div class="session-top"><span class="agent-mark ${escape(s.kind)}" aria-hidden="true">${escape(s.symbol)}</span><h2>${escape(s.project)}</h2><button class="icon-button" data-action="hide" aria-label="Ocultar sessão ${escape(s.project)}">${eye}</button></div><p class="session-meta">${escape(s.agent)} · ${escape(s.origin)}</p>${real ? `<p class="session-id">Sessão ${escape(s.id.split(':').slice(1).join(':') || s.id)}</p>` : ''}<p class="session-message">${escape(s.message)}</p>${!real && s.state === 'waiting' && s.command ? `<code class="request-command">${escape(s.command)}</code>` : ''}${buttons ? `<div class="actions">${buttons}</div>` : ''}<p class="state-line"><span aria-hidden="true"></span>${states[state] ?? 'Estado desconhecido'}</p></article>`;
}

export function petState(data: Snapshot): string {
  const followed = data.sessions.filter(s => !s.hidden);
  if (followed.some(s => s.state === 'waiting')) return 'waiting';
  if (followed.some(s => s.state === 'working')) return 'working';
  if (followed.some(s => s.state === 'unknown' || s.state === 'idle')) return 'idle';
  return followed.length ? 'done' : 'sleeping';
}

export function quotaRows(real: boolean): string {
  if (real) return '<p class="quota-unavailable">Cotas reais ainda não estão conectadas.</p>';
  return [{label:'Claude · Conta 1',remaining:38},{label:'Codex',remaining:72},{label:'Antigravity',remaining:61}].map(q => `<div class="quota-row"><span>${q.label}</span><div class="quota-track" aria-hidden="true"><span style="width:${q.remaining}%"></span></div><strong>${q.remaining}%</strong></div>`).join('');
}
