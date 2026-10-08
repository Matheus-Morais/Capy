import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';
import artwork from '../prototypes/front-pet.svg?raw';
import artworkV1 from '../prototypes/front-pet.v1.svg?raw';
import artworkV2 from '../prototypes/front-pet.v2.svg?raw';
import artworkV3 from '../prototypes/front-pet.v3.svg?raw';
import artworkV4 from '../prototypes/front-pet.v4.svg?raw';
import '../prototypes/pet.css';
import '../prototypes/front-pet.css';
import './style.css';
import { desktopCommand, native, showError, snapshot, subscribe, subscribeVisibility, type Snapshot } from './bridge';
import { PetBehavior, hoverGaze } from './pet-behavior';
import { playPetSound } from './pet-sound';

let currentSkin = (typeof localStorage !== 'undefined' && localStorage.getItem('capy_pet_skin')) || 'v5';
let currentHat = (typeof localStorage !== 'undefined' && localStorage.getItem('capy_hat')) || 'none';
let currentClothes = (typeof localStorage !== 'undefined' && localStorage.getItem('capy_clothes')) || 'none';
let currentCostume = (typeof localStorage !== 'undefined' && localStorage.getItem('capy_costume')) || 'none';
let currentFriend = (typeof localStorage !== 'undefined' && localStorage.getItem('capy_friend')) || 'none';
let currentEnv = (typeof localStorage !== 'undefined' && localStorage.getItem('capy_env')) || 'auto';

const HATS = ['none', 'cap', 'straw', 'top', 'beanie', 'flower'];
const CLOTHES = ['none', 'scarf', 'bowtie', 'hoodie', 'raincoat', 'sunglasses'];
const COSTUMES = ['none', 'wizard', 'dino', 'pirate', 'detective', 'royal'];
const FRIENDS = ['none', 'bird', 'butterfly', 'duck', 'turtle', 'all'];
const ENVIRONMENTS = ['auto', 'day', 'sunset', 'night', 'rain'];

const DEV_TIPS = [
  'Git push feito! 🚀',
  'Compilando... Tudo verde! ✅',
  'Não esquece o git commit! 💡',
  'Seu código tá lindo hoje! ✨',
  'Café recarregado, foco total! ☕',
  '34 testes passando! 🧪',
  'Respira fundo e refatora! 🦫',
  'Hora de hidratar a mente! 💧',
  'Você manda muito bem! 🌟'
];

const artworkContainer = document.getElementById('petArtwork')!;
const button = document.getElementById('petToggle') as HTMLButtonElement;
button.title = 'Capy · Atalhos: V (skins), 1-3 (roupas/fantasias), 4 (amiguinhos), 5 (clima), J (dica), L (saúde), T (dançar), A (andar), R (girar/spin), E (espreguiçar), G (rebolar), Z (bater pata), 0 (limpar), C (carinho), B (bocejo), S (sacudir), K (café), W (ofurô), P (lofi), O (laranja), H (high-five), D (sonho), M (lanche).';

function applyWardrobe(hat?: string, clothes?: string, costume?: string) {
  if (hat !== undefined) currentHat = hat;
  if (clothes !== undefined) currentClothes = clothes;
  if (costume !== undefined) currentCostume = costume;
  try {
    localStorage.setItem('capy_hat', currentHat);
    localStorage.setItem('capy_clothes', currentClothes);
    localStorage.setItem('capy_costume', currentCostume);
  } catch {}
  for (const c of ['w-hat-cap', 'w-hat-straw', 'w-hat-top', 'w-hat-beanie', 'w-hat-flower']) pet.classList.remove(c);
  for (const c of ['w-cloth-sunglasses', 'w-cloth-scarf', 'w-cloth-bowtie', 'w-cloth-hoodie', 'w-cloth-raincoat']) pet.classList.remove(c);
  for (const c of ['w-costume-wizard', 'w-costume-dino', 'w-costume-pirate', 'w-costume-detective', 'w-costume-royal']) pet.classList.remove(c);
  if (currentHat && currentHat !== 'none') pet.classList.add(`w-hat-${currentHat}`);
  if (currentClothes && currentClothes !== 'none') pet.classList.add(`w-cloth-${currentClothes}`);
  if (currentCostume && currentCostume !== 'none') pet.classList.add(`w-costume-${currentCostume}`);
}

function applyFriend(friend?: string) {
  if (friend !== undefined) currentFriend = friend;
  try { localStorage.setItem('capy_friend', currentFriend); } catch {}
  for (const f of ['has-friend-bird', 'has-friend-butterfly', 'has-friend-duck', 'has-friend-turtle', 'has-friend-all']) pet.classList.remove(f);
  if (currentFriend && currentFriend !== 'none') pet.classList.add(`has-friend-${currentFriend}`);
}

