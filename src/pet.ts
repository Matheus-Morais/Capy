import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';
import artwork from '../prototypes/front-pet.svg?raw';
import '../prototypes/pet.css';
import '../prototypes/front-pet.css';
import './style.css';
import { desktopCommand, native, showError, snapshot, subscribe, subscribeVisibility, type Snapshot } from './bridge';
import { PetBehavior, hoverGaze } from './pet-behavior';
import { playPetSound } from './pet-sound';

document.getElementById('petArtwork')!.innerHTML = artwork;
const button = document.getElementById('petToggle') as HTMLButtonElement;
const pet = document.getElementById('pet')!;
const badge = document.getElementById('petBadge')!;
const behavior = new PetBehavior(Date.now());
let latest: Snapshot | undefined;
let lastGestureId = -1;
let windowVisible = true;
function render(data: Snapshot) {
  latest = data;
  const frame = behavior.update(data, Date.now());
  const {state, waiting, gesture} = frame;
  for (const value of ['idle', 'working', 'waiting', 'done', 'sleeping']) pet.classList.toggle(`s-${value}`, state === value);
  for (const pose of ['sit', 'lie', 'stand']) pet.classList.toggle(`p-${pose}`, pose === (state === 'working' ? 'sit' : state === 'sleeping' ? 'lie' : 'stand'));
  if (lastGestureId !== frame.gestureId) {
    if (windowVisible && !document.hidden && data.preferences?.sounds && (gesture === 'hello' || gesture === 'wave' || gesture === 'celebrate')) playPetSound(gesture);
    for (const value of ['hello', 'wave', 'click', 'celebrate']) pet.classList.remove(`g-${value}`);
    if (gesture) { void pet.getBoundingClientRect(); pet.classList.add(`g-${gesture}`); }
    lastGestureId = frame.gestureId;
  }
  if (!gesture) for (const value of ['hello', 'wave', 'click', 'celebrate']) pet.classList.remove(`g-${value}`);
  badge.hidden = !waiting;
  badge.textContent = String(waiting);
  document.getElementById('petStatus')!.textContent = waiting ? `${waiting} precisam de você` : gesture === 'celebrate' ? state === 'working' ? 'Conclusão confirmada · outros agentes trabalhando' : 'Conclusão confirmada' : state === 'working' ? 'Agentes trabalhando' : state === 'idle' ? 'Sessões abertas' : 'Capy descansando';
  document.body.classList.toggle('reduce-motion', data.reduceMotion);
}

let pointer: { x: number; y: number; dragged: boolean } | null = null;
let suppressClick = false;
button.addEventListener('pointerdown', event => {
  if (event.button !== 0) return;
  pointer = { x: event.clientX, y: event.clientY, dragged: false };
  suppressClick = false;
});
button.addEventListener('pointerenter', () => {
  behavior.interact(Date.now());
  pet.classList.add('is-hovered');
  if (latest) render(latest);
});
button.addEventListener('pointerleave', () => {
  pet.classList.remove('is-hovered');
  pet.style.removeProperty('--gaze-x');
  pet.style.removeProperty('--gaze-y');
});
button.addEventListener('pointermove', event => {
  if (pointer || !pet.classList.contains('is-hovered')) return;
  const rect = button.getBoundingClientRect();
  const [x, y] = hoverGaze(event.clientX - rect.left, event.clientY - rect.top, rect.width, rect.height);
  pet.style.setProperty('--gaze-x', `${x}px`);
  pet.style.setProperty('--gaze-y', `${y}px`);
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
  behavior.interact(Date.now(), true);
  if (latest) render(latest);
  void desktopCommand('toggle_summary').catch(showError);
});
button.addEventListener('keydown', event => {
  const directions: Record<string, [number, number]> = { ArrowLeft: [-16, 0], ArrowRight: [16, 0], ArrowUp: [0, -16], ArrowDown: [0, 16] };
  const direction = directions[event.key];
  if (direction) { event.preventDefault(); void desktopCommand('move_pet', { dx: direction[0], dy: direction[1] }).catch(showError); }
});
async function start() {
  await subscribe(render);
  await subscribeVisibility(open => {
    button.setAttribute('aria-expanded', String(open));
    if (open) { behavior.acknowledge(); behavior.interact(Date.now()); if (latest) render(latest); }
  });
  if(native){
    windowVisible=await getCurrentWindow().isVisible();
    await listen<boolean>('pet-visibility',event=>{
      windowVisible=event.payload;
      document.body.classList.toggle('pet-paused',document.hidden||!windowVisible);
      if(windowVisible){behavior.interact(Date.now());if(latest)render(latest);}
    });
    await listen('attention-viewed',()=>{behavior.acknowledge();behavior.interact(Date.now());if(latest)render(latest);});
  }
  document.body.classList.toggle('pet-paused',document.hidden||!windowVisible);
  render(await snapshot());
  setInterval(() => { if (latest && !document.hidden && windowVisible) render(latest); }, 1_000);
  document.addEventListener('visibilitychange', () => {
    document.body.classList.toggle('pet-paused', document.hidden||!windowVisible);
    if (!document.hidden && windowVisible && latest) render(latest);
  });
  await desktopCommand('ui_ready');
}
void start().catch(showError);
