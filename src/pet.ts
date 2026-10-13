import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';
import artwork from '../prototypes/front-pet.svg?raw';
import artworkV1 from '../prototypes/front-pet.v1.svg?raw';
import artworkV2 from '../prototypes/front-pet.v2.svg?raw';
import artworkV3 from '../prototypes/front-pet.v3.svg?raw';
import artworkV4 from '../prototypes/front-pet.v4.svg?raw';
import '../prototypes/front-pet.css';
import './style.css';
import { desktopCommand, native, showError, snapshot, subscribe, subscribeVisibility, type Snapshot } from './bridge';
import { PetBehavior, hoverGaze } from './pet-behavior';
import { playPetSound } from './pet-sound';
import { PetLifeEngine } from './pet-life';
import { checkAndApplyUpdates } from './updater';

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
  'Confira o destino antes do git push! 🚀',
  'Confira a build antes de entregar! ✅',
  'Não esquece o git commit! 💡',
  'Seu código tá lindo hoje! ✨',
  'Café recarregado, foco total! ☕',
  'Rode os testes da sua mudança! 🧪',
  'Respira fundo e refatora! 🦫',
  'Hora de hidratar a mente! 💧',
  'Você manda muito bem! 🌟'
];

const artworkContainer = document.getElementById('petArtwork')!;
const button = document.getElementById('petToggle') as HTMLButtonElement;
button.title = 'Capy · Atalhos: V (skins), 1-3 (roupas/fantasias), 4 (amiguinhos), 5 (clima), U (atualizar), J (dica), L (saúde), T (dançar), A (andar), R (girar/spin), E (espreguiçar), G (rebolar), Z (bater pata), 0 (limpar), C (carinho), B (bocejo), S (sacudir), K (café), W (ofurô), P (lofi), O (laranja), H (high-five), D (sonho), M (lanche).';

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

let pet: HTMLElement;

// ===== Continuous Gaze Lerp Loop (rAF Exponential Smoothing) =====
let targetGazeX = 0;
let targetGazeY = 0;
let currentGazeX = 0;
let currentGazeY = 0;
let gazeRafId: number | null = null;

function updateGazeLoop() {
  const dx = targetGazeX - currentGazeX;
  const dy = targetGazeY - currentGazeY;

  // Settle threshold: when within 0.02px, sleep the rAF loop to consume 0% CPU at rest
  if (Math.abs(dx) < 0.02 && Math.abs(dy) < 0.02) {
    currentGazeX = targetGazeX;
    currentGazeY = targetGazeY;
    if (targetGazeX === 0 && targetGazeY === 0) {
      if (pet?.style) {
        pet.style.removeProperty('--gaze-x');
        pet.style.removeProperty('--gaze-y');
      }
    } else if (pet?.style) {
      pet.style.setProperty('--gaze-x', `${currentGazeX.toFixed(2)}px`);
      pet.style.setProperty('--gaze-y', `${currentGazeY.toFixed(2)}px`);
    }
    gazeRafId = null;
    return;
  }

  // Exponential smoothing factor (0.16 gives buttery smooth tracking at 60-144Hz)
  currentGazeX += dx * 0.16;
  currentGazeY += dy * 0.16;
  if (pet?.style) {
    pet.style.setProperty('--gaze-x', `${currentGazeX.toFixed(2)}px`);
    pet.style.setProperty('--gaze-y', `${currentGazeY.toFixed(2)}px`);
  }

  if (typeof requestAnimationFrame !== 'undefined') {
    gazeRafId = requestAnimationFrame(updateGazeLoop);
  } else {
    gazeRafId = null;
  }
}

