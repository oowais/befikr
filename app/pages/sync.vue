<script setup lang="ts">
// Sync between this device and paired ones (P2P, no server). Mode, this
// device's name, paired devices, and pairing: one device shows a code, the
// other picks it from "Nearby" and types the code.
import { listen } from '@tauri-apps/api/event'
import { ArrowLeft, Laptop, Link2, Radar, RefreshCw, X } from 'lucide-vue-next'
import type { SyncMode, SyncStatus } from '~/composables/api'

const router = useRouter()
const { show } = useToast()
const st = ref<SyncStatus | null>(null)
const error = ref('')
const name = ref('')
const busy = ref(false)
const joining = ref<{ endpoint: string; name: string } | null>(null)
const code = ref('')
const confirmRemove = ref<string | null>(null)
const now = ref(Date.now())

const modes: { id: SyncMode; label: string; hint: string }[] = [
  { id: 'off', label: 'Off', hint: 'Nothing leaves this device.' },
  { id: 'lan', label: 'Same Wi-Fi', hint: 'Syncs when both devices are on the same network. No third party involved.' },
  {
    id: 'relay',
    label: 'Anywhere',
    hint: 'Same Wi-Fi first; otherwise through n0’s free public relays. They see that two devices connect, never your data (end-to-end encrypted).',
  },
]

async function load() {
  try {
    st.value = await api.syncStatus()
    if (!name.value) name.value = st.value.name || (await defaultName())
  } catch (e) {
    error.value = String(e)
  }
}
const defaultName = async () => (await reminders.status().then((s) => s.device, () => '')) || 'Desktop'

let unlisten: (() => void) | undefined
let timer: ReturnType<typeof setInterval> | undefined
onMounted(async () => {
  await load()
  unlisten = await listen('sync-changed', load).catch(() => undefined)
  timer = setInterval(() => (now.value = Date.now()), 1000)
})
onUnmounted(() => {
  unlisten?.()
  clearInterval(timer)
  if (st.value?.pairing) api.stopPairing().catch(() => {})
})

async function run(f: () => Promise<unknown>, done?: string) {
  error.value = ''
  busy.value = true
  try {
    await f()
    if (done) show({ message: done })
  } catch (e) {
    error.value = String(e)
  } finally {
    busy.value = false
    await load()
  }
}

async function setMode(mode: SyncMode) {
  if (mode !== 'off' && !st.value?.name) await saveName()
  run(() => api.setSyncMode(mode))
}
async function saveName() {
  const n = name.value.trim()
  if (!n || n === st.value?.name) return
  await api.setDeviceName(n).catch((e) => (error.value = String(e)))
}

function pair() {
  const target = joining.value
  if (!target || code.value.trim().length !== 6) return
  run(async () => {
    await api.pairWith(target.endpoint, code.value.trim())
    joining.value = null
    code.value = ''
  }, `Paired with ${target.name}`)
}

