export type FidgetName =
  | 'ear-twitch'
  | 'sniff'
  | 'yawn'
  | 'stretch'
  | 'shake'
  | 'tap'
  | 'wiggle'
  | 'orange-trick'
  | 'snack'
  | 'purr'
  | 'curious';

export interface FidgetConfig {
  name: FidgetName;
  className: string;
  durationMs: number;
  weight: number;
  sound?: 'purr' | 'crunch' | 'sparkle' | 'chirp' | 'bubble';
  cooldownMs: number;
}

export const FIDGET_CATALOG: FidgetConfig[] = [
  { name: 'ear-twitch', className: 'g-ear-twitch', durationMs: 1200, weight: 26, cooldownMs: 8000 },
  { name: 'sniff', className: 'g-sniff', durationMs: 1300, weight: 24, cooldownMs: 9000 },
  { name: 'yawn', className: 'g-yawn', durationMs: 1800, weight: 14, sound: 'purr', cooldownMs: 24000 },
  { name: 'curious', className: 'g-look-up', durationMs: 2400, weight: 18, cooldownMs: 14000 },
  { name: 'orange-trick', className: 'g-orange-trick', durationMs: 1800, weight: 12, sound: 'sparkle', cooldownMs: 28000 },
  { name: 'shake', className: 'g-shake', durationMs: 700, weight: 12, cooldownMs: 22000 },
  { name: 'tap', className: 'g-tap', durationMs: 1600, weight: 14, sound: 'crunch', cooldownMs: 18000 },
  { name: 'wiggle', className: 'g-wiggle', durationMs: 1800, weight: 12, sound: 'chirp', cooldownMs: 25000 },
  { name: 'stretch', className: 'g-stretch', durationMs: 2400, weight: 10, sound: 'purr', cooldownMs: 32000 },
  { name: 'snack', className: 's-snack', durationMs: 3000, weight: 10, sound: 'crunch', cooldownMs: 36000 },
  { name: 'purr', className: 'g-love', durationMs: 1800, weight: 8, sound: 'purr', cooldownMs: 40000 },
];

function safeTimeout(fn: () => void, delayMs: number): any {
  const t = setTimeout(fn, delayMs);
  if (typeof (t as any)?.unref === 'function') {
    (t as any).unref();
  }
  return t;
}

export interface PetLifeDelegate {
  applyGesture(className: string, durationMs: number, sound?: string): void;
  removeGesture(className: string): void;
  setGaze(x: number, y: number): void;
  canFidget(): boolean;
}

export class PetLifeEngine {
  private readonly delegate: PetLifeDelegate;
  private timer: number | null = null;
  private gazeTimer: number | null = null;
  private lastTriggeredTimes = new Map<FidgetName, number>();
  private activeGesture: string | null = null;
  private activeGestureTimeout: number | null = null;
  private lastDragX = 0;
  private lastDragY = 0;
  private globalGazeActive = false;

  constructor(delegate: PetLifeDelegate) {
    this.delegate = delegate;
  }

  start(): void {
    this.scheduleNextFidget();
    this.scheduleNextGazeDrift();
  }

  stop(): void {
    if (this.timer) {
      clearTimeout(this.timer);
      this.timer = null;
    }
    if (this.gazeTimer) {
      clearTimeout(this.gazeTimer);
      this.gazeTimer = null;
    }
    this.cancelActiveGesture();
  }

  cancelActiveGesture(): void {
    if (this.activeGestureTimeout) {
      clearTimeout(this.activeGestureTimeout);
      this.activeGestureTimeout = null;
    }
    if (this.activeGesture) {
      this.delegate.removeGesture(this.activeGesture);
      this.activeGesture = null;
    }
  }

  private scheduleNextFidget(): void {
    if (this.timer) clearTimeout(this.timer);
    // Intervalo dinâmico e natural: a cada 7 a 15 segundos sorteia um micro-movimento
    const delay = 7000 + Math.random() * 8000;
    this.timer = safeTimeout(() => {
      this.tryTriggerFidget();
      this.scheduleNextFidget();
    }, delay);
  }

  private tryTriggerFidget(): void {
    if (!this.delegate.canFidget() || this.activeGesture) {
      return;
    }

    const now = Date.now();
    const available = FIDGET_CATALOG.filter(item => {
      const last = this.lastTriggeredTimes.get(item.name) ?? 0;
      return now - last >= item.cooldownMs;
    });

    if (available.length === 0) return;

    const totalWeight = available.reduce((acc, cur) => acc + cur.weight, 0);
    let rand = Math.random() * totalWeight;
    const picked = available.find(item => {
      rand -= item.weight;
      return rand <= 0;
    }) ?? available[0];

    this.lastTriggeredTimes.set(picked.name, now);
    this.activeGesture = picked.className;

    this.delegate.applyGesture(picked.className, picked.durationMs, picked.sound);

    this.activeGestureTimeout = safeTimeout(() => {
      if (this.activeGesture === picked.className) {
        this.delegate.removeGesture(picked.className);
        this.activeGesture = null;
      }
      this.activeGestureTimeout = null;
    }, picked.durationMs);
  }

  private scheduleNextGazeDrift(): void {
    if (this.gazeTimer) clearTimeout(this.gazeTimer);
    // Intervalo de desvio do olhar natural: a cada 3 a 7 segundos
    const delay = 3200 + Math.random() * 4000;
    this.gazeTimer = safeTimeout(() => {
      this.tryGazeDrift();
      this.scheduleNextGazeDrift();
    }, delay);
  }

  private tryGazeDrift(): void {
    // Se o mouse global estiver controlando ativamente ou o pet não puder se mexer, respeita
    if (this.globalGazeActive || !this.delegate.canFidget()) {
      return;
    }

    // Olhar curioso para diferentes quadrantes do monitor (onde o dev costuma estar)
    const options: [number, number][] = [
      [0, 0],         // Centro / frente
      [-2.2, 0.4],    // Olhando à esquerda (código)
      [-1.4, -0.8],   // Olhando para cima/esquerda
      [1.8, 0.2],     // Olhando à direita
      [0, -1.2],      // Olhando para cima (pensativa)
      [-0.8, 0.8],    // Olhando levemente para baixo
    ];
    const target = options[Math.floor(Math.random() * options.length)];
    this.delegate.setGaze(target[0], target[1]);
  }

  setGlobalGaze(x: number, y: number): void {
    this.globalGazeActive = true;
    this.delegate.setGaze(x, y);
  }

  clearGlobalGaze(): void {
    this.globalGazeActive = false;
  }

  onDragStart(clientX: number, clientY: number): void {
    this.lastDragX = clientX;
    this.lastDragY = clientY;
    this.cancelActiveGesture();
  }

  onDragMove(clientX: number, clientY: number): { tilt: number; scaleX: number; scaleY: number } {
    const dx = clientX - this.lastDragX;
    const dy = clientY - this.lastDragY;
    this.lastDragX = clientX;
    this.lastDragY = clientY;

    const tilt = Math.max(-12, Math.min(12, dx * 0.8));
    const stretch = Math.max(0.9, Math.min(1.15, 1 + Math.abs(dy) * 0.015));
    const squash = Math.max(0.88, Math.min(1.1, 1 / stretch));

    return { tilt, scaleX: squash, scaleY: stretch };
  }
}
