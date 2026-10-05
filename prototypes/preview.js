const companion = document.getElementById('companion');
const summary = document.getElementById('summary');
const petButton = document.getElementById('petToggle');
const pet = document.getElementById('pet');
const sessionsBox = document.getElementById('sessions');
const fullPanel = document.getElementById('fullPanel');
const fullSessions = document.getElementById('fullSessions');
const hiddenSessions = document.getElementById('hiddenSessions');
const restoreButton = document.getElementById('restoreHidden');
const eyeIcon = '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 3l18 18M10 5c5-1 9 3 11 7-1 2-2 3-4 4M7 7c-2 1-3 3-4 5 3 6 8 8 13 5m-7-7a3 3 0 0 0 4 4"/></svg>';
const originSessions = [
  {id:'claude', project:'Token-Watch', agent:'Claude Code', symbol:'C', kind:'claude', origin:'Terminal externo', state:'waiting', request:'permission', message:'Executar os testes do projeto?', command:'python -m pytest', path:'C:\\PProjetos\\Token-Watch', hidden:false},
  {id:'codex', project:'Terminal-Hub', agent:'Codex', symbol:'O', kind:'codex', origin:'Terminal externo', state:'waiting', request:'question', message:'Qual tipo de notificação você prefere?', hidden:false},
  {id:'agy', project:'Capy', agent:'Antigravity', symbol:'A', kind:'agy', origin:'Criada no Capy', state:'working', message:'Desenhando o resumo compacto.', hidden:false}
];
let sessions = [];
let noticeTimer;
let selectedScenario = 'waiting';
let dragState = null;
let ignoreClick = false;
let lastPetState = '';

function announce(text) {
  const notice = document.getElementById('notice');
  notice.textContent = text;
  notice.classList.add('visible');
  clearTimeout(noticeTimer);
  noticeTimer = setTimeout(() => notice.classList.remove('visible'), 3200);
}

function openSummary(open) {
  const anchor = companion.style.left ? companion.getBoundingClientRect() : null;
  summary.hidden = !open;
  petButton.setAttribute('aria-expanded', String(open));
  if (anchor) setPosition(anchor.left, anchor.bottom - companion.offsetHeight);
}

function rowMarkup(s, allowRestore = false) {
  const stateText = {waiting:'Precisa de você', working:'Trabalhando', done:'Concluída'}[s.state];
  const action = s.hidden && allowRestore
    ? `<div class="actions"><button data-action="restore" data-id="${s.id}">Restaurar sessão</button></div>`
    : s.state === 'waiting' && s.request === 'permission'
      ? `<code class="request-command">${s.command}</code><div class="actions"><button class="primary" data-action="allow" data-id="${s.id}">Permitir uma vez</button><button class="secondary" data-action="deny" data-id="${s.id}">Negar</button></div>`
      : s.state === 'waiting'
        ? `<div class="actions"><button class="primary" data-action="answer" data-answer="Só balão" data-id="${s.id}">Só balão</button><button data-action="answer" data-answer="Balão e som" data-id="${s.id}">Balão e som</button></div>`
        : `<div class="actions"><button data-action="terminal" data-id="${s.id}">Ver terminal simulado</button></div>`;
  return `<article class="session ${s.state}"><div class="session-top"><span class="agent-mark ${s.kind}" aria-hidden="true">${s.symbol}</span><strong>${s.project}</strong>${s.hidden ? '' : `<button class="icon-button" data-action="hide" data-id="${s.id}" aria-label="Ocultar sessão ${s.project}">${eyeIcon}</button>`}</div><p class="session-meta">${s.agent} · ${s.origin}${s.request === 'permission' && s.state === 'waiting' ? `<br>${s.path}` : ''}</p><p class="session-message">${s.message}</p>${action}<p class="state-line ${s.state}"><span class="state-dot" aria-hidden="true"></span>${s.hidden ? 'Oculta' : stateText}</p></article>`;
}

