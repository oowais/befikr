// Drawings for the landscape that grows with carefree days (Doodle.vue).
// Each piece is drawn at its base (0, 0), growing up (negative y), in the
// doodle's line style; `f` names a soft fill (see Doodle.vue styles).
// Leafy trees follow the season: fresh green in spring, full in summer,
// coloured in autumn, bare (or snowed on) in winter.

export type Part = { d: string; f?: Fill }
export type Fill =
  | 'pine' | 'leaf' | 'spring' | 'light' | 'gold' | 'rust' | 'autumn' | 'bloom' | 'bloom-dot' | 'blossom'
  | 'fruit' | 'bark' | 'snow' | 'red' | 'yellow' | 'pink' | 'purple' | 'white' | 'core'
export type Season = 'spring' | 'summer' | 'autumn' | 'winter'

// Time zones south of the equator (seasons flipped). Tropical ones don't matter much.
const SOUTH = /^(Australia|Antarctica)\/|^Pacific\/(Auckland|Chatham|Fiji)|^America\/(Argentina|Santiago|Montevideo|Sao_Paulo|Asuncion|Punta_Arenas)|^Africa\/(Johannesburg|Maputo|Harare|Windhoek|Gaborone|Maseru|Mbabane)|^Indian\/(Mauritius|Reunion)/

/** Meteorological season (Dec–Feb winter in the north) for this device's zone. */
export function seasonNow(d = new Date(), tz = Intl.DateTimeFormat().resolvedOptions().timeZone): Season {
  const order: Season[] = ['winter', 'spring', 'summer', 'autumn']
  const i = Math.floor(((d.getMonth() + 1) % 12) / 3)
  return order[SOUTH.test(tz) ? (i + 2) % 4 : i]!
}

const circle = (cx: number, cy: number, r: number) => `M${cx - r} ${cy} a${r} ${r} 0 1 0 ${2 * r} 0 a${r} ${r} 0 1 0 ${-2 * r} 0`
const dots = (pts: number[][], r: number, f: Fill): Part[] => pts.map(([x, y]) => ({ d: circle(x!, y!, r), f }))
/** A snow cap on a peak at (x, y), `w` wide, `h` tall, with a wavy edge. */
const cap = (x: number, y: number, w: number, h: number) =>
  `M${x} ${y} L${x + w / 2} ${y + h} Q${x + w / 4} ${y + h + 1.4} ${x} ${y + h} Q${x - w / 4} ${y + h + 1.4} ${x - w / 2} ${y + h} Z`

/**
 * A tree: `base` always (trunk, evergreen body), then a leafy `crown` that
 * changes colour with the season (`fills` overrides the default per season)
 * or gives way to `bare` branches in winter; `extra` adds per-season bits
 * (fruit, blossom, snow).
 */
type Tree = {
  base: Part[]
  crown?: string
  fills?: Partial<Record<Season, Fill>>
  bare?: string
  extra?: Partial<Record<Season, Part[]>>
}
const LEAF: Record<Season, Fill> = { spring: 'spring', summer: 'leaf', autumn: 'autumn', winter: 'snow' }

const cloud = 'M-12 -16 C-16 -18 -14 -26 -9 -25 C-9 -32 0 -34 2 -29 C6 -33 14 -30 12 -24 C17 -22 15 -15 10 -16 C8 -12 -9 -12 -12 -16 Z'
const cloudBare = 'M0 -8 L-7 -17 M-4 -12.5 L-10 -14 M0 -11 L7 -20 M4 -15.5 L10 -16 M0 -13 L-2 -25 M0 -13 L3 -27 M-1.3 -20 L-5 -24'
const roundBare = 'M0 -8 L-6 -19 M-3 -13.5 L-9 -15 M0 -10 L5 -22 M2.5 -16 L8 -17 M0 -10 V-27 M0 -21 L-3 -25'

