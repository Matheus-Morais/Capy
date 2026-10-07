import type { Snapshot } from './bridge';

export const SLEEP_AFTER_MS = 180_000;
export const ATTENTION_REPEAT_MS = 30_000;
export type Gesture = 'hello' | 'wave' | 'click' | 'celebrate' | '';
export interface PetFrame { state: string; gesture: Gesture; gestureId: number; waiting: number; }

export class PetBehavior {
  private lastActive: number;
  private gesture: Gesture = 'hello';
  private gestureUntil: number;
  private gestureId = 1;
  private requests = new Set<string>();
  private seen = new Set<string>();
  private lastWave = -Infinity;
  private completions = new Set<string>();
  private previousDone = new Set<string>();

  constructor(now: number) {
    this.lastActive = now;
    this.gestureUntil = now + 1_800;
  }

  interact(now: number, click = false): void {
    this.lastActive = now;
    if (click) this.play('click', now, 650);
  }

  acknowledge(): void {
    this.requests.forEach(id => this.seen.add(id));
    if (this.gesture === 'wave') this.gestureUntil = 0;
  }

  private play(gesture: Gesture, now: number, duration: number): void {
    this.gesture = gesture;
    this.gestureUntil = now + duration;
    this.gestureId++;
  }

  update(data: Snapshot, now: number): PetFrame {
    const sessions = data.sessions.filter(s => !s.hidden);
    const requests = new Set<string>();
    (data.handoffs ?? []).forEach(review=>requests.add(`handoff:${review.nonce}`));
    (data.chatTransfers ?? []).forEach(review=>requests.add(`chat-transfer:${review.nonce}`));
    for (const session of sessions) {
      const exact = data.interventions.filter(r => r.sessionId === session.id && r.status === 'pending');
      exact.forEach(r => requests.add(`${r.generation}:${r.nonce}`));
      if (!exact.length && session.state === 'waiting') requests.add(`waiting:${session.id}`);
    }
    const newRequest = [...requests].some(id => !this.requests.has(id));
    this.seen = new Set([...this.seen].filter(id => requests.has(id)));
    this.requests = requests;
    const working = sessions.some(s => s.state === 'working');
    if (working || requests.size) this.lastActive = now;

    const done = new Set<string>();
    let completed = false;
    for (const session of sessions) {
      if (data.scenario !== 'real' && session.state === 'done') {
        done.add(session.id);
        if (!this.previousDone.has(session.id)) completed = true;
      }
      if (data.scenario === 'real' && session.completion) {
        const key = `${session.id}:${session.completion}`;
        if (!this.completions.has(key)) {
          this.completions.add(key);
          completed = true;
        }
      }
    }
    this.previousDone = done;
    if (this.completions.size > 1_000) this.completions.delete(this.completions.values().next().value!);

    const unseen = [...requests].some(id => !this.seen.has(id));
    if (unseen && (newRequest || now - this.lastWave >= ATTENTION_REPEAT_MS)) {
      this.lastWave = now;
      this.play('wave', now, 1_600);
    } else if (completed && !requests.size) {
      this.lastActive = now;
      this.play('celebrate', now, 2_200);
    }
    if (now >= this.gestureUntil) this.gesture = '';
    const state = requests.size ? 'waiting' : working ? 'working'
      : this.gesture === 'celebrate' ? 'done'
      : now - this.lastActive >= SLEEP_AFTER_MS ? 'sleeping' : 'idle';
    return { state, gesture: this.gesture, gestureId: this.gestureId,
      waiting: sessions.filter(s => s.state === 'waiting' || data.interventions.some(r => r.sessionId === s.id && r.status === 'pending')).length + (data.handoffs ?? []).length + (data.chatTransfers ?? []).length };
  }
}

export function hoverGaze(x: number, y: number, width: number, height: number): [number, number] {
  if (width <= 0 || height <= 0) return [0, 0];
  return [Math.max(-2.5, Math.min(2.5, (x / width - .5) * 5)),
    Math.max(-1.5, Math.min(1.5, (y / height - .5) * 3))];
}
