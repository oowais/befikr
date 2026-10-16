<script setup lang="ts">
// Sync at a glance in the top bar, only while sync is on: green = working,
// red = a problem (error or not running), grey = no paired devices yet.
// Opens the Sync page (devices, last sync, the error in full).
import { listen } from '@tauri-apps/api/event'
import { RefreshCw } from 'lucide-vue-next'
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
    <RefreshCw :size="20" :class="{ spin: st?.syncing }" />
    <span class="dot" :class="state" />
  </NuxtLink>
</template>

<style scoped>
.sync-dot { position: relative; }
.dot {
  position: absolute; top: 9px; right: 8px; width: 9px; height: 9px; border-radius: 50%;
  box-shadow: 0 0 0 2px var(--bg);
}
.ok { background: var(--green); }
.bad { background: var(--red); }
.idle { background: var(--muted); }
.spin { animation: spin 1s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
</style>
