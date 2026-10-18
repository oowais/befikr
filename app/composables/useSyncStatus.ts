// Sync status for the home screen (dot + stale hint), loaded once and kept
// fresh by the `sync-changed` event. `now` ticks each minute so "stale"
// shows up while the app sits open.
import { listen } from '@tauri-apps/api/event'
import type { SyncStatus } from '~/composables/api'

/** No device synced for this long: the list here may be missing their edits. */
export const STALE_MS = 6 * 3_600_000

const st = ref<SyncStatus | null>(null)
const now = ref(Date.now())
let started: Promise<void> | undefined

const load = async () => {
  st.value = await api.syncStatus().catch(() => null)
  now.value = Date.now()
}

function start() {
  started ??= (async () => {
    await load()
    listen('sync-changed', load).catch(() => undefined)
    setInterval(() => (now.value = Date.now()), 60_000)
    document.addEventListener('visibilitychange', () => document.visibilityState === 'visible' && load())
  })()
  return started
}

export function useSyncStatus() {
  const ready = start()
  /** null = sync off; 'bad' = error or not running; 'idle' = no devices yet. */
  const state = computed(() => {
    const s = st.value
    if (!s || s.mode === 'off') return null
    if (!s.running || s.lastError) return 'bad'
    if (!s.peers.length) return 'idle'
    return now.value - (lastSync.value ?? 0) > STALE_MS ? 'stale' : 'ok'
  })
  /** The most recent sync with any device. */
  const lastSync = computed(() => {
    const times = (st.value?.peers ?? []).map((p) => p.lastSyncAt ?? 0)
    return times.length ? Math.max(...times) || null : null
  })
  return { st, state, lastSync, now, ready }
}
