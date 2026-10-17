<script setup lang="ts">
import { getVersion } from '@tauri-apps/api/app'
import { AlarmClock, ArrowLeftRight, BatteryFull, Bell, Check, ChevronDown, ChevronRight, ChevronUp, Clock, Download, FolderSync } from 'lucide-vue-next'
import type { BackupStatus, DesktopPrefs, Settings, Status, SyncStatus } from '~/composables/api'

const router = useRouter()
const sound = useSound()
const carefree = useCarefree()
// This computer's startup (desktop only; null on Android).
const desktop = ref<DesktopPrefs | null>(null)
onMounted(async () => (desktop.value = await api.desktopPrefs().catch(() => null)))
async function setAutostart(on: boolean) {
  desktop.value!.autostart = on
  await run(() => api.setAutostart(on))
  desktop.value = await api.desktopPrefs().catch(() => desktop.value)
}
async function setStartHidden(on: boolean) {
  desktop.value!.startHidden = on
  await run(() => api.setStartHidden(on))
}
const settings = ref<Settings | null>(null)
const status = ref<Status | null>(null)
// All permissions granted: one collapsed "set up" row; tap to see them.
const permsOpen = ref(false)
const allSet = computed(() => !!status.value && allGranted(status.value))
const error = ref('')
const version = ref('')
const backup = ref<BackupStatus | null>(null)
onMounted(async () => (version.value = await getVersion().catch(() => '')))
const update = useUpdate()
const updateNote = ref('')
const updateFailed = ref(false)
async function checkUpdate() {
  updateNote.value = ''
  updateFailed.value = false
  try {
    await update.check(true)
    if (!update.available.value) updateNote.value = 'Up to date'
  } catch (e) {
    updateNote.value = `Couldn't check: ${e}`
    updateFailed.value = true
  }
}
async function installUpdate() {
  updateNote.value = ''
  updateFailed.value = false
  try {
    const r = await update.install()
    if (r.needsPermission) updateNote.value = 'Allow Befikr to install apps, then tap Update again'
  } catch (e) {
    updateNote.value = `Update failed: ${e}`
    updateFailed.value = true
  }
}

async function load() {
  try {
    settings.value = await api.settings()
  } catch (e) {
    error.value = String(e)
  }
  status.value = await reminders.status().catch(() => null)
  backup.value = await reminders.backupStatus().catch(() => null)
}
onMounted(load)
const { theme, setTheme } = useTheme()
const sync = ref<SyncStatus | null>(null)
onMounted(async () => (sync.value = await api.syncStatus().catch(() => null)))
const syncLabel = computed(() => {
  const s = sync.value
  if (!s || s.mode === 'off') return 'Off'
  const n = s.peers.length
  return `On · ${n ? `${n} ${n === 1 ? 'device' : 'devices'}` : 'no devices yet'}`
})
const timesLabel = computed(() => {
  const s = settings.value
  if (!s) return ''
  const n = Object.keys(s.timeWords).length
  return `Re-remind after ${s.reremindMinutes.join(', ')} min · ${n} time ${n === 1 ? 'word' : 'words'}`
})
const backupLabel = computed(() => {
  const b = backup.value
  if (!b?.folder) return 'Daily backup off · export, restore'
  if (b.lastError) return 'Last backup failed'
  return b.lastAt ? `Daily · last: ${dayLabel(b.lastAt)}` : 'Daily · not yet'
})
const onVisible = () => document.visibilityState === 'visible' && load()
onMounted(() => document.addEventListener('visibilitychange', onVisible))
onUnmounted(() => document.removeEventListener('visibilitychange', onVisible))

async function run(f: () => Promise<unknown>) {
  try {
    await f()
    error.value = ''
    await load()
  } catch (e) {
    error.value = String(e)
  }
}
</script>

