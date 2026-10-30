/**
 * One short synthesized tone per unlock or level-up (DESIGN.md, Sound). No
 * audio file. Callers check `Settings.sound` before calling.
 */
export type Tone = "unlock" | "levelUp";

/** Note frequencies in Hz, played one after another. */
const NOTES: Record<Tone, readonly number[]> = {
  unlock: [784, 1175],
  levelUp: [523, 659, 784, 1047],
};

const NOTE_S = 0.09;
const PEAK_GAIN = 0.12;

let context: AudioContext | null = null;

function audioContext(): AudioContext | null {
  try {
    context ??= new AudioContext();
    return context;
  } catch {
    return null;
  }
}

/** Plays the tone. Never throws; silently does nothing without audio support. */
export function playTone(tone: Tone): void {
  const ctx = audioContext();
  if (ctx === null) return;
  try {
    if (ctx.state === "suspended") void ctx.resume();
    const start = ctx.currentTime + 0.01;
    NOTES[tone].forEach((hz, i) => {
      const at = start + i * NOTE_S;
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = "sine";
      osc.frequency.value = hz;
      gain.gain.setValueAtTime(0.0001, at);
      gain.gain.exponentialRampToValueAtTime(PEAK_GAIN, at + 0.01);
      gain.gain.exponentialRampToValueAtTime(0.0001, at + NOTE_S + 0.05);
      osc.connect(gain).connect(ctx.destination);
      osc.start(at);
      osc.stop(at + NOTE_S + 0.06);
    });
  } catch {
    // Audio is a nicety; never break the popup over it.
  }
}
