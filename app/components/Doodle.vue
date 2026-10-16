<script setup lang="ts">
// A line-drawn landscape for the empty space under the list. Sun by
// day, moon at night. `clear`: nothing to do, so it's the whole screen with
// a short line.
const props = defineProps<{ clear?: boolean }>()
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
      <g fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
        <!-- sky -->
        <template v-if="night">
          <path d="M278 52 a22 22 0 1 0 22 30 a17 17 0 1 1 -22 -30 z" />
          <path d="M70 40 l0 6 M67 43 l6 0 M130 70 l0 4 M128 72 l4 0 M210 30 l0 5 M207.5 32.5 l5 0" />
        </template>
        <template v-else>
          <circle cx="282" cy="62" r="20" />
          <path d="M282 30 v-8 M282 102 v-8 M250 62 h-8 M322 62 h-8 M259 39 l-6 -6 M305 85 l6 6 M305 39 l6 -6 M259 85 l-6 6" />
          <path d="M60 58 q6 -6 12 0 q6 -6 12 0 M110 82 q5 -5 10 0 q5 -5 10 0" />
        </template>
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
    </svg>
    <p v-if="clear" class="line">{{ line }}</p>
  </div>
</template>

<style scoped>
.doodle { position: relative; display: flex; flex-direction: column; color: var(--muted); opacity: 0.55; overflow: hidden; }
.doodle svg { width: 100%; flex: 1; min-height: 0; max-height: 240px; }
.clear { opacity: 0.85; justify-content: center; }
.clear svg { flex: none; height: auto; max-height: 320px; }
.line { text-align: center; margin: 14px 0 0; font-size: 1.45rem; font-weight: 500; letter-spacing: -0.01em; color: var(--text); opacity: 0.75; }
</style>
