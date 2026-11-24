<script setup lang="ts">
// Sync at a glance in the top bar, only while sync is on: a dot, green =
// working (pulses while a sync takes a while), orange = no device synced for a few hours
// (SyncStaleHint says which to open), red = a problem (error or not
// running), grey = no paired devices yet. Tapping it opens the Sync page
// (devices, last sync, the error in full).
const { st, state, lastSync, now, ready } = useSyncStatus()

// In the header from the first frame, not popping in after the splash.
const releaseSplash = holdSplash()

// Pulse only for a sync that takes a while while someone's looking: quick
// rounds would just flicker, and an endless animation in a hidden or
// background window (in the tray) keeps the renderer drawing every frame.
const slow = ref(false)
let slowTimer: ReturnType<typeof setTimeout> | undefined
watch(
  () => !!st.value?.syncing,
  (on) => {
    clearTimeout(slowTimer)
    slow.value = false
    if (on) slowTimer = setTimeout(() => (slow.value = true), 1000)
  },
  { immediate: true },
)
const looking = ref(false)
const look = () => (looking.value = document.visibilityState === 'visible' && document.hasFocus())
onMounted(() => {
  ready.then(releaseSplash)
  look()
  window.addEventListener('focus', look)
  window.addEventListener('blur', look)
  document.addEventListener('visibilitychange', look)
})
onBeforeUnmount(() => {
  clearTimeout(slowTimer)
  window.removeEventListener('focus', look)
  window.removeEventListener('blur', look)
  document.removeEventListener('visibilitychange', look)
})

const label = computed(() => {
  const s = st.value
  if (!s || !state.value) return ''
  if (state.value === 'bad') return `Sync problem: ${s.lastError ?? 'not running'}`
  if (state.value === 'stale') return lastSync.value ? `Last synced ${span(now.value - lastSync.value)} ago` : 'Not synced yet'
  const n = s.peers.length
  return n ? `Sync on, ${n} ${n === 1 ? 'device' : 'devices'}` : 'Sync on, no devices yet'
})
</script>

<template>
  <NuxtLink v-if="state" to="/sync" class="icon-btn sync-dot" :aria-label="label" :title="label">
    <span class="dot" :class="[state, { pulse: st?.syncing && slow && looking }]" />
  </NuxtLink>
</template>

<style scoped>
/* Narrow hit area (still full height); the dot sits at the title's top
   right, like an exponent. */
.sync-dot { width: 24px; margin-left: -9px; }
/* The shared pressed glass would be a stretched ellipse on this narrow
   target: the dot itself answers the press. */
.sync-dot:active:not(:disabled) { background: none; box-shadow: none; }
.sync-dot:active .dot { opacity: 0.5; scale: 0.8; }
.dot { transition: opacity 0.12s, scale 0.12s; }
.dot { width: 8px; height: 8px; border-radius: 50%; margin-top: -16px; }
.ok { background: var(--green); }
.stale { background: var(--orange); }
.bad { background: var(--red); }
.idle { background: var(--muted); }
.pulse { animation: pulse 1s ease-in-out infinite; }
@keyframes pulse { 50% { opacity: 0.35; } }
</style>