const pine: Tree = { base: [{ d: 'M0 0 v-4' }, { d: 'M0 -24 L7 -4 Q0 -2.5 -7 -4 Z', f: 'pine' }], extra: { winter: [{ d: cap(0, -24, 5.4, 7.5), f: 'snow' }] } }
const fir: Tree = {
  base: [
    { d: 'M0 0 v-3' },
    { d: 'M-10 -3 Q0 -1 10 -3 L0 -14 Z', f: 'pine' },
    { d: 'M-8 -10 Q0 -8 8 -10 L0 -21 Z', f: 'pine' },
    { d: 'M-6 -17 Q0 -15 6 -17 L0 -28 Z', f: 'pine' },
  ],
  extra: { winter: [{ d: cap(0, -28, 4.6, 5.5), f: 'snow' }, { d: 'M-8 -10 Q0 -8 8 -10 Q4 -8.4 0 -9 Q-4 -8.4 -8 -10 Z', f: 'snow' }, { d: 'M-10 -3 Q0 -1 10 -3 Q5 -1.4 0 -2 Q-5 -1.4 -10 -3 Z', f: 'snow' }] },
}
const oak: Tree = { base: [{ d: 'M0 0 V-13 M0 -8 L-5 -14 M0 -10 L5 -16' }], crown: cloud, bare: cloudBare, fills: { autumn: 'rust' } }

/** Trees in the order they grow (later ones are rarer and grander). */
const TREE_DEFS: Tree[] = [
  pine,
  // bush (snowed over in winter)
  { base: [], crown: 'M-10 0 C-13 -1 -12 -7 -7 -7 C-7 -12 1 -13 3 -8 C6 -11 12 -8 10 -3 C13 -2 12 0 10 0 Z', fills: { autumn: 'gold' } },
  // birch
  {
    base: [],
    crown: 'M0 -24 m-6 0 a6 9 0 1 0 12 0 a6 9 0 1 0 -12 0',
    fills: { summer: 'light', autumn: 'gold' },
    bare: 'M0 -16 L-4 -27 M0 -16 L4 -29 M0 -16 V-32 M-2 -21 L-6 -24 M2 -22 L6 -25',
    extra: Object.fromEntries((['spring', 'summer', 'autumn', 'winter'] as const).map((s) => [s, [{ d: 'M-1.4 0 V-17 H1.4 V0 Z', f: 'bark' }, { d: 'M-1.4 -5 h1.6 M1.4 -10 h-1.4 M-1.4 -14 h1.2' }]])),
  },
  fir,
  oak,
  // cypress
  {
    base: [{ d: 'M0 0 v-3' }, { d: 'M0 -32 C5 -24 6 -10 3 -3 H-3 C-6 -10 -5 -24 0 -32 Z', f: 'pine' }, { d: 'M0 -27 V-8 M0 -14 l-2 -3 M0 -20 l2 -3' }],
    extra: { winter: [{ d: 'M0 -32 C2 -29.5 3 -27.5 3.6 -25 Q1.8 -23.6 0 -25 Q-1.8 -23.6 -3.6 -25 C-3 -27.5 -2 -29.5 0 -32 Z', f: 'snow' }] },
  },
  // apple: blossom in spring, fruit in summer and autumn
  {
    base: [{ d: 'M0 0 V-10' }],
    crown: circle(0, -18, 10),
    fills: { autumn: 'gold' },
    bare: roundBare,
    extra: {
      spring: dots([[-4, -20], [3, -14], [5, -22], [-3, -13], [0, -25], [-6, -16], [6, -17]], 1.4, 'blossom'),
      summer: dots([[-4, -20], [3, -14], [5, -22], [-3, -13], [0, -25]], 1.7, 'fruit'),
      autumn: dots([[-4, -20], [3, -14], [5, -22]], 1.7, 'fruit'),
    },
  },
  fir,
  // cherry: pink in spring
  {
    base: [{ d: 'M0 0 V-13 M0 -8 L-5 -14 M0 -10 L5 -16' }],
    crown: cloud,
    fills: { spring: 'bloom', autumn: 'rust' },
    bare: cloudBare,
    extra: { spring: dots([[-8, -20], [-2, -27], [5, -23], [9, -18], [-4, -16], [2, -18]], 1.2, 'bloom-dot') },
  },
  // maple: orange in autumn
  {
    base: [{ d: 'M0 0 V-10' }],
    crown: 'M0 -31 C8 -29 13 -21 10 -14 C8 -9 -8 -9 -10 -14 C-13 -21 -8 -29 0 -31 Z',
    bare: roundBare,
    extra: Object.fromEntries((['spring', 'summer', 'autumn'] as const).map((s) => [s, [{ d: 'M0 -10 V-26 M0 -15 L-5 -20 M0 -19 L5 -24' }]])),
  },
  // two pines
  {
    base: [
      { d: 'M5 0 v-4' },
      { d: 'M5 -27 L13 -4 Q5 -2.5 -3 -4 Z', f: 'pine' },
      { d: 'M-5 0 v-3' },
      { d: 'M-5 -19 L1 -3 Q-5 -1.8 -11 -3 Z', f: 'pine' },
    ],
    extra: { winter: [{ d: cap(5, -27, 5.6, 8), f: 'snow' }, { d: cap(-5, -19, 4.4, 6), f: 'snow' }] },
  },
  // grand oak
  oak,
]