function applyEnvironment(env?: string) {
  if (env !== undefined) currentEnv = env;
  try { localStorage.setItem('capy_env', currentEnv); } catch {}
  for (const e of ['env-day', 'env-sunset', 'env-night', 'env-rain']) pet.classList.remove(e);
  let resolved = currentEnv;
  if (resolved === 'auto') {
    const hour = new Date().getHours();
    resolved = (hour >= 6 && hour < 17) ? 'day' : (hour >= 17 && hour < 19) ? 'sunset' : 'night';
  }
  if (resolved !== 'none') pet.classList.add(`env-${resolved}`);
}

let speechTimeout: number | undefined;
function showSpeechBubble(text: string, durationMs = 3500) {
  const textElem = pet.querySelector('.speech-text');
  if (textElem) textElem.textContent = text;
  pet.classList.remove('has-speech');
  void pet.getBoundingClientRect();
  pet.classList.add('has-speech');
  if (speechTimeout) clearTimeout(speechTimeout);
  speechTimeout = window.setTimeout(() => pet.classList.remove('has-speech'), durationMs);
}

let lastWellness = 0;
function triggerWellnessReminder() {
  lastWellness = (lastWellness + 1) % 2;
  if (lastWellness === 0) {
    pet.classList.add('s-water');
    showSpeechBubble('Hora de se hidratar! 💧', 3500);
    if (latest?.preferences?.sounds) playPetSound('sip');
    setTimeout(() => pet.classList.remove('s-water'), 3500);
  } else {
    triggerPlayfulGesture('g-yawn', 2000);
    showSpeechBubble('Alongue as costas e respire! 🧘', 3500);
    if (latest?.preferences?.sounds) playPetSound('chirp');
  }
}

function applySkin(skin: string) {
  currentSkin = skin;
  try { localStorage.setItem('capy_pet_skin', skin); } catch {}
  artworkContainer.innerHTML = skin === 'v1' ? artworkV1 : skin === 'v2' ? artworkV2 : skin === 'v3' ? artworkV3 : skin === 'v4' ? artworkV4 : artwork;
  pet = document.getElementById('pet')!;
  applyWardrobe();
  applyFriend();
  applyEnvironment();
}

