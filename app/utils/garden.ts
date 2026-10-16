// Drawings for the landscape that grows with carefree days (Doodle.vue).
// Each piece is drawn at its base (0, 0), growing up (negative y), in the
// doodle's line style; `f` names a soft fill (see Doodle.vue styles).

export type Part = { d: string; f?: Fill }
export type Fill = 'pine' | 'leaf' | 'light' | 'bloom' | 'bloom-dot' | 'fruit' | 'autumn' | 'bark' | 'red' | 'yellow' | 'pink' | 'purple' | 'white' | 'core'

const circle = (cx: number, cy: number, r: number) => `M${cx - r} ${cy} a${r} ${r} 0 1 0 ${2 * r} 0 a${r} ${r} 0 1 0 ${-2 * r} 0`
const cloud = 'M-12 -16 C-16 -18 -14 -26 -9 -25 C-9 -32 0 -34 2 -29 C6 -33 14 -30 12 -24 C17 -22 15 -15 10 -16 C8 -12 -9 -12 -12 -16 Z'

const pine: Part[] = [{ d: 'M0 0 v-4' }, { d: 'M0 -24 L7 -4 Q0 -2.5 -7 -4 Z', f: 'pine' }]
const fir: Part[] = [
  { d: 'M0 0 v-3' },
  { d: 'M-10 -3 Q0 -1 10 -3 L0 -14 Z', f: 'pine' },
  { d: 'M-8 -10 Q0 -8 8 -10 L0 -21 Z', f: 'pine' },
  { d: 'M-6 -17 Q0 -15 6 -17 L0 -28 Z', f: 'pine' },
]
const oak: Part[] = [{ d: 'M0 0 V-13 M0 -8 L-5 -14 M0 -10 L5 -16' }, { d: cloud, f: 'leaf' }]

/** Trees in the order they grow (later ones are rarer and grander). */
export const TREES: Part[][] = [
  pine,
  // bush
  [{ d: 'M-10 0 C-13 -1 -12 -7 -7 -7 C-7 -12 1 -13 3 -8 C6 -11 12 -8 10 -3 C13 -2 12 0 10 0 Z', f: 'leaf' }],
  // birch
  [
    { d: 'M0 -24 m-6 0 a6 9 0 1 0 12 0 a6 9 0 1 0 -12 0', f: 'light' },
    { d: 'M-1.4 0 V-17 H1.4 V0 Z', f: 'bark' },
    { d: 'M-1.4 -5 h1.6 M1.4 -10 h-1.4 M-1.4 -14 h1.2' },
  ],
  fir,
  oak,
  // cypress
  [{ d: 'M0 0 v-3' }, { d: 'M0 -32 C5 -24 6 -10 3 -3 H-3 C-6 -10 -5 -24 0 -32 Z', f: 'pine' }, { d: 'M0 -27 V-8 M0 -14 l-2 -3 M0 -20 l2 -3' }],
  // apple
  [
    { d: 'M0 0 V-10' },
    { d: circle(0, -18, 10), f: 'leaf' },
    ...[[-4, -20], [3, -14], [5, -22], [-3, -13], [0, -25]].map(([x, y]) => ({ d: circle(x!, y!, 1.7), f: 'fruit' as const })),
  ],
  fir,
  // cherry blossom
  [
    { d: 'M0 0 V-13 M0 -8 L-5 -14 M0 -10 L5 -16' },
    { d: cloud, f: 'bloom' },
    ...[[-8, -20], [-2, -27], [5, -23], [9, -18], [-4, -16], [2, -18]].map(([x, y]) => ({ d: circle(x!, y!, 1.2), f: 'bloom-dot' as const })),
  ],
  // maple in autumn
  [
    { d: 'M0 0 V-10' },
    { d: 'M0 -31 C8 -29 13 -21 10 -14 C8 -9 -8 -9 -10 -14 C-13 -21 -8 -29 0 -31 Z', f: 'autumn' },
    { d: 'M0 -10 V-26 M0 -15 L-5 -20 M0 -19 L5 -24' },
  ],
  // two pines
  [
    { d: 'M5 0 v-4' },
    { d: 'M5 -27 L13 -4 Q5 -2.5 -3 -4 Z', f: 'pine' },
    { d: 'M-5 0 v-3' },
    { d: 'M-5 -19 L1 -3 Q-5 -1.8 -11 -3 Z', f: 'pine' },
  ],
  // grand oak
  oak,
]

/** Sizes per slot (the last trees are the biggest). */
export const TREE_SCALE = [1, 1, 1, 1, 1, 1, 1, 1.25, 1.05, 1.05, 1, 1.35]

/** Flowers, one per on-time completion this week, in order. */
const stem = { d: 'M0 0 v-7 M0 -2 q3 -1 4 -4' }
const tulip = (f: Fill): Part[] => [stem, { d: 'M-2.6 -7 V-11.5 L-1 -9.8 L0 -12 L1 -9.8 L2.6 -11.5 V-7 Q0 -5.4 -2.6 -7 Z', f }]
const daisy: Part[] = [stem, { d: circle(0, -10, 3.4), f: 'white' }, { d: circle(0, -10, 1.3), f: 'core' }]
const bell: Part[] = [{ d: 'M0 0 Q1 -6 -1 -9 M0 -3 q-3 -1 -4 -3' }, { d: 'M-4 -5 Q-4 -10 -1 -10 Q2 -10 2 -5 Q-1 -4 -4 -5 Z', f: 'purple' }]
const sun: Part[] = [stem, { d: circle(0, -10, 3.2), f: 'yellow' }, { d: circle(0, -10, 1.1), f: 'core' }]
export const FLOWER_KINDS: Part[][] = [tulip('red'), daisy, bell, tulip('yellow'), sun, tulip('pink'), daisy]