const ago = (ms: number | null) => {
  if (ms == null) return 'Not synced yet'
  const s = Math.max(0, Math.round((now.value - ms) / 1000))
  if (s < 60) return 'Synced just now'
  return `Synced ${span(s * 1000)} ago`
}
const left = computed(() => {
  const s = Math.max(0, Math.round(((st.value?.pairing?.untilMs ?? 0) - now.value) / 1000))
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`
})
const on = computed(() => st.value && st.value.mode !== 'off')

// Fallback when the other device isn't found automatically (mDNS blocked).
const byAddress = ref(false)
const address = ref('')
function pairByAddress() {
  const a = address.value.trim()
  if (!a || code.value.trim().length !== 6) return
  run(async () => {
    await api.pairWithAddress(a, code.value.trim())
    byAddress.value = false
    address.value = code.value = ''
  }, 'Paired')
}

// Restart discovery; keep the spinner for a moment so the press registers.
const scanning = ref(false)
async function rescan() {
  scanning.value = true
  await run(api.rescanSync)
  setTimeout(() => (scanning.value = false), 1200)
}
</script>

<template>
  <main class="page">
    <header class="topbar">
      <button class="icon-btn" aria-label="Back" @click="router.back()"><ArrowLeft :size="22" /></button>
      <h1>Sync</h1>
      <button v-if="on && st?.peers.length" class="icon-btn" aria-label="Sync now" :disabled="busy" @click="run(api.syncNow, 'Synced')">
        <RefreshCw :size="20" :class="{ spin: busy }" />
      </button>
    </header>
    <p v-if="error" class="error">{{ error }}</p>
    <p v-else-if="st?.lastError" class="error">{{ st.lastError }}</p>
    <p v-if="st && st.mode !== 'off' && !st.running" class="error">Sync isn't running. Switch it off and on again.</p>

    <template v-if="st">
      <div class="seg" role="radiogroup" aria-label="Sync">
        <button
          v-for="m in modes"
          :key="m.id"
          type="button"
          role="radio"
          class="opt"
          :class="{ chosen: st.mode === m.id }"
          :aria-checked="st.mode === m.id"
          :disabled="busy"
          @click="setMode(m.id)"
        >
          {{ m.label }}
        </button>
      </div>
      <p class="muted small hint">{{ modes.find((m) => m.id === st!.mode)?.hint }}</p>

      <template v-if="on">
        <h2 class="section-title">This device</h2>
        <input
          v-model="name"
          class="field"
          placeholder="Name shown to your other devices"
          maxlength="40"
          enterkeyhint="done"
          @blur="saveName().then(load)"
          @keydown.enter="($event.target as HTMLInputElement).blur()"
        />

        <h2 class="section-title">Paired devices<span v-if="st.syncing" class="syncing"> · Syncing…</span></h2>
        <div v-if="st.peers.length" class="pane rows">
          <div v-for="p in st.peers" :key="p.endpoint" class="item">
            <Laptop :size="20" class="muted" />
            <span class="flex stack">
              <span>{{ p.name }}</span>
              <span class="muted small">{{ ago(p.lastSyncAt) }}</span>
            </span>
            <button v-if="confirmRemove === p.endpoint" class="btn danger" @click="run(() => api.unpair(p.endpoint), 'Removed'); confirmRemove = null">Remove</button>
            <button v-else class="icon-btn" :aria-label="`Remove ${p.name}`" @click="confirmRemove = p.endpoint"><X :size="18" /></button>
          </div>
        </div>
        <p v-else class="muted small">None yet. Add one below.</p>

        <h2 class="section-title">Add a device</h2>
        <div v-if="st.pairing" class="pane pairing">
          <span class="muted small">On the other device, open Sync and pick <b>{{ st.name || name }}</b>, then type:</span>
          <span class="code">{{ st.pairing.code.slice(0, 3) }} {{ st.pairing.code.slice(3) }}</span>
          <span v-if="st.addresses.length" class="muted small">Not listed there? Use “Enter an address” with <b>{{ st.addresses.join(' or ') }}</b></span>
          <span class="muted small">Closes in {{ left }}</span>
          <button class="btn" @click="run(api.stopPairing)">Cancel</button>
        </div>
        <button v-else class="btn wide" :disabled="busy" @click="run(api.startPairing)"><Link2 :size="18" /> Show a pairing code</button>

        <template v-if="!st.pairing">
          <div class="nearby-head">
            <h2 class="section-title">Nearby, waiting to pair</h2>
            <button class="btn small-btn" :disabled="busy || scanning" @click="rescan"><RefreshCw :size="15" :class="{ spin: scanning }" /> Search again</button>
          </div>
          <p v-if="st.running" class="muted small looking">
            <Radar :size="15" class="pulse" /> Looking for devices {{ st.mode === 'relay' ? 'nearby' : 'on this Wi-Fi' }}…
          </p>
          <div v-if="st.nearby.length" class="pane rows">
            <template v-for="n in st.nearby" :key="n.endpoint">
              <button class="item" @click="joining = joining?.endpoint === n.endpoint ? null : n; code = ''">
                <Laptop :size="20" class="muted" />
                <span class="flex left">{{ n.name }}</span>
              </button>
              <form v-if="joining?.endpoint === n.endpoint" class="join" @submit.prevent="pair">
                <input
                  v-model="code"
                  class="field"
                  inputmode="numeric"
                  autocomplete="one-time-code"
                  maxlength="7"
                  placeholder="6-digit code"
                  enterkeyhint="go"
                  @input="code = code.replace(/\D/g, '')"
                />
                <button class="btn primary" :disabled="busy || code.length !== 6">Pair</button>
              </form>
            </template>
          </div>
          <p v-else class="muted small">
            None yet. Tap “Show a pairing code” on the other device; it shows up here. Both need sync on and the same Wi-Fi.
          </p>

          <button v-if="!byAddress" class="btn link-btn" @click="byAddress = true">Not listed? Enter an address</button>
          <form v-else class="pane by-address" @submit.prevent="pairByAddress">
            <span class="muted small">The other device shows its address under its pairing code.</span>
            <input v-model="address" class="field" placeholder="192.168.1.20" inputmode="decimal" autocapitalize="off" autocorrect="off" />
            <input
              v-model="code"
              class="field"
              inputmode="numeric"
              autocomplete="one-time-code"
              maxlength="7"
              placeholder="6-digit code"
              @input="code = code.replace(/\D/g, '')"
            />
            <div class="row-end">
              <button type="button" class="btn" @click="byAddress = false">Cancel</button>
              <button class="btn primary" :disabled="busy || !address.trim() || code.length !== 6">Pair</button>
            </div>
          </form>
        </template>

        <p class="muted small tip">Using a VPN? Allow LAN connections in its settings (Proton VPN: Settings → Connection → Advanced).</p>
        <p class="muted small">Firewall on a computer? Allow port 47474 (UDP and TCP) on your home network.</p>
      </template>
    </template>
  </main>
</template>

<style scoped>
.seg {
  display: inline-flex; gap: 2px; padding: 3px; border-radius: 999px; margin-top: 4px;
  box-shadow: inset 0 1px 0 var(--glass-control-rim), inset 0 0 0 1px var(--line);
}
.opt { padding: 6px 14px; border-radius: 999px; font-size: 0.9rem; color: var(--muted); white-space: nowrap; }
.opt.chosen { color: var(--text); }
.hint { margin: 8px 0 0; }
.item { display: flex; align-items: center; gap: 12px; min-height: 56px; width: 100%; }
.flex { flex: 1; }
.left { text-align: left; }
.stack { display: flex; flex-direction: column; gap: 2px; text-align: left; min-width: 0; }
.wide { width: 100%; }
.pairing { display: flex; flex-direction: column; align-items: center; gap: 8px; padding: 18px 16px; text-align: center; }
.code { font-size: 2.2rem; font-weight: 600; letter-spacing: 0.08em; font-variant-numeric: tabular-nums; }
.join { display: flex; gap: 8px; padding: 0 0 12px 32px; }
.tip { margin-top: 24px; }
.link-btn { width: 100%; margin-top: 10px; }
.by-address { display: flex; flex-direction: column; gap: 8px; padding: 14px; margin-top: 10px; }
.row-end { display: flex; justify-content: flex-end; gap: 8px; }
.nearby-head { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
.small-btn { min-height: 32px; padding: 4px 10px; font-size: 0.85rem; margin-top: 16px; }
.looking { display: flex; align-items: center; gap: 6px; margin: 0 0 8px; }
.syncing { font-weight: 400; }
.pulse { animation: pulse 1.6s ease-in-out infinite; }
@keyframes pulse { 50% { opacity: 0.35; } }
.spin { animation: spin 1s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
</style>