function applySmoothGaze(targetX: number, targetY: number) {
  if (latest?.reduceMotion || (pet && pet.classList.contains('s-sleeping'))) {
    targetX = 0;
    targetY = 0;
  }
  targetGazeX = targetX;
  targetGazeY = targetY;

  if (typeof requestAnimationFrame === 'undefined') {
    // Fallback for Node.js test environment where rAF is absent
    currentGazeX = targetX;
    currentGazeY = targetY;
    if (pet?.style) {
      pet.style.setProperty('--gaze-x', `${targetX}px`);
      pet.style.setProperty('--gaze-y', `${targetY}px`);
    }
    return;
  }

  if (gazeRafId === null) {
    gazeRafId = requestAnimationFrame(updateGazeLoop);
  }
}

function resetSmoothGaze() {
  applySmoothGaze(0, 0);
}

applySkin(currentSkin);
const badge = document.getElementById('petBadge')!;
const behavior = new PetBehavior(Date.now());
let forcePauseWork = false;
let latest: Snapshot | undefined;
let lastGestureId = -1;
let windowVisible = true;

const lifeEngine = new PetLifeEngine({
  applyGesture(className, durationMs, sound) {
    if (latest?.reduceMotion || !windowVisible || document.hidden) return;
    pet.classList.add(className);
    if (sound && latest?.preferences?.sounds) {
      playPetSound(sound as any);
    }
  },
  removeGesture(className) {
    pet.classList.remove(className);
  },
  setGaze(x, y) {
    if (latest?.reduceMotion || pet.classList.contains('is-hovered')) return;
    applySmoothGaze(x, y);
  },
  canFidget() {
    if (!latest || latest.reduceMotion || document.hidden || !windowVisible) return false;
    const isSleeping = pet.classList.contains('s-sleeping');
    const isWaiting = pet.classList.contains('s-waiting');
    const isHovered = pet.classList.contains('is-hovered');
    const waitingCount = parseInt(badge.textContent || '0', 10);
    return !isSleeping && !isWaiting && !isHovered && waitingCount === 0;
  }
});

function render(data: Snapshot) {
  latest = data;
  const frame = behavior.update(data, Date.now());
  let {state, waiting, gesture} = frame;
  if (forcePauseWork && state === 'working') {
    state = 'idle';
  }
  if (state === 'sleeping' || gesture !== '' || waiting > 0) {
    lifeEngine.cancelActiveGesture();
  }
  if (state === 'sleeping' || data.reduceMotion) {
    resetSmoothGaze();
  }
  for (const value of ['idle', 'working', 'waiting', 'done', 'sleeping']) pet.classList.toggle(`s-${value}`, state === value);
  for (const pose of ['sit', 'lie', 'stand']) pet.classList.toggle(`p-${pose}`, pose === (state === 'working' ? 'sit' : state === 'sleeping' ? 'lie' : 'stand'));
  if (lastGestureId !== frame.gestureId) {
    if (windowVisible && !document.hidden && data.preferences?.sounds && (gesture === 'hello' || gesture === 'wave' || gesture === 'celebrate')) playPetSound(gesture);
    for (const value of ['hello', 'wave', 'click', 'celebrate']) pet.classList.remove(`g-${value}`);
    if (gesture) void pet.getBoundingClientRect();
    lastGestureId = frame.gestureId;
  }
  if (gesture) pet.classList.add(`g-${gesture}`);
  if (!gesture) for (const value of ['hello', 'wave', 'click', 'celebrate']) pet.classList.remove(`g-${value}`);
  badge.hidden = !waiting;
  badge.textContent = String(waiting);
  const statusText = waiting ? `${waiting} precisam de você` : gesture === 'celebrate' ? state === 'working' ? 'Conclusão confirmada · outros agentes trabalhando' : 'Conclusão confirmada' : state === 'working' ? 'Agentes trabalhando' : forcePauseWork ? 'Pausa da Capy ☕' : state === 'idle' ? 'Sessões abertas' : 'Capy descansando';
  document.getElementById('petStatus')!.textContent = statusText;
  button.title = `Capy: ${statusText}\nClique para abrir. Arraste para mover.`;
  document.body.classList.toggle('reduce-motion', data.reduceMotion);
}

