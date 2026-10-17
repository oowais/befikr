<script setup lang="ts">
// A line-drawn landscape for the empty space under the list. Sun by
// day, moon at night. `clear`: nothing to do, so it's the whole screen with
// a short line. `garden` (carefree days on): trees for on-time completions
// ever, flowers for this week's, birds (stars at night) for the streak.
// Nothing is taken away for missed days; it just stops growing. The garden
// follows the season: blossom, summer green, autumn colours, snow.
import type { Carefree } from '~/composables/api'
import { flowersFor, seasonNow, treesFor, TREE_AT, TREE_SCALE } from '~/utils/garden'
const props = defineProps<{ clear?: boolean; garden?: Carefree | null; celebrate?: boolean }>()

// Slots on the hills (y = the hill line there), in the order they fill.
const SLOTS = [
  [112, 146, 1], [300, 155, 1.1], [30, 151, 0.9], [206, 141, 1], [334, 148, 0.9], [142, 157, 0.8],
  [262, 147, 1], [16, 160, 0.8], [318, 153, 0.8], [100, 143, 0.9], [348, 141, 0.8], [178, 151, 1],
] as const
/** Garden pieces are drawn a bit larger than the base landscape. */
const GROW = 1.25
const FLOWERS = [96, 124, 228, 258, 290, 42, 152, 214, 322, 110, 272, 14, 338, 244]
const FRONT: Record<number, number> = {
  14: 192.4, 42: 187.6, 96: 184.5, 110: 184.5, 124: 184.6, 152: 184.8, 214: 180.2,
  228: 178.5, 244: 176.8, 258: 175.7, 272: 175.1, 290: 175.2, 322: 178.1, 338: 180.8,
}
const BIRDS = [[150, 48], [186, 62], [118, 30], [216, 40]] as const
const BIRD_AT = [3, 7, 14, 30]

const season = seasonNow()
const TREES = treesFor(season)
const FLOWER_KINDS = flowersFor(season)
// Season in the sky: snowflakes, falling leaves, butterflies.
const FLAKES = [[40, 30], [96, 58], [158, 22], [214, 88], [334, 108], [18, 92], [240, 18]] as const
const LEAVES = [[70, 118, 20], [152, 108, -30], [246, 104, 60], [322, 116, -10]] as const
const BUTTERFLIES = [[132, 112], [236, 98]] as const

const trees = computed(() => {
  const n = props.garden?.onTimeTotal ?? 0
  return SLOTS.filter((_, i) => n >= TREE_AT[i]!).map(([x, y, k], i) => ({ x, y, k: k * TREE_SCALE[i]! * GROW, parts: TREES[i]! }))
})
const flowers = computed(() =>
  FLOWERS.slice(0, Math.min(props.garden?.onTimeWeek ?? 0, FLOWERS.length)).map((x, i) => ({
    x,
    y: FRONT[x]! + 1,
    parts: FLOWER_KINDS[i % FLOWER_KINDS.length]!,
  })),
)
const birds = computed(() => BIRDS.filter((_, i) => (props.garden?.streak ?? 0) >= BIRD_AT[i]!))
const hour = new Date().getHours()
const night = hour >= 20 || hour < 6
const line = computed(() =>
  !props.clear ? '' : night ? 'All clear. Sleep well.' : hour < 12 ? 'All clear. Good morning.' : 'All clear. Enjoy the day.',
)
</script>

