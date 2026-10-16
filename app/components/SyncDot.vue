<script setup lang="ts">
// Sync at a glance in the top bar, only while sync is on: a dot, green =
// working (pulses while syncing), red = a problem (error or not running),
// grey = no paired devices yet. Tapping it opens the Sync page (devices,
// last sync, the error in full).
import { listen } from '@tauri-apps/api/event'
import type { SyncStatus } from '~/composables/api'

const st = ref<SyncStatus | null>(null)
const load = async () => (st.value = await api.syncStatus().catch(() => null))

let unlisten: (() => void) | undefined
onMounted(async () => {
  await load()
  unlisten = await listen('sync-changed', load).catch(() => undefined)
})
onUnmounted(() => unlisten?.())

const state = computed(() => {
  const s = st.value
  if (!s || s.mode === 'off') return null
  if (!s.running || s.lastError) return 'bad'
  return s.peers.length ? 'ok' : 'idle'
})
const label = computed(() => {
  const s = st.value
  if (!s || !state.value) return ''
  if (state.value === 'bad') return `Sync problem: ${s.lastError ?? 'not running'}`
  const n = s.peers.length
  return n ? `Sync on, ${n} ${n === 1 ? 'device' : 'devices'}` : 'Sync on, no devices yet'
})
</script>

<template>
  <NuxtLink v-if="state" to="/sync" class="icon-btn sync-dot" :aria-label="label" :title="label">
    <span class="dot" :class="[state, { pulse: st?.syncing }]" />
  </NuxtLink>
</template>

<style scoped>
/* Narrow hit area (still full height); the dot sits at the title's top
   right, like an exponent. */
.sync-dot { width: 24px; margin-left: -9px; }
.dot { width: 8px; height: 8px; border-radius: 50%; margin-top: -16px; }
.ok { background: var(--green); }
.bad { background: var(--red); }
.idle { background: var(--muted); }
.pulse { animation: pulse 1s ease-in-out infinite; }
@keyframes pulse { 50% { opacity: 0.35; } }
</style>
