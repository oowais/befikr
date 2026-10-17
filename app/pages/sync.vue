<script setup lang="ts">
// Sync between this device and paired ones (P2P, no server of ours): on/off,
// this device's name, paired devices, and pairing: one device shows a QR
// code (a pairing link: id, one-time code, addresses, relay), the other
// scans it, or on a computer pastes the link.
import { listen } from '@tauri-apps/api/event'
import { ChevronRight, ClipboardPaste, Copy, GitMerge, Laptop, Link2, RefreshCw, ScanLine, X } from 'lucide-vue-next'
import { renderSVG } from 'uqr'
import type { SyncMode, SyncStatus } from '~/composables/api'

const { show } = useToast()
const st = ref<SyncStatus | null>(null)
const error = ref('')
const name = ref('')
const busy = ref(false)
const confirmRemove = ref<string | null>(null)
const now = ref(Date.now())

const modes: { id: SyncMode; label: string }[] = [
  { id: 'off', label: 'Off' },
  { id: 'relay', label: 'On' },
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

const ago = (ms: number | null) => {
  if (ms == null) return 'Not synced yet'
  const s = Math.max(0, Math.round((now.value - ms) / 1000))
  if (s < 60) return 'Synced just now'
  return `Synced ${span(s * 1000)} ago`
}
// How fresh a device's copy is: green is normal; orange after a few hours,
// red after two days (or never), when that device's list may be out of date.
const HOUR_MS = 3_600_000
const freshness = (ms: number | null) => {
  if (ms == null) return 'red'
  const age = now.value - ms
  return age < 6 * HOUR_MS ? 'green' : age < 48 * HOUR_MS ? 'orange' : 'red'
}
const left = computed(() => {
  const s = Math.max(0, Math.round(((st.value?.pairing?.untilMs ?? 0) - now.value) / 1000))
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`
})
const on = computed(() => st.value && st.value.mode !== 'off')
const isDesktop = typeof navigator !== 'undefined' && !/Android/i.test(navigator.userAgent)

// QR code of the pairing link (dark on white in both themes, so cameras read it).
const qr = computed(() => (st.value?.pairing?.link ? renderSVG(st.value.pairing.link, { border: 2 }) : ''))
async function copyLink() {
  const link = st.value?.pairing?.link
  if (!link) return
  await navigator.clipboard.writeText(link).then(() => show({ message: 'Link copied' }), (e) => (error.value = String(e)))
}

function pairWithLink(text: string) {
  if (!text.trim().startsWith('befikr://pair?')) {
    error.value = 'That isn’t a Befikr pairing code.'
    return
  }
  run(() => api.pairWithLink(text.trim()), 'Paired')
}

// Phones scan the other device's QR code with the camera (QrScanner).
const camera = ref(false)
function scanned(text: string) {
  camera.value = false
  pairWithLink(text)
}
function scanFailed(message: string) {
  camera.value = false
  error.value = message
}

// Computers have no camera to scan with: paste the link ("Copy link" there).
const pasting = ref(false)
const pasted = ref('')
function pairPasted() {
  pairWithLink(pasted.value)
  pasting.value = false
  pasted.value = ''
}
</script>

<template>
  <Screen title="Sync">
    <template #actions>
      <button v-if="on && st?.peers.length" class="icon-btn" aria-label="Sync now" :disabled="busy" @click="run(api.syncNow, 'Synced')">
        <RefreshCw :size="20" :class="{ spin: busy }" />
      </button>
    </template>
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
      <p class="muted small hint">
        <template v-if="on">
          Devices connect directly when they can, otherwise through n0’s free public relays. Relays see that two devices
          connect, never your data (end-to-end encrypted).
        </template>
        <template v-else>Nothing leaves this device.</template>
      </p>

      <NuxtLink v-if="st.conflicts" to="/conflicts" class="pane item conflicts">
        <GitMerge :size="20" class="muted" />
        <span class="flex stack">
          <span>{{ st.conflicts }} {{ st.conflicts === 1 ? 'change' : 'changes' }} overwritten</span>
          <span class="muted small">Edited on two devices before syncing. Review them</span>
        </span>
        <ChevronRight :size="20" class="muted" />
      </NuxtLink>

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
              <span class="muted small when" :class="freshness(p.lastSyncAt)">
                <span class="dot" :class="freshness(p.lastSyncAt)" />{{ ago(p.lastSyncAt) }}<template v-if="freshness(p.lastSyncAt) === 'red'"> · open Befikr there</template>
              </span>
            </span>
            <button v-if="confirmRemove === p.endpoint" class="btn danger" @click="run(() => api.unpair(p.endpoint), 'Removed'); confirmRemove = null">Remove</button>
            <button v-else class="icon-btn" :aria-label="`Remove ${p.name}`" @click="confirmRemove = p.endpoint"><X :size="18" /></button>
          </div>
        </div>
        <p v-else class="muted small">None yet. Add one below.</p>

        <h2 class="section-title">Add a device</h2>
        <div v-if="st.pairing" class="pane pairing">
          <span class="muted small">On the other device, open Sync and tap <b>Scan a pairing code</b>.</span>
          <div v-if="qr" class="qr" role="img" aria-label="Pairing QR code" v-html="qr" />
          <span class="muted small">Closes in {{ left }}</span>
          <div class="row-end">
            <button v-if="st.pairing.link" class="btn" @click="copyLink"><Copy :size="16" /> Copy link</button>
            <button class="btn" @click="run(api.stopPairing)">Cancel</button>
          </div>
        </div>
        <template v-else>
          <button class="btn wide" :disabled="busy" @click="run(api.startPairing)"><Link2 :size="18" /> Show a pairing code</button>
          <button v-if="!isDesktop" class="btn wide gap" :disabled="busy" @click="error = ''; camera = true"><ScanLine :size="18" /> Scan a pairing code</button>
          <template v-else>
            <button v-if="!pasting" class="btn wide gap" :disabled="busy" @click="pasting = true"><ClipboardPaste :size="18" /> Paste a pairing link</button>
            <form v-else class="pane paste" @submit.prevent="pairPasted">
              <span class="muted small">On the other device: Show a pairing code → Copy link.</span>
              <input v-model="pasted" class="field" placeholder="befikr://pair?…" autocapitalize="off" autocorrect="off" spellcheck="false" />
              <div class="row-end">
                <button type="button" class="btn" @click="pasting = false; pasted = ''">Cancel</button>
                <button class="btn primary" :disabled="busy || !pasted.trim()">Pair</button>
              </div>
            </form>
          </template>
        </template>
      </template>
    </template>
    <QrScanner v-if="camera" @found="scanned" @error="scanFailed" @close="camera = false" />
  </Screen>
</template>

<style scoped>
.when { display: inline-flex; align-items: center; gap: 6px; }
.when.orange { color: var(--orange); }
.when.red { color: var(--red); }
.seg {
  display: inline-flex; gap: 2px; padding: 3px; border-radius: 999px; margin-top: 4px;
  box-shadow: inset 0 1px 0 var(--glass-control-rim), inset 0 0 0 1px var(--line);
}
.opt { padding: 6px 14px; border-radius: 999px; font-size: 0.9rem; color: var(--muted); white-space: nowrap; }
.opt.chosen { color: var(--text); }
.hint { margin: 8px 0 0; }
.item { display: flex; align-items: center; gap: 12px; min-height: 56px; width: 100%; }
a.item { color: inherit; text-decoration: none; }
.conflicts { padding: 10px 14px; margin-top: 16px; }
.flex { flex: 1; }
.stack { display: flex; flex-direction: column; gap: 2px; text-align: left; min-width: 0; }
.wide { width: 100%; }
.gap { margin-top: 8px; }
.pairing { display: flex; flex-direction: column; align-items: center; gap: 8px; padding: 18px 16px; text-align: center; }
.qr { width: 200px; height: 200px; padding: 8px; border-radius: 12px; background: #fff; }
.qr :deep(svg) { display: block; width: 100%; height: 100%; }
.paste { display: flex; flex-direction: column; gap: 8px; padding: 14px; margin-top: 8px; }
.row-end { display: flex; justify-content: flex-end; gap: 8px; }
.syncing { font-weight: 400; }
.spin { animation: spin 1s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
</style>