<template>
  <div class="doodle" :class="{ clear }" aria-hidden="true">
    <!-- Under the list: full width; when short the sky is cut, the hills stay. -->
    <svg viewBox="0 0 360 220" :preserveAspectRatio="clear ? 'xMidYMax meet' : 'xMidYMax slice'">
      <!-- winter: snow on the ground, under everything, fading out at the bottom -->
      <defs v-if="garden && season === 'winter'">
        <linearGradient id="doodle-snow" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0.45" class="snow-stop" />
          <stop offset="1" class="snow-stop" stop-opacity="0" />
        </linearGradient>
      </defs>
      <path
        v-if="garden && season === 'winter'"
        class="snowfield"
        fill="url(#doodle-snow)"
        d="M0 170 C50 128 92 134 130 152 S210 120 250 140 S320 158 360 132 V220 H0 Z"
      />
      <g class="base" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
        <!-- sky (rises when the list was just cleared) -->
        <g :class="{ rise: celebrate }">
        <template v-if="night">
          <path d="M278 52 a22 22 0 1 0 22 30 a17 17 0 1 1 -22 -30 z" />
          <path d="M70 40 l0 6 M67 43 l6 0 M130 70 l0 4 M128 72 l4 0 M210 30 l0 5 M207.5 32.5 l5 0" />
        </template>
        <template v-else>
          <circle cx="282" cy="62" r="20" />
          <path d="M282 30 v-8 M282 102 v-8 M250 62 h-8 M322 62 h-8 M259 39 l-6 -6 M305 85 l6 6 M305 39 l6 -6 M259 85 l-6 6" />
          <path d="M60 58 q6 -6 12 0 q6 -6 12 0 M110 82 q5 -5 10 0 q5 -5 10 0" />
        </template>
        </g>
        <!-- hills -->
        <path d="M0 170 C50 128 92 134 130 152 S210 120 250 140 S320 158 360 132" />
        <path d="M0 196 C70 176 140 190 200 182 S300 170 360 186" />
        <!-- trees -->
        <path d="M58 166 v-8 M48 158 l10 -22 l10 22 z" />
        <path d="M84 161 v-6 M77 155 l7 -16 l7 16 z" />
        <path d="M232 144 v-7 M223 137 l9 -20 l9 20 z" />
        <!-- path -->
        <path d="M168 220 C176 204 186 196 196 186" stroke-dasharray="2 5" />
      </g>
      <!-- garden: grows with on-time completions and the streak (see utils/garden.ts) -->
      <g v-if="garden" class="garden" fill="none" stroke="currentColor" stroke-width="1.1" stroke-linecap="round" stroke-linejoin="round">
        <g v-for="(t, i) in trees" :key="`t${t.x}`" :transform="`translate(${t.x} ${t.y}) scale(${t.k})`">
          <g class="sprout" :style="{ animationDelay: `${0.15 + i * 0.04}s` }"><path v-for="(p, i) in t.parts" :key="i" :d="p.d" :class="p.f && `f-${p.f}`" /></g>
        </g>
        <g v-for="(f, i) in flowers" :key="`f${f.x}`" :transform="`translate(${f.x} ${f.y}) scale(${GROW})`">
          <g class="sprout" :style="{ animationDelay: `${0.3 + i * 0.03}s` }"><path v-for="(p, i) in f.parts" :key="i" :d="p.d" :class="p.f && `f-${p.f}`" /></g>
        </g>
        <template v-if="season === 'winter'">
          <path v-for="([x, y], i) in FLAKES" :key="`s${i}`" class="flake" :d="`M${x} ${y - 3} v6 M${x - 2.6} ${y - 1.5} l5.2 3 M${x - 2.6} ${y + 1.5} l5.2 -3`" />
        </template>
        <template v-else-if="season === 'autumn'">
          <path v-for="([x, y, r], i) in LEAVES" :key="`l${i}`" :class="i % 2 ? 'f-rust' : 'f-gold'" :transform="`translate(${x} ${y}) rotate(${r})`" d="M-3 0 Q0 -3 3 0 Q0 3 -3 0 Z M-3 0 h-1.5" />
        </template>
        <template v-else-if="season === 'spring' && !night">
          <path v-for="([x, y], i) in BUTTERFLIES" :key="`u${i}`" class="f-pink" :d="`M${x} ${y} q-4 -5 -5 -1 q1 3 5 1 q4 -5 5 -1 q-1 3 -5 1 z`" />
        </template>
        <template v-for="([x, y], i) in birds" :key="`b${i}`">
          <path v-if="night" class="sprout" :d="`M${x} ${y - 2} l0 4 M${x - 2} ${y} l4 0`" />
          <path v-else class="sprout" :d="`M${x - 6} ${y} q3 -4 6 0 q3 -4 6 0`" />
        </template>
      </g>
    </svg>
    <p v-if="clear" class="line" :class="{ rise: celebrate }">{{ line }}</p>
  </div>
