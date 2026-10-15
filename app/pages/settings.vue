<script setup lang="ts">
import { getVersion } from '@tauri-apps/api/app'
import { AlarmClock, ArrowLeft, BatteryFull, Bell, Check, Download, FolderSync, Plus, RefreshCw, Upload, X } from 'lucide-vue-next'
import type { BackupStatus, Settings, Status } from '~/composables/api'

const router = useRouter()
const sound = useSound()
const settings = ref<Settings | null>(null)
const status = ref<Status | null>(null)
const reremind = ref('')
const newWord = ref('')
const newTime = ref('')
const error = ref('')
const version = ref('')
const backup = ref<BackupStatus | null>(null)
const backupBusy = ref(false)
const importPath = ref<string | null>(null)
const { show } = useToast()
onMounted(async () => (version.value = await getVersion().catch(() => '')))

async function load() {
  try {
    settings.value = await api.settings()
    reremind.value = settings.value.reremindMinutes.join(', ')
  } catch (e) {
    error.value = String(e)
  }
  status.value = await reminders.status().catch(() => null)
  backup.value = await reminders.backupStatus().catch(() => null)
}

async function backupAction(f: () => Promise<BackupStatus | unknown>, done?: string) {
  backupBusy.value = true
  error.value = ''
  try {
    const r = await f()
    if (r && typeof r === 'object' && 'folder' in r) backup.value = r as BackupStatus
    if (done) show({ message: done })
  } catch (e) {
    error.value = String(e)
  } finally {
    backupBusy.value = false
    backup.value = await reminders.backupStatus().catch(() => backup.value)
  }
}
async function exportCopy() {
  const r = await reminders.exportBackup().catch((e) => ((error.value = String(e)), null))
  if (r?.saved) show({ message: 'Backup saved' })
}
async function chooseImport() {
  const r = await reminders.pickImportFile().catch((e) => ((error.value = String(e)), null))
  if (r?.path) importPath.value = r.path
}
async function confirmImport() {
  const path = importPath.value
  importPath.value = null
  if (!path) return
  try {
    await api.importBackup(path)
    await loadTimeWords(true)
    await load()
    show({ message: 'Backup restored' })
  } catch (e) {
    error.value = String(e)
  }
}
onMounted(load)
const onVisible = () => document.visibilityState === 'visible' && load()
onMounted(() => document.addEventListener('visibilitychange', onVisible))
onUnmounted(() => document.removeEventListener('visibilitychange', onVisible))

async function run(f: () => Promise<unknown>) {
  try {
    await f()
    error.value = ''
    await load()
    await loadTimeWords(true)
  } catch (e) {
    error.value = String(e)
  }
}

const saveReremind = () =>
  run(() => api.setReremindMinutes(reremind.value.split(/[,\s]+/).filter(Boolean).map(Number)))
const setWord = (w: string, t: string) => run(() => api.setTimeWord(w, t))
function addWord() {
  if (!newWord.value || !newTime.value) return
  const [w, t] = [newWord.value, newTime.value]
  newWord.value = newTime.value = ''
  setWord(w, t)
}
</script>