function render() {
  const anchor = companion.style.left ? companion.getBoundingClientRect() : null;
  const visible = sessions.filter(s => !s.hidden);
  const waiting = visible.filter(s => s.state === 'waiting');
  const hidden = sessions.filter(s => s.hidden);
  const ordered = [...visible].sort((a,b) => Number(b.state === 'waiting') - Number(a.state === 'waiting'));
  sessionsBox.innerHTML = ordered.length ? ordered.map(s => rowMarkup(s)).join('') : '<div class="empty"><strong>Tudo tranquilo por aqui.</strong><p>As sessões dos agentes suportados aparecem automaticamente. Nesta demonstração, escolha outro cenário para continuar.</p></div>';
  document.getElementById('summarySubtitle').textContent = waiting.length ? `${waiting.length} sessões precisam de você` : visible.length ? `${visible.length} sessões acompanhadas` : 'Nenhuma sessão visível';
  restoreButton.hidden = !hidden.length;
  restoreButton.textContent = `${hidden.length} ${hidden.length === 1 ? 'sessão oculta' : 'sessões ocultas'} · Restaurar`;
  fullSessions.innerHTML = ordered.length ? ordered.map(s => rowMarkup(s)).join('') : '<div class="empty"><strong>Nenhuma sessão visível.</strong><p>Selecione um cenário ou restaure as sessões ocultas.</p></div>';
  hiddenSessions.innerHTML = hidden.length ? '<p>Sessões ocultas</p>' + hidden.map(s => rowMarkup(s, true)).join('') : '';
  const badge = document.getElementById('petBadge');
  badge.hidden = !waiting.length;
  badge.textContent = waiting.length;
  let state = waiting.length ? 'waiting' : visible.some(s => s.state === 'working') ? 'working' : visible.length ? 'done' : 'sleeping';
  if (state !== lastPetState) {
    pet.setAttribute('class', `s-${state} p-${state === 'working' ? 'sit' : state === 'sleeping' ? 'lie' : 'stand'}`);
    lastPetState = state;
  }
  document.getElementById('petLabel').textContent = waiting.length ? `${waiting.length} precisam de você` : state === 'working' ? 'Agentes trabalhando' : state === 'done' ? 'Trabalho concluído' : 'Nenhuma sessão acompanhada';
  document.querySelectorAll('[data-scenario]').forEach(b => b.setAttribute('aria-pressed', String(b.dataset.scenario === selectedScenario)));
  if (anchor) setPosition(anchor.left, anchor.bottom - companion.offsetHeight);
}

function scenario(key) {
  selectedScenario = key;
  sessions = structuredClone(originSessions);
  if (key === 'sleeping') sessions = [];
  else if (key !== 'waiting') sessions.forEach(s => {s.state = key; s.message = key === 'working' ? 'O agente está executando sua tarefa.' : 'A tarefa foi concluída. Confira o resultado no terminal.';});
  render();
}

function handleSessionAction(event) {
  const button = event.target.closest('[data-action]');
  if (!button) return;
  const s = sessions.find(item => item.id === button.dataset.id);
  if (!s) return;
  const action = button.dataset.action;
  if (action === 'hide') {s.hidden = true; announce(`${s.project} foi ocultada. Você pode restaurar a sessão.`);}
  else if (action === 'restore') {s.hidden = false; announce(`${s.project} foi restaurada.`);}
  else if (action === 'terminal') {announce(`Terminal simulado: ${s.agent} · ${s.project}. O protótipo não abre processos reais.`); return;}
  else {
    s.state = 'working';
    s.message = action === 'allow' ? 'Permissão concedida na simulação. Executando os testes.' : action === 'deny' ? 'Permissão negada na simulação. Procurando outra alternativa.' : `Resposta simulada: ${button.dataset.answer}. O agente está continuando.`;
    announce(action === 'answer' ? 'Resposta simulada enviada.' : 'Decisão simulada enviada.');
  }
  const focusedId = s.id;
  const container = fullPanel.hidden ? sessionsBox : fullSessions;
  render();
  if (action === 'hide') (restoreButton.hidden ? petButton : restoreButton).focus();
  else container.querySelector(`[data-id="${focusedId}"]`)?.focus();
}