</template>

<style scoped>
.doodle { position: relative; display: flex; flex-direction: column; color: var(--muted); overflow: hidden; }
.base { opacity: 0.55; }
.clear .base { opacity: 0.85; }
.garden { opacity: 0.95; }
.doodle svg { width: 100%; flex: 1; min-height: 0; max-height: 240px; }
.clear { justify-content: center; }
.clear svg { flex: none; height: auto; max-height: 320px; }
/* Garden fills: fixed natural colours, toned slightly toward the background. */
.snowfield { stroke: none; }
.snow-stop { stop-color: color-mix(in oklch, white 30%, var(--bg)); }
.flake { opacity: 0.6; }
.f-spring { fill: color-mix(in oklch, oklch(0.82 0.15 128) 88%, var(--bg)); }
.f-gold { fill: color-mix(in oklch, oklch(0.83 0.15 88) 88%, var(--bg)); }
.f-rust { fill: color-mix(in oklch, oklch(0.66 0.16 42) 88%, var(--bg)); }
.f-blossom { fill: oklch(0.99 0.01 350); stroke: none; }
.f-snow { fill: oklch(0.99 0 0); }
.f-pine { fill: color-mix(in oklch, oklch(0.58 0.09 155) 88%, var(--bg)); }
.f-leaf { fill: color-mix(in oklch, oklch(0.74 0.13 135) 88%, var(--bg)); }
.f-light { fill: color-mix(in oklch, oklch(0.84 0.12 118) 88%, var(--bg)); }
.f-bloom { fill: color-mix(in oklch, oklch(0.86 0.07 350) 88%, var(--bg)); }
.f-bloom-dot { fill: color-mix(in oklch, oklch(0.68 0.16 355) 88%, var(--bg)); stroke: none; }
.f-fruit { fill: color-mix(in oklch, oklch(0.62 0.2 28) 88%, var(--bg)); stroke: none; }
.f-autumn { fill: color-mix(in oklch, oklch(0.76 0.14 62) 88%, var(--bg)); }
.f-bark { fill: color-mix(in oklch, oklch(0.97 0.01 90) 88%, var(--bg)); }
.f-red { fill: color-mix(in oklch, oklch(0.64 0.2 25) 88%, var(--bg)); }
.f-yellow { fill: color-mix(in oklch, oklch(0.86 0.15 85) 88%, var(--bg)); }
.f-pink { fill: color-mix(in oklch, oklch(0.76 0.13 350) 88%, var(--bg)); }
.f-purple { fill: color-mix(in oklch, oklch(0.63 0.15 300) 88%, var(--bg)); }
.f-white { fill: color-mix(in oklch, oklch(0.98 0 0) 88%, var(--bg)); }
.f-core { fill: color-mix(in oklch, oklch(0.8 0.16 75) 88%, var(--bg)); stroke: none; }
.sprout { transform-box: fill-box; transform-origin: 50% 100%; animation: sprout 0.6s cubic-bezier(0.22, 1, 0.36, 1) both; }
@keyframes sprout { from { opacity: 0; scale: 1 0.3; } }
.rise { animation: rise 1.6s cubic-bezier(0.22, 1, 0.36, 1) both; }
p.rise { animation-duration: 0.8s; animation-delay: 0.7s; }
@keyframes rise { from { opacity: 0; transform: translateY(46px); } }
.line { text-align: center; margin: 14px 0 0; font-size: 1.45rem; font-weight: 500; letter-spacing: -0.01em; color: var(--text); opacity: 0.75; }
</style>
