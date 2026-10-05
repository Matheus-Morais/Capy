import { getCurrentWindow } from '@tauri-apps/api/window';
import artwork from '../prototypes/front-pet.svg?raw';
import '../prototypes/pet.css';
import '../prototypes/front-pet.css';
import './style.css';
import { desktopCommand, native, showError, snapshot, subscribe, subscribeVisibility, type Snapshot } from './bridge';
import { petState } from './presentation';

document.getElementById('petArtwork')!.innerHTML = artwork;
const button = document.getElementById('petToggle') as HTMLButtonElement;
const pet = document.getElementById('pet')!;
const badge = document.getElementById('petBadge')!;
let lastState = '';
function render(data: Snapshot) {
  const followed = data.sessions.filter(s => !s.hidden);
  const waiting = followed.filter(s => s.state === 'waiting').length;
  const state = petState(data);
  if (state !== lastState) { pet.setAttribute('class', `s-${state} p-${state === 'working' ? 'sit' : state === 'sleeping' ? 'lie' : 'stand'}`); lastState = state; }
  badge.hidden = !waiting;
  badge.textContent = String(waiting);
  document.getElementById('petStatus')!.textContent = waiting ? `${waiting} precisam de você` : state === 'working' ? 'Agentes trabalhando' : state === 'idle' ? 'Sessões abertas' : state === 'done' ? 'Trabalho concluído' : 'Capy descansando';
  document.body.classList.toggle('reduce-motion', data.reduceMotion);
}

let pointer: { x: number; y: number; dragged: boolean } | null = null;
let suppressClick = false;
button.addEventListener('pointerdown', event => {
  if (event.button !== 0) return;
  pointer = { x: event.clientX, y: event.clientY, dragged: false };
  suppressClick = false;
});
button.addEventListener('pointermove', event => {
  if (!pointer || pointer.dragged || !event.buttons) return;
  if (Math.hypot(event.clientX - pointer.x, event.clientY - pointer.y) <= 5) return;
  pointer.dragged = true;
  suppressClick = true;
  if (native) void getCurrentWindow().startDragging().catch(showError);
});
button.addEventListener('pointerup', () => { pointer = null; });
button.addEventListener('pointercancel', () => { pointer = null; });
button.addEventListener('click', () => {
  if (suppressClick) { suppressClick = false; return; }
  void desktopCommand('toggle_summary').catch(showError);
});
button.addEventListener('keydown', event => {
  const directions: Record<string, [number, number]> = { ArrowLeft: [-16, 0], ArrowRight: [16, 0], ArrowUp: [0, -16], ArrowDown: [0, 16] };
  const direction = directions[event.key];
  if (direction) { event.preventDefault(); void desktopCommand('move_pet', { dx: direction[0], dy: direction[1] }).catch(showError); }
});
async function start() {
  await subscribe(render);
  await subscribeVisibility(open => button.setAttribute('aria-expanded', String(open)));
  render(await snapshot());
  await desktopCommand('ui_ready');
}
void start().catch(showError);
