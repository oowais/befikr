// Completion sounds, synthesised with Web Audio (no audio files).
// Device-local on/off preference.
const KEY = 'befikr.sound'

const enabled = ref(read())
let ctx: AudioContext | null = null

function read(): boolean {
  try {
    return localStorage.getItem(KEY) !== 'off'
  } catch {
    return true
  }
}

/** A short glassy note: sine + a quiet octave, fast attack, soft decay. */
function note(ac: AudioContext, freq: number, at: number, gain: number, length = 0.45) {
  const out = ac.createGain()
  out.gain.setValueAtTime(0.0001, at)
  out.gain.exponentialRampToValueAtTime(gain, at + 0.008)
  out.gain.exponentialRampToValueAtTime(0.0001, at + length)
  out.connect(ac.destination)
  for (const [mult, level] of [[1, 1], [2, 0.18]] as const) {
    const osc = ac.createOscillator()
    const g = ac.createGain()
    osc.type = 'sine'
    osc.frequency.value = freq * mult
    g.gain.value = level
    osc.connect(g).connect(out)
    osc.start(at)
    osc.stop(at + length + 0.05)
  }
}

function play(f: (ac: AudioContext, t: number) => void) {
  if (!enabled.value) return
  try {
    ctx ??= new AudioContext()
    if (ctx.state === 'suspended') void ctx.resume()
    f(ctx, ctx.currentTime + 0.01)
  } catch {
    /* no audio: stay silent */
  }
}

export function useSound() {
  function setEnabled(on: boolean) {
    enabled.value = on
    try {
      localStorage.setItem(KEY, on ? 'on' : 'off')
    } catch {
      /* session only */
    }
  }
  return {
    enabled,
    setEnabled,
    /** Task done: a rising two-note chime (E6 → A6). */
    complete: () =>
      play((ac, t) => {
        note(ac, 1318.5, t, 0.16, 0.35)
        note(ac, 1760, t + 0.09, 0.14, 0.6)
      }),
    /** Last task done, list clear: a soft rising arpeggio (C6 E6 G6 C7). */
    allClear: () =>
      play((ac, t) => {
        note(ac, 1046.5, t, 0.11, 0.5)
        note(ac, 1318.5, t + 0.11, 0.1, 0.5)
        note(ac, 1568, t + 0.22, 0.09, 0.6)
        note(ac, 2093, t + 0.36, 0.08, 1)
      }),
    /** Subtask ticked: one quieter note. */
    tick: () => play((ac, t) => note(ac, 1568, t, 0.09, 0.3)),
  }
}
