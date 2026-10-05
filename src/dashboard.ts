import './style.css';
import { action, desktopCommand, showError, snapshot, subscribe, type Session, type Snapshot } from './bridge';

const eye = '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 3l18 18M10 5c5-1 9 3 11 7-1 2-2 3-4 4M7 7c-2 1-3 3-4 5 3 6 8 8 13 5m-7-7a3 3 0 0 0 4 4"/></svg>';
const escape = (s: string) => s.replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]!));
const sessions = document.getElementById('sessions')!;
const notice = document.getElementById('notice')!;
function row(s: Session): string {
  const buttons = s.state === 'waiting' ? s.request === 'permission'
    ? '<button class="primary" data-action="allow">Permitir uma vez</button><button data-action="deny">Negar</button>'
    : '<button class="primary" data-action="answer" data-answer="Só balão">Só balão</button><button data-action="answer" data-answer="Balão e som">Balão e som</button>'
    : '<button data-action="terminal">Ver terminal simulado</button>';
  return `<article class="session ${escape(s.state)}" data-id="${escape(s.id)}"><div class="session-top"><span class="agent-mark ${escape(s.kind)}" aria-hidden="true">${escape(s.symbol)}</span><h2>${escape(s.project)}</h2><button class="icon-button" data-action="hide" aria-label="Ocultar sessão ${escape(s.project)}">${eye}</button></div><p class="session-meta">${escape(s.agent)} · ${escape(s.origin)}</p><p class="session-message">${escape(s.message)}</p>${s.state === 'waiting' && s.command ? `<code class="request-command">${escape(s.command)}</code>` : ''}<div class="actions">${buttons}</div><p class="state-line"><span aria-hidden="true"></span>${{waiting: 'Precisa de você', working: 'Trabalhando', done: 'Concluída'}[s.state] ?? 'Estado desconhecido'}</p></article>`;
}
function render(data: Snapshot) {
  const visible = data.sessions.filter(s => !s.hidden).sort((a,b) => Number(b.state === 'waiting') - Number(a.state === 'waiting'));
  const waiting = visible.filter(s => s.state === 'waiting').length;
  document.getElementById('subtitle')!.textContent = waiting ? `${waiting} sessões precisam de você` : `${visible.length} sessões acompanhadas`;
  sessions.innerHTML = visible.length ? visible.map(row).join('') : '<div class="empty"><h2>Tudo tranquilo por aqui.</h2><p>Restaure as sessões ocultas ou escolha outro cenário no painel completo.</p></div>';
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
  if (button.dataset.action === 'terminal') { notice.textContent = 'Terminal simulado. Esta versão não abre terminais reais.'; return; }
  const kind = button.dataset.action!;
  void action(kind, id, button.dataset.answer ?? '').then(() => {
    notice.textContent = kind === 'hide' ? 'Sessão oculta. Você pode restaurá-la abaixo.' : 'Resposta simulada enviada.';
    (document.querySelector<HTMLButtonElement>(`[data-id="${id}"] button`) ?? document.getElementById('restore'))?.focus();
  }).catch(showError);
});
document.getElementById('restore')!.addEventListener('click', () => void action('restore').catch(showError));
document.getElementById('close')!.addEventListener('click', () => void desktopCommand('hide_window', { label: document.body.dataset.surface }).catch(showError));
document.getElementById('openPanel')?.addEventListener('click', () => void desktopCommand('show_panel').catch(showError));
document.querySelectorAll<HTMLButtonElement>('[data-scenario]').forEach(b => b.addEventListener('click', () => void action('scenario', '', b.dataset.scenario!).catch(showError)));
document.getElementById('reduceMotion')?.addEventListener('change', event => void action('motion', '', String((event.target as HTMLInputElement).checked)).catch(showError));
document.addEventListener('keydown', event => { if (event.key === 'Escape') void desktopCommand('hide_window', { label: document.body.dataset.surface }).catch(showError); });
document.getElementById('quotaRows')!.innerHTML = [{label:'Claude · Conta 1',remaining:38},{label:'Codex',remaining:72},{label:'Antigravity',remaining:61}].map(q => `<div class="quota-row"><span>${q.label}</span><div class="quota-track" aria-hidden="true"><span style="width:${q.remaining}%"></span></div><strong>${q.remaining}%</strong></div>`).join('');
async function start() { await subscribe(render); render(await snapshot()); await desktopCommand('ui_ready'); }
void start().catch(showError);