<template>
  <Screen title="Settings">
    <p v-if="error" class="error">{{ error }}</p>

    <div v-if="update.available.value" class="pane rows">
      <div class="item">
        <Download :size="20" class="muted" />
        <span class="flex stack">
          <span>Befikr {{ update.available.value }} is out</span>
          <span class="muted small">{{ update.canInstall.value ? `You have ${version}` : 'Update with your package manager or the install script' }}</span>
        </span>
        <button v-if="update.canInstall.value" class="btn primary" :disabled="update.installing.value" @click="installUpdate">
          {{ update.installing.value ? 'Downloading…' : 'Update' }}
        </button>
      </div>
    </div>

    <h2 class="section-title">Devices</h2>
    <div class="pane rows">
      <NuxtLink to="/sync" class="item">
        <ArrowLeftRight :size="20" class="muted" />
        <span class="flex stack">
          <span>Sync</span>
          <span class="muted small">{{ syncLabel }}</span>
        </span>
        <ChevronRight :size="20" class="muted" />
      </NuxtLink>
    </div>

    <h2 class="section-title">Reminders</h2>
    <div class="pane rows">
      <template v-if="status">
        <button v-if="allSet" class="item" :aria-expanded="permsOpen" @click="permsOpen = !permsOpen">
          <Check :size="20" class="muted" />
          <span class="flex left">All set up</span>
          <component :is="permsOpen ? ChevronUp : ChevronDown" :size="20" class="muted" />
        </button>
        <template v-if="!allSet || permsOpen">
        <div v-for="p in [
          { icon: Bell, label: 'Notifications', ok: status.notifications, allow: reminders.requestNotifications },
          { icon: AlarmClock, label: 'Exact alarms', ok: status.exactAlarms, allow: reminders.openExactAlarmSettings },
          { icon: BatteryFull, label: 'Unrestricted battery', ok: status.batteryUnrestricted, allow: reminders.openBatterySettings },
        ]" :key="p.label" class="item">
          <component :is="p.icon" :size="20" class="muted" />
          <span class="flex">{{ p.label }}</span>
          <span v-if="p.ok" class="checkbox checked"><Check :size="14" :stroke-width="3" /></span>
          <button v-else class="btn" @click="run(p.allow)">Allow</button>
        </div>
        </template>
      </template>
      <NuxtLink to="/timing" class="item">
        <Clock :size="20" class="muted" />
        <span class="flex stack">
          <span>Times</span>
          <span class="muted small">{{ timesLabel }}</span>
        </span>
        <ChevronRight :size="20" class="muted" />
      </NuxtLink>
    </div>
    <button v-if="status && (!allSet || permsOpen)" class="btn link" @click="router.push('/welcome')">Open setup guide</button>

    <h2 class="section-title">Theme</h2>
    <div class="seg" role="radiogroup" aria-label="Theme">
      <button
        v-for="t in (['system', 'light', 'dark'] as const)"
        :key="t"
        type="button"
        role="radio"
        class="opt"
        :class="{ chosen: theme === t }"
        :aria-checked="theme === t"
        @click="setTheme(t)"
      >
        {{ { system: 'System', light: 'Light', dark: 'Dark' }[t] }}
      </button>
    </div>

    <h2 class="section-title">Feedback</h2>
    <div class="pane rows">
      <button class="item" role="switch" :aria-checked="sound.enabled.value" @click="sound.setEnabled(!sound.enabled.value); sound.enabled.value && sound.complete()">
        <span class="flex left">Sound when completing</span>
        <span class="switch" :class="{ on: sound.enabled.value }"><span class="knob" /></span>
      </button>
      <button class="item" role="switch" :aria-checked="carefree.enabled.value" @click="carefree.setEnabled(!carefree.enabled.value)">
        <span class="flex left">Carefree days<span class="muted small hint">Streak beside the title; the landscape grows as you finish on time</span></span>
        <span class="switch" :class="{ on: carefree.enabled.value }"><span class="knob" /></span>
      </button>
    </div>

    <template v-if="desktop">
      <h2 class="section-title">Startup</h2>
      <div class="pane rows">
        <button class="item" role="switch" :aria-checked="desktop.autostart" @click="setAutostart(!desktop.autostart)">
          <span class="flex left">Start at login</span>
          <span class="switch" :class="{ on: desktop.autostart }"><span class="knob" /></span>
        </button>
        <button v-if="desktop.autostart" class="item" role="switch" :aria-checked="desktop.startHidden" @click="setStartHidden(!desktop.startHidden)">
          <span class="flex left">Start hidden<span class="muted small hint">In the tray, no window; reminders still come</span></span>
          <span class="switch" :class="{ on: desktop.startHidden }"><span class="knob" /></span>
        </button>
      </div>
    </template>

    <h2 class="section-title">Data</h2>
    <div class="pane rows">
      <NuxtLink to="/backup" class="item">
        <FolderSync :size="20" class="muted" />
        <span class="flex stack">
          <span>Backup</span>
          <span class="muted small" :class="{ error: backup?.lastError }">{{ backupLabel }}</span>
        </span>
        <ChevronRight :size="20" class="muted" />
      </NuxtLink>
    </div>

    <div v-if="version" class="muted small version">
      <p>Befikr {{ version }}</p>
      <button class="btn" :disabled="update.checking.value" @click="checkUpdate">{{ update.checking.value ? 'Checking…' : 'Check for updates' }}</button>
      <p v-if="updateNote" :class="{ error: updateFailed }">{{ updateNote }}</p>
    </div>
  </Screen>
</template>

<style scoped>
.item { display: flex; align-items: center; gap: 12px; min-height: 56px; width: 100%; }
a.item { color: inherit; text-decoration: none; }
.flex { flex: 1; }
.left { text-align: left; }
.hint { display: block; margin-top: 2px; }
.link { margin-top: 10px; width: 100%; }
.switch {
  width: 46px; height: 28px; border-radius: 999px; position: relative; flex: none;
  box-shadow: inset 0 1.5px 2px var(--well-shade), inset 0 -1px 0 var(--well-rim), inset 0 0 0 1px var(--glass-edge);
  transition: background-color 0.2s;
}
.switch.on {
  background: color-mix(in oklch, var(--accent) var(--primary-solid), transparent);
  box-shadow: inset 0 1px 0 var(--primary-rim), inset 0 -1px 0 color-mix(in oklch, var(--primary-under) 45%, transparent);
}
.knob {
  position: absolute; top: 3px; left: 3px; width: 22px; height: 22px; border-radius: 50%; background: #fff;
  box-shadow: inset 0 1px 0 oklch(1 0 0 / 90%), inset 0 -1px 0 oklch(0 0 0 / 14%), 0 1px 3px oklch(0 0 0 / 30%);
  transition: transform 0.2s cubic-bezier(0.22, 1, 0.36, 1);
}
.switch.on .knob { transform: translateX(18px); }
.version { margin-top: 40px; text-align: center; display: flex; flex-direction: column; align-items: center; gap: 8px; }
.version p { margin: 0; }
.seg {
  display: inline-flex; gap: 2px; padding: 3px; border-radius: 999px;
  box-shadow: inset 0 1px 0 var(--glass-control-rim), inset 0 0 0 1px var(--line);
}
.opt { padding: 6px 14px; border-radius: 999px; font-size: 0.9rem; color: var(--muted); white-space: nowrap; }
.opt.chosen { color: var(--text); }
.stack { display: flex; flex-direction: column; gap: 2px; text-align: left; }
</style>