applySkin(currentSkin);
let pet = document.getElementById('pet')!;
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
function triggerPlayfulGesture(gestureClass: string, durationMs: number) {
  for (const value of ['g-love', 'g-yawn', 'g-shake', 'g-click', 'g-wave', 'g-hello', 'g-orange-trick', 'g-highfive', 'g-dream', 'g-dance', 'g-waddle', 'g-spin', 'g-stretch', 'g-wiggle', 'g-tap']) pet.classList.remove(value);
  void pet.getBoundingClientRect();
  pet.classList.add(gestureClass);
  setTimeout(() => pet.classList.remove(gestureClass), durationMs);
}
function toggleCompanionState(stateClass: string) {
  const willEnable = !pet.classList.contains(stateClass);
  for (const s of ['s-snack', 's-coffee', 's-bath', 's-music']) pet.classList.remove(s);
  if (willEnable) pet.classList.add(stateClass);
}
button.addEventListener('dblclick', event => {
  event.preventDefault();
  triggerPlayfulGesture('g-love', 1800);
});
button.addEventListener('keydown', event => {
  const key = event.key.toLowerCase();
  if (key === 'v') {
    event.preventDefault();
    const next = currentSkin === 'v5' ? 'v4' : currentSkin === 'v4' ? 'v3' : currentSkin === 'v3' ? 'v2' : currentSkin === 'v2' ? 'v1' : 'v5';
    applySkin(next);
    if (latest) render(latest);
    return;
  }
  if (key === 'c') {
    event.preventDefault();
    triggerPlayfulGesture('g-love', 1800);
    if (latest?.preferences?.sounds) playPetSound('purr');
    return;
  }
  if (key === 'b') {
    event.preventDefault();
    triggerPlayfulGesture('g-yawn', 1600);
    if (latest?.preferences?.sounds) playPetSound('purr');
    return;
  }
  if (key === 's') {
    event.preventDefault();
    triggerPlayfulGesture('g-shake', 650);
    return;
  }
  if (key === 'k') {
    event.preventDefault();
    toggleCompanionState('s-coffee');
    if (latest?.preferences?.sounds) playPetSound('sip');
    return;
  }
  if (key === 'w') {
    event.preventDefault();
    toggleCompanionState('s-bath');
    if (latest?.preferences?.sounds) playPetSound('bubble');
    return;
  }
  if (key === 'p') {
    event.preventDefault();
    toggleCompanionState('s-music');
    return;
  }
  if (key === 'o') {
    event.preventDefault();
    triggerPlayfulGesture('g-orange-trick', 1800);
    if (latest?.preferences?.sounds) playPetSound('sparkle');
    return;
  }
  if (key === 'h') {
    event.preventDefault();
    triggerPlayfulGesture('g-highfive', 1500);
    if (latest?.preferences?.sounds) playPetSound('sparkle');
    return;
  }
  if (key === 'd') {
    event.preventDefault();
    triggerPlayfulGesture('g-dream', 2500);
    return;
  }
  if (key === 'm') {
    event.preventDefault();
    toggleCompanionState('s-snack');
    if (latest?.preferences?.sounds) playPetSound('crunch');
    return;
  }
  if (key === 't') {
    event.preventDefault();
    triggerPlayfulGesture('g-dance', 2400);
    if (latest?.preferences?.sounds) playPetSound('sparkle');
    return;
  }
  if (key === 'a') {
    event.preventDefault();
    triggerPlayfulGesture('g-waddle', 2000);
    if (latest?.preferences?.sounds) playPetSound('bubble');
    return;
  }
  if (key === 'r') {
    event.preventDefault();
    triggerPlayfulGesture('g-spin', 1400);
    if (latest?.preferences?.sounds) playPetSound('celebrate');
    return;
  }
  if (key === 'e') {
    event.preventDefault();
    triggerPlayfulGesture('g-stretch', 2400);
    if (latest?.preferences?.sounds) playPetSound('purr');
    return;
  }
  if (key === 'g') {
    event.preventDefault();
    triggerPlayfulGesture('g-wiggle', 1800);
    if (latest?.preferences?.sounds) playPetSound('chirp');
    return;
  }
  if (key === 'z') {
    event.preventDefault();
    triggerPlayfulGesture('g-tap', 1600);
    if (latest?.preferences?.sounds) playPetSound('crunch');
    return;
  }
  if (key === '1') {
    event.preventDefault();
    const nextIdx = (HATS.indexOf(currentHat) + 1) % HATS.length;
    applyWardrobe(HATS[nextIdx], currentClothes, currentCostume);
    return;
  }
  if (key === '2') {
    event.preventDefault();
    const nextIdx = (CLOTHES.indexOf(currentClothes) + 1) % CLOTHES.length;
    applyWardrobe(currentHat, CLOTHES[nextIdx], currentCostume);
    return;
  }
  if (key === '3') {
    event.preventDefault();
    const nextIdx = (COSTUMES.indexOf(currentCostume) + 1) % COSTUMES.length;
    applyWardrobe(currentHat, currentClothes, COSTUMES[nextIdx]);
    return;
  }
  if (key === '4') {
    event.preventDefault();
    const nextIdx = (FRIENDS.indexOf(currentFriend) + 1) % FRIENDS.length;
    applyFriend(FRIENDS[nextIdx]);
    if (latest?.preferences?.sounds && FRIENDS[nextIdx] !== 'none') playPetSound('chirp');
    return;
  }
  if (key === '5') {
    event.preventDefault();
    const nextIdx = (ENVIRONMENTS.indexOf(currentEnv) + 1) % ENVIRONMENTS.length;
    applyEnvironment(ENVIRONMENTS[nextIdx]);
    if (latest?.preferences?.sounds) playPetSound('sparkle');
    return;
  }
  if (key === 'j') {
    event.preventDefault();
    const randomTip = DEV_TIPS[Math.floor(Math.random() * DEV_TIPS.length)];
    showSpeechBubble(randomTip);
    if (latest?.preferences?.sounds) playPetSound('sparkle');
    return;
  }
  if (key === 'l') {
    event.preventDefault();
    triggerWellnessReminder();
    return;
  }
  if (key === '0') {
    event.preventDefault();
    applyWardrobe('none', 'none', 'none');
    applyFriend('none');
    applyEnvironment('none');
    return;
  }
  const directions: Record<string, [number, number]> = { ArrowLeft: [-16, 0], ArrowRight: [16, 0], ArrowUp: [0, -16], ArrowDown: [0, 16] };
  const direction = directions[event.key];
  if (direction) { event.preventDefault(); void desktopCommand('move_pet', { dx: direction[0], dy: direction[1] }).catch(showError); }
});
window.addEventListener('storage', event => {
  if (event.key === 'capy_pet_skin' && event.newValue && event.newValue !== currentSkin) {
    applySkin(event.newValue);
    if (latest) render(latest);
  }
  if (event.key === 'capy_wardrobe' || event.key === 'capy_hat' || event.key === 'capy_clothes' || event.key === 'capy_costume') {
    currentHat = localStorage.getItem('capy_hat') || 'none';
    currentClothes = localStorage.getItem('capy_clothes') || 'none';
    currentCostume = localStorage.getItem('capy_costume') || 'none';
    applyWardrobe();
  }
  if (event.key === 'capy_friend') {
    currentFriend = localStorage.getItem('capy_friend') || 'none';
    applyFriend(currentFriend);
  }
  if (event.key === 'capy_env') {
    currentEnv = localStorage.getItem('capy_env') || 'auto';
    applyEnvironment(currentEnv);
  }
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
