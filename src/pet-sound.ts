let context: AudioContext | undefined;
export function playPetSound(kind: 'hello' | 'wave' | 'celebrate'): void {
  try {
    context ??= new AudioContext();
    if (context.state === 'suspended') void context.resume().catch(() => {});
    const start = context.currentTime;
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