<template>
  <main class="page">
    <header class="topbar">
      <button class="icon-btn" aria-label="Back" @click="router.back()"><ArrowLeft :size="22" /></button>
      <h1>Settings</h1>
    </header>
    <p v-if="error" class="error">{{ error }}</p>

    <template v-if="status">
      <h2 class="section-title">Reminders</h2>
      <div class="pane rows">
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
      </div>
      <button class="btn link" @click="router.push('/welcome')">Open setup guide</button>
    </template>

    <h2 class="section-title">Feedback</h2>
    <div class="pane rows">
      <button class="item" role="switch" :aria-checked="sound.enabled.value" @click="sound.setEnabled(!sound.enabled.value); sound.enabled.value && sound.complete()">
        <span class="flex left">Sound when completing</span>
        <span class="switch" :class="{ on: sound.enabled.value }"><span class="knob" /></span>
      </button>
    </div>

    <template v-if="backup">
      <h2 class="section-title">Backup</h2>
      <div class="pane rows">
        <div class="item">
          <FolderSync :size="20" class="muted" />
          <span class="flex stack">
            <span>Daily backup</span>
            <span class="muted small">{{ backup.folder ?? 'Off — choose a folder, e.g. your Syncthing folder' }}</span>
          </span>
          <button class="btn" :disabled="backupBusy" @click="backupAction(reminders.pickBackupFolder)">
            {{ backup.folder ? 'Change' : 'Choose' }}
          </button>
        </div>
        <div v-if="backup.folder" class="item">
          <RefreshCw :size="20" class="muted" :class="{ spin: backupBusy }" />
          <span class="flex stack">
            <span>{{ backup.lastAt ? `Last: ${dayLabel(backup.lastAt)}` : 'No backup yet' }}</span>
            <span v-if="backup.lastError" class="error small">{{ backup.lastError }}</span>
            <span v-else class="muted small">Every day, keeps the newest 7</span>
          </span>
          <button class="btn" :disabled="backupBusy" @click="backupAction(reminders.backupNow, 'Backed up')">Now</button>
          <button class="icon-btn" aria-label="Turn off daily backup" @click="backupAction(reminders.clearBackupFolder)"><X :size="18" /></button>
        </div>
        <button class="item" @click="exportCopy">
          <Upload :size="20" class="muted" /><span class="flex left">Export a copy…</span>
        </button>
        <button class="item" @click="chooseImport">
          <Download :size="20" class="muted" /><span class="flex left">Restore from a backup…</span>
        </button>
      </div>
      <div v-if="importPath" class="pane confirm">
        <b>Replace all tasks with this backup?</b>
        <span class="muted small">Your current tasks are saved first, just in case.</span>
        <div class="row end">
          <button class="btn" @click="importPath = null">Cancel</button>
          <button class="btn primary" @click="confirmImport">Replace</button>
        </div>
      </div>
    </template>

    <template v-if="settings">
      <h2 class="section-title">Re-remind every (minutes)</h2>
      <form class="row" @submit.prevent="saveReremind">
        <input v-model="reremind" class="field" inputmode="numeric" />
        <button class="btn">Save</button>
      </form>
      <p class="muted small">Unacted reminders come back after each interval; the last one repeats.</p>

      <h2 class="section-title">Time words</h2>
      <div class="pane rows">
        <div v-for="(time, word) in settings.timeWords" :key="word" class="item">
          <span class="flex word">{{ word }}</span>
          <input type="time" class="field time" :value="time" @change="setWord(String(word), ($event.target as HTMLInputElement).value)" />
          <button class="icon-btn" :aria-label="`Remove ${word}`" @click="run(() => api.removeTimeWord(String(word)))"><X :size="18" /></button>
        </div>
        <form class="item" @submit.prevent="addWord">
          <input v-model="newWord" class="field" placeholder="e.g. dawn" autocapitalize="none" />
          <input v-model="newTime" type="time" class="field time" />
          <button class="icon-btn" aria-label="Add time word" :disabled="!newWord || !newTime"><Plus :size="20" /></button>
        </form>
      </div>
    </template>

    <p v-if="version" class="muted small version">Befikr {{ version }}</p>
  </main>
</template>

<style scoped>
.item { display: flex; align-items: center; gap: 12px; min-height: 56px; width: 100%; }
.flex { flex: 1; }
.left { text-align: left; }
.word { text-transform: capitalize; }
.time { width: 136px; flex: none; padding: 8px 10px; }
.row { display: flex; gap: 8px; }
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
.version { margin-top: 40px; text-align: center; }
.stack { display: flex; flex-direction: column; gap: 2px; text-align: left; }
.confirm { margin-top: 10px; padding: 14px; display: flex; flex-direction: column; gap: 8px; }
.end { justify-content: flex-end; }
.spin { animation: spin 1s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
</style>
