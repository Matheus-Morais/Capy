import './style.css';
import { action, desktopCommand, native, showError, snapshot, subscribe, type Snapshot } from './bridge';
import { escape, sessionRow, quotaRows } from './presentation';

const sessions = document.getElementById('sessions')!;
const notice = document.getElementById('notice')!;
let realMode = false;
function render(data: Snapshot) {
  if (realMode !== (data.scenario === 'real')) notice.textContent = '';
  realMode = data.scenario === 'real';
  const visible = data.sessions.filter(s => !s.hidden).sort((a,b) => Number(b.state === 'waiting') - Number(a.state === 'waiting'));
  const waiting = visible.filter(s => s.state === 'waiting').length;
  document.getElementById('subtitle')!.textContent = waiting ? `${waiting} sessões precisam de você` : `${visible.length} sessões acompanhadas`;
  sessions.innerHTML = visible.length ? visible.map(s => sessionRow(s, realMode)).join('') : realMode
    ? '<div class="empty"><h2>Nenhuma sessão visível.</h2><p>Abra uma sessão de Claude Code ou Codex. A lista é atualizada automaticamente a cada 5 segundos; sessões ocultas podem ser restauradas abaixo.</p></div>'
    : '<div class="empty"><h2>Tudo tranquilo por aqui.</h2><p>Restaure as sessões ocultas ou escolha outro cenário no painel completo.</p></div>';
  document.querySelector<HTMLElement>('.simulation')!.textContent = realMode ? 'Sessões reais · descoberta local experimental' : 'Demonstração · sessões, respostas e cotas simuladas';
  document.getElementById('quotaRows')!.innerHTML = quotaRows(realMode);
  document.querySelector<HTMLElement>('#quotaTitle span')!.textContent = realMode ? 'Não conectadas' : 'Simulação';
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
document.addEventListener('keydown', event => { if (event.key === 'Escape') void desktopCommand('hide_window', { label: document.body.dataset.surface }).catch(showError); });
async function start() { await subscribe(render); render(await snapshot()); await desktopCommand('ui_ready'); }
void start().catch(showError);