let pointer: { x: number; y: number; dragged: boolean } | null = null;
let suppressClick = false;
button.addEventListener('pointerdown', event => {
  if (event.button !== 0) return;
  pointer = { x: event.clientX, y: event.clientY, dragged: false };
  suppressClick = false;
  lifeEngine.onDragStart(event.clientX, event.clientY);
});
button.addEventListener('pointerenter', () => {
  behavior.interact(Date.now());
  pet.classList.add('is-hovered');
  if (latest) render(latest);
});
button.addEventListener('pointerleave', () => {
  pet.classList.remove('is-hovered');
  lifeEngine.clearGlobalGaze();
  pet.style.removeProperty('--gaze-x');
  pet.style.removeProperty('--gaze-y');
  targetGazeX = 0;
  targetGazeY = 0;
  currentGazeX = 0;
  currentGazeY = 0;
  gazeRafId = null;
});
button.addEventListener('pointermove', event => {
  if (pointer || !pet.classList.contains('is-hovered')) return;
  const rect = button.getBoundingClientRect();
  const [x, y] = hoverGaze(event.clientX - rect.left, event.clientY - rect.top, rect.width, rect.height);
  applySmoothGaze(x, y);
});
button.addEventListener('pointermove', event => {
  if (!pointer || !event.buttons) return;
  if (!pointer.dragged && Math.hypot(event.clientX - pointer.x, event.clientY - pointer.y) > 5) {
    pointer.dragged = true;
    suppressClick = true;
    pet.classList.add('is-dragging');
    if (native) void getCurrentWindow().startDragging().catch(showError);
  }
  if (pointer.dragged) {
    const dynamics = lifeEngine.onDragMove(event.clientX, event.clientY);
    pet.style.setProperty('--drag-tilt', `${dynamics.tilt}deg`);
    pet.style.setProperty('--drag-scale-x', `${dynamics.scaleX}`);
    pet.style.setProperty('--drag-scale-y', `${dynamics.scaleY}`);
  }
});
function endDrag() {
  if (pointer?.dragged) {
    pet.classList.remove('is-dragging');
    pet.style.removeProperty('--drag-tilt');
    pet.style.removeProperty('--drag-scale-x');
    pet.style.removeProperty('--drag-scale-y');
    pet.classList.add('g-bounce-land');
    setTimeout(() => pet.classList.remove('g-bounce-land'), 450);
  }
  pointer = null;
}
button.addEventListener('pointerup', endDrag);
button.addEventListener('pointercancel', endDrag);
button.addEventListener('click', () => {
  if (suppressClick) { suppressClick = false; return; }
  behavior.interact(Date.now(), true);
  if (latest) render(latest);
  void desktopCommand('toggle_summary').catch(showError);
});
function triggerPlayfulGesture(gestureClass: string, durationMs: number) {
  lifeEngine.cancelActiveGesture();
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
button.addEventListener('contextmenu', event => {
  event.preventDefault();
});

function handlePetKeydown(event: KeyboardEvent) {
  const key = event.key.toLowerCase();
  if (key === 'x') {
    event.preventDefault();
    forcePauseWork = !forcePauseWork;
    showSpeechBubble(forcePauseWork ? 'Pausa para café! ☕ Capy livre!' : 'De volta ao foco! 💻', 2500);
    if (latest?.preferences?.sounds) playPetSound(forcePauseWork ? 'sip' : 'sparkle');
    if (latest) render(latest);
    return;
  }
  if (key === 'v') {
    event.preventDefault();
    const next = currentSkin === 'v5' ? 'v4' : currentSkin === 'v4' ? 'v3' : currentSkin === 'v3' ? 'v2' : currentSkin === 'v2' ? 'v1' : 'v5';
    applySkin(next);
    showSpeechBubble(`Capy Skin: ${next.toUpperCase()} ✨`, 1800);
    if (latest) render(latest);
    return;
  }
  if (key === 'c') {
    event.preventDefault();
    triggerPlayfulGesture('g-love', 1800);
    showSpeechBubble('Muuuita fofura! ❤️', 1800);
    if (latest?.preferences?.sounds) playPetSound('purr');
    return;
  }
  if (key === 'b') {
    event.preventDefault();
    triggerPlayfulGesture('g-yawn', 1600);
    showSpeechBubble('Que soninho gostoso... 🥱', 1800);
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
    showSpeechBubble('Cafézinho quente! ☕', 2000);
    if (latest?.preferences?.sounds) playPetSound('sip');
    return;
  }
  if (key === 'w') {
    event.preventDefault();
    toggleCompanionState('s-bath');
    showSpeechBubble('Banho relaxante de ofurô! 🛁', 2200);
    if (latest?.preferences?.sounds) playPetSound('bubble');
    return;
  }
  if (key === 'p') {
    event.preventDefault();
    toggleCompanionState('s-music');
    showSpeechBubble('Lofi Beats na vibe! 🎵', 2000);
    return;
  }
  if (key === 'o') {
    event.preventDefault();
    triggerPlayfulGesture('g-orange-trick', 1800);
    showSpeechBubble('Equilibrando a laranja! 🍊', 2000);
    if (latest?.preferences?.sounds) playPetSound('sparkle');
    return;
  }
  if (key === 'h') {
    event.preventDefault();
    triggerPlayfulGesture('g-highfive', 1500);
    showSpeechBubble('Toca aqui! ✋🎉', 1800);
    if (latest?.preferences?.sounds) playPetSound('sparkle');
    return;
  }
  if (key === 'd') {
    event.preventDefault();
    triggerPlayfulGesture('g-dream', 2500);
    showSpeechBubble('Pensando em código limpo... 💭', 2200);
    return;
  }
  if (key === 'm') {
    event.preventDefault();
    toggleCompanionState('s-snack');
    showSpeechBubble('Nhac nhac nhac! 🌿', 2000);
    if (latest?.preferences?.sounds) playPetSound('crunch');
    return;
  }
  if (key === 't') {
    event.preventDefault();
    triggerPlayfulGesture('g-dance', 2400);
    showSpeechBubble('Hora da dancinha! 🕺✨', 2200);
    if (latest?.preferences?.sounds) playPetSound('sparkle');
    return;
  }
  if (key === 'a') {
    event.preventDefault();
    triggerPlayfulGesture('g-waddle', 2000);
    showSpeechBubble('Passeando fofinha! 🐾', 2000);
    if (latest?.preferences?.sounds) playPetSound('bubble');
    return;
  }
  if (key === 'r') {
    event.preventDefault();
    triggerPlayfulGesture('g-spin', 1400);
    showSpeechBubble('Rodopiando! 🌀', 1600);
    if (latest?.preferences?.sounds) playPetSound('celebrate');
    return;
  }
  if (key === 'e') {
    event.preventDefault();
    triggerPlayfulGesture('g-stretch', 2400);
    showSpeechBubble('Alongando as costas! 🧘', 2000);
    if (latest?.preferences?.sounds) playPetSound('purr');
    return;
  }
  if (key === 'g') {
    event.preventDefault();
    triggerPlayfulGesture('g-wiggle', 1800);
    showSpeechBubble('Reboladinha alegre! 💃', 1800);
    if (latest?.preferences?.sounds) playPetSound('chirp');
    return;
  }
  if (key === 'z') {
    event.preventDefault();
    triggerPlayfulGesture('g-tap', 1600);
    showSpeechBubble('Batendo palminhas! 👏', 1800);
    if (latest?.preferences?.sounds) playPetSound('crunch');
    return;
  }
  if (key === 'u') {
    event.preventDefault();
    showSpeechBubble('Verificando atualizações... 🔍', 3000);
    if (latest?.preferences?.sounds) playPetSound('sparkle');
    void checkAndApplyUpdates((msg, duration) => showSpeechBubble(msg, duration), true).catch(() => {});
    return;
  }
  if (key === '1') {
    event.preventDefault();
    const nextIdx = (HATS.indexOf(currentHat) + 1) % HATS.length;
    applyWardrobe(HATS[nextIdx], currentClothes, currentCostume);
    showSpeechBubble(`Chapéu: ${HATS[nextIdx]} 🎩`, 1800);
    return;
  }
  if (key === '2') {
    event.preventDefault();
    const nextIdx = (CLOTHES.indexOf(currentClothes) + 1) % CLOTHES.length;
    applyWardrobe(currentHat, CLOTHES[nextIdx], currentCostume);
    showSpeechBubble(`Roupinha: ${CLOTHES[nextIdx]} 👕`, 1800);
    return;
  }
  if (key === '3') {
    event.preventDefault();
    const nextIdx = (COSTUMES.indexOf(currentCostume) + 1) % COSTUMES.length;
    applyWardrobe(currentHat, currentClothes, COSTUMES[nextIdx]);
    showSpeechBubble(`Fantasia: ${COSTUMES[nextIdx]} 🧙`, 1800);
    return;
  }
  if (key === '4') {
    event.preventDefault();
    const nextIdx = (FRIENDS.indexOf(currentFriend) + 1) % FRIENDS.length;
    applyFriend(FRIENDS[nextIdx]);
    showSpeechBubble(`Amiguinho: ${FRIENDS[nextIdx]} 🐾`, 1800);
    if (latest?.preferences?.sounds && FRIENDS[nextIdx] !== 'none') playPetSound('chirp');
    return;
  }
  if (key === '5') {
    event.preventDefault();
    const nextIdx = (ENVIRONMENTS.indexOf(currentEnv) + 1) % ENVIRONMENTS.length;
    applyEnvironment(ENVIRONMENTS[nextIdx]);
    showSpeechBubble(`Cenário: ${ENVIRONMENTS[nextIdx]} 🌅`, 1800);
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
    showSpeechBubble('Resetado ao padrão! ✨', 1800);
    return;
  }
  const directions: Record<string, [number, number]> = { ArrowLeft: [-16, 0], ArrowRight: [16, 0], ArrowUp: [0, -16], ArrowDown: [0, 16] };
  const direction = directions[event.key];
  if (direction) { event.preventDefault(); void desktopCommand('move_pet', { dx: direction[0], dy: direction[1] }).catch(showError); }
}

button.addEventListener('keydown', handlePetKeydown);
if (typeof window !== 'undefined') {
  window.addEventListener('keydown', handlePetKeydown);
}
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
      else { resetSmoothGaze(); }
    });
    await listen('attention-viewed',()=>{behavior.acknowledge();behavior.interact(Date.now());if(latest)render(latest);});
    await listen<{ x: number; y: number }>('global-gaze', event => {
      if (!pet.classList.contains('is-hovered') && !pet.classList.contains('s-sleeping') && !latest?.reduceMotion) {
        lifeEngine.setGlobalGaze(event.payload.x, event.payload.y);
      }
    });
    setTimeout(() => {
      void checkAndApplyUpdates((msg, duration) => showSpeechBubble(msg, duration)).catch(() => {});
    }, 15_000);
  }
  document.body.classList.toggle('pet-paused',document.hidden||!windowVisible);
  render(await snapshot());
  lifeEngine.start();
  setInterval(() => { if (latest && !document.hidden && windowVisible) render(latest); }, 1_000);
  document.addEventListener('visibilitychange', () => {
    document.body.classList.toggle('pet-paused', document.hidden||!windowVisible);
    if (!document.hidden && windowVisible && latest) render(latest);
    else { resetSmoothGaze(); }
  });
  await desktopCommand('ui_ready');
}
void start().catch(showError);
