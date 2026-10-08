let context: AudioContext | undefined;

export type PetSoundKind =
  | 'hello'
  | 'wave'
  | 'celebrate'
  | 'chirp'
  | 'crunch'
  | 'purr'
  | 'sip'
  | 'bubble'
  | 'sparkle';

export function playPetSound(kind: PetSoundKind): void {
  try {
    context ??= new AudioContext();
    if (context.state === 'suspended') void context.resume().catch(() => {});
    const start = context.currentTime;

    if (kind === 'chirp') {
      const osc = context.createOscillator();
      const gain = context.createGain();
      osc.type = 'sine';
      osc.frequency.setValueAtTime(560, start);
      osc.frequency.exponentialRampToValueAtTime(780, start + 0.08);
      osc.frequency.exponentialRampToValueAtTime(620, start + 0.18);
      gain.gain.setValueAtTime(0, start);
      gain.gain.linearRampToValueAtTime(0.04, start + 0.04);
      gain.gain.exponentialRampToValueAtTime(0.0001, start + 0.22);
      osc.connect(gain);
      gain.connect(context.destination);
      osc.start(start);
      osc.stop(start + 0.23);
      return;
    }

    if (kind === 'crunch') {
      for (let i = 0; i < 2; i++) {
        const osc = context.createOscillator();
        const gain = context.createGain();
        const t = start + i * 0.09;
        osc.type = 'triangle';
        osc.frequency.setValueAtTime(320 - i * 50, t);
        osc.frequency.exponentialRampToValueAtTime(120, t + 0.05);
        gain.gain.setValueAtTime(0.03, t);
        gain.gain.exponentialRampToValueAtTime(0.0001, t + 0.06);
        osc.connect(gain);
        gain.connect(context.destination);
        osc.start(t);
        osc.stop(t + 0.07);
      }
      return;
    }

    if (kind === 'purr') {
      const osc = context.createOscillator();
      const lfo = context.createOscillator();
      const lfoGain = context.createGain();
      const gain = context.createGain();
      osc.type = 'triangle';
      osc.frequency.value = 65;
      lfo.type = 'sine';
      lfo.frequency.value = 14;
      lfoGain.gain.value = 0.02;
      gain.gain.setValueAtTime(0.01, start);
      gain.gain.linearRampToValueAtTime(0.035, start + 0.2);
      gain.gain.exponentialRampToValueAtTime(0.0001, start + 0.6);
      lfo.connect(lfoGain);
      osc.connect(gain);
      gain.connect(context.destination);
      osc.start(start);
      osc.stop(start + 0.65);
      return;
    }

    if (kind === 'sip') {
      const osc = context.createOscillator();
      const gain = context.createGain();
      osc.type = 'sine';
      osc.frequency.setValueAtTime(420, start);
      osc.frequency.exponentialRampToValueAtTime(260, start + 0.12);
      gain.gain.setValueAtTime(0, start);
      gain.gain.linearRampToValueAtTime(0.025, start + 0.03);
      gain.gain.exponentialRampToValueAtTime(0.0001, start + 0.14);
      osc.connect(gain);
      gain.connect(context.destination);
      osc.start(start);
      osc.stop(start + 0.15);
      return;
    }

    if (kind === 'bubble') {
      const osc = context.createOscillator();
      const gain = context.createGain();
      osc.type = 'sine';
      osc.frequency.setValueAtTime(280, start);
      osc.frequency.exponentialRampToValueAtTime(680, start + 0.08);
      gain.gain.setValueAtTime(0.04, start);
      gain.gain.exponentialRampToValueAtTime(0.0001, start + 0.09);
      osc.connect(gain);
      gain.connect(context.destination);
      osc.start(start);
      osc.stop(start + 0.1);
      return;
    }

    if (kind === 'sparkle') {
      const notes = [587, 740, 880, 1175];
      for (let i = 0; i < notes.length; i++) {
        const osc = context.createOscillator();
        const gain = context.createGain();
        const t = start + i * 0.045;
        osc.type = 'sine';
        osc.frequency.value = notes[i];
        gain.gain.setValueAtTime(0, t);
        gain.gain.linearRampToValueAtTime(0.025, t + 0.015);
        gain.gain.exponentialRampToValueAtTime(0.0001, t + 0.12);
        osc.connect(gain);
        gain.connect(context.destination);
        osc.start(t);
        osc.stop(t + 0.13);
      }
      return;
    }

    const frequencies = kind === 'celebrate' ? [440, 554, 659] : kind === 'wave' ? [392, 523] : [330, 440];
    for (let i = 0; i < frequencies.length; i++) {
      const oscillator = context.createOscillator();
      const gain = context.createGain();
      const time = start + i * .11;
      oscillator.type = 'sine'; oscillator.frequency.value = frequencies[i];
      gain.gain.setValueAtTime(0, time);
      gain.gain.linearRampToValueAtTime(.025, time+.02);
      gain.gain.exponentialRampToValueAtTime(.0001, time+.15);
      oscillator.connect(gain); gain.connect(context.destination);
      oscillator.start(time); oscillator.stop(time+.16);
    }
  } catch { /* Sound remains optional when audio is unavailable. */ }
}