function treeParts(t: Tree, season: Season): Part[] {
  const parts = [...t.base]
  if (t.crown) {
    if (season === 'winter' && t.bare) parts.push({ d: t.bare })
    else parts.push({ d: t.crown, f: t.fills?.[season] ?? LEAF[season] })
  }
  return [...parts, ...(t.extra?.[season] ?? [])]
}

/** The trees as drawn in `season`, in growth order. */
export const treesFor = (season: Season): Part[][] => TREE_DEFS.map((t) => treeParts(t, season))

/** Sizes per slot (the last trees are the biggest). */
export const TREE_SCALE = [1, 1, 1, 1, 1, 1, 1, 1.25, 1.05, 1.05, 1, 1.35]

// Flowers, one per on-time completion this week.
const stem = { d: 'M0 0 v-7 M0 -2 q3 -1 4 -4' }
const tulip = (f: Fill): Part[] => [stem, { d: 'M-2.6 -7 V-11.5 L-1 -9.8 L0 -12 L1 -9.8 L2.6 -11.5 V-7 Q0 -5.4 -2.6 -7 Z', f }]
const daisy: Part[] = [stem, { d: circle(0, -10, 3.4), f: 'white' }, { d: circle(0, -10, 1.3), f: 'core' }]
const bell: Part[] = [{ d: 'M0 0 Q1 -6 -1 -9 M0 -3 q-3 -1 -4 -3' }, { d: 'M-4 -5 Q-4 -10 -1 -10 Q2 -10 2 -5 Q-1 -4 -4 -5 Z', f: 'purple' }]
const sun: Part[] = [stem, { d: circle(0, -10, 3.2), f: 'yellow' }, { d: circle(0, -10, 1.1), f: 'core' }]
const mushroom: Part[] = [
  { d: 'M-1.3 0 V-4 H1.3 V0 Z', f: 'white' },
  { d: 'M-4.5 -4 Q-4.5 -9 0 -9 Q4.5 -9 4.5 -4 Q0 -3 -4.5 -4 Z', f: 'red' },
  ...dots([[-2, -6.2], [1.6, -7.4]], 0.7, 'blossom'),
]
const snowdrop: Part[] = [
  { d: 'M0 0 v-8 q0 -2 2 -2 M0 0 q-2 -3 -1 -6' },
  { d: 'M0.6 -9.6 Q0.6 -6 2.3 -5.4 Q4 -6 4 -9.6 Q2.3 -10.6 0.6 -9.6 Z', f: 'white' },
]
const holly: Part[] = [
  { d: 'M0 0 v-3' },
  { d: 'M0 -3 Q-3 -6.5 -6.5 -4.5 Q-4.5 -3.4 -5.4 -1.2 Q-2.2 -2 0 -3 Z', f: 'pine' },
  { d: 'M0 -3 Q3 -6.5 6.5 -4.5 Q4.5 -3.4 5.4 -1.2 Q2.2 -2 0 -3 Z', f: 'pine' },
  ...dots([[-1.3, -4.6], [1.3, -4.9], [0, -6.6]], 1.15, 'fruit'),
]
const sprig: Part[] = [{ d: 'M0 0 v-2' }, { d: 'M0 -10 L3.4 -2 Q0 -1.2 -3.4 -2 Z', f: 'pine' }, { d: cap(0, -10, 2.4, 3.4), f: 'snow' }]

const FLOWERS: Record<Season, Part[][]> = {
  spring: [tulip('red'), daisy, tulip('pink'), bell, tulip('yellow'), daisy],
  summer: [tulip('red'), daisy, bell, tulip('yellow'), sun, tulip('pink'), daisy],
  autumn: [sun, mushroom, bell, sun, mushroom, daisy],
  winter: [sprig, holly, snowdrop, sprig, holly],
}
export const flowersFor = (season: Season) => FLOWERS[season]