function setPosition(x, y, remember = false) {
  const w = companion.offsetWidth, h = companion.offsetHeight;
  const nx = Math.min(Math.max(8, x), Math.max(8, window.innerWidth - w - 8));
  const ny = Math.min(Math.max(8, y), Math.max(8, window.innerHeight - h - 8));
  companion.style.left = `${nx}px`;
  companion.style.top = `${ny}px`;
  companion.style.right = 'auto';
  companion.style.bottom = 'auto';
  if (remember) try {localStorage.setItem('capy-prototype-position', JSON.stringify({x:nx,y:ny}));} catch {}
}

function clampPosition() {
  if (!companion.style.left) return;
  const box = companion.getBoundingClientRect();
  setPosition(box.left, box.top);
}

petButton.addEventListener('pointerdown', event => {
  if (event.button !== 0) return;
  const box = companion.getBoundingClientRect();
  dragState = {x:event.clientX, y:event.clientY, left:box.left, top:box.top, moved:false};
  ignoreClick = false;
  petButton.setPointerCapture(event.pointerId);
});
petButton.addEventListener('pointermove', event => {
  if (!dragState) return;
  const dx = event.clientX - dragState.x, dy = event.clientY - dragState.y;
  if (Math.hypot(dx,dy) > 5) dragState.moved = true;
  if (dragState.moved) setPosition(dragState.left + dx, dragState.top + dy);
});
petButton.addEventListener('pointerup', () => {
  if (!dragState) return;
  ignoreClick = dragState.moved;
  if (dragState.moved) {const box = companion.getBoundingClientRect(); setPosition(box.left,box.top,true);}
  dragState = null;
});
petButton.addEventListener('pointercancel', () => {dragState = null; ignoreClick = true;});
petButton.addEventListener('click', () => {
  if (ignoreClick) {ignoreClick = false; return;}
  openSummary(summary.hidden);
});
petButton.addEventListener('keydown', event => {
  const direction = {ArrowLeft:[-16,0],ArrowRight:[16,0],ArrowUp:[0,-16],ArrowDown:[0,16]}[event.key];
  if (!direction) return;
  event.preventDefault();
  const box = companion.getBoundingClientRect();
  setPosition(box.left + direction[0], box.top + direction[1], true);
});
document.getElementById('closeSummary').addEventListener('click', () => {openSummary(false); petButton.focus();});
document.getElementById('openFull').addEventListener('click', () => {
  fullPanel.hidden = false;
  document.querySelector('.work-scene').hidden = true;
  openSummary(false);
  document.getElementById('closeFull').focus();
});
document.getElementById('closeFull').addEventListener('click', () => {
  fullPanel.hidden = true;
  document.querySelector('.work-scene').hidden = false;
  openSummary(true);
  document.getElementById('openFull').focus();
});
restoreButton.addEventListener('click', () => {sessions.forEach(s => s.hidden = false); render(); announce('Sessões restauradas.'); petButton.focus();});
sessionsBox.addEventListener('click', handleSessionAction);
fullPanel.addEventListener('click', handleSessionAction);
document.querySelectorAll('[data-scenario]').forEach(b => b.addEventListener('click', () => scenario(b.dataset.scenario)));
document.getElementById('reduceMotion').addEventListener('change', event => document.body.classList.toggle('reduce-motion', event.target.checked));
document.getElementById('reset').addEventListener('click', () => {
  try {localStorage.removeItem('capy-prototype-position');} catch {}
  companion.style.cssText = '';
  fullPanel.hidden = true;
  document.querySelector('.work-scene').hidden = false;
  openSummary(true);
  scenario('waiting');
  announce('Demonstração reiniciada.');
});
document.addEventListener('keydown', event => {if (event.key === 'Escape' && !summary.hidden) {openSummary(false); petButton.focus();}});
window.addEventListener('resize', clampPosition);
document.getElementById('quotaRows').innerHTML = [{label:'Claude · Conta 1',remaining:38},{label:'Codex',remaining:72},{label:'Antigravity',remaining:61}].map(q => `<div class="quota-row"><span>${q.label}</span><div class="quota-track" aria-hidden="true"><span style="--remaining:${q.remaining}%"></span></div><strong>${q.remaining}%</strong></div>`).join('');
scenario('waiting');
try {const saved = JSON.parse(localStorage.getItem('capy-prototype-position')); if (saved && Number.isFinite(saved.x) && Number.isFinite(saved.y)) setPosition(saved.x,saved.y);} catch {}
