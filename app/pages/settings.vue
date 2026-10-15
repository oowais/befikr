<script setup lang="ts">
import { getVersion } from '@tauri-apps/api/app'
import type { Settings, Status } from '~/composables/api'

const router = useRouter()
const { design, set: setDesign } = useDesign()
const sound = useSound()
const settings = ref<Settings | null>(null)
const status = ref<Status | null>(null)
const reremind = ref('')
const newWord = ref('')
const newTime = ref('')
const error = ref('')
const version = ref('')
onMounted(async () => (version.value = await getVersion().catch(() => '')))

async function load() {
  try {
    settings.value = await api.settings()
    reremind.value = settings.value.reremindMinutes.join(', ')
  } catch (e) {
    error.value = String(e)
  }
  status.value = await reminders.status().catch(() => null)
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
      <button class="icon-btn" aria-label="Back" @click="router.back()">←</button>
      <h1>Settings</h1>
    </header>
    <p v-if="error" class="error">{{ error }}</p>

    <h2 class="section-title">Design</h2>
    <div class="designs">
      <button
        v-for="d in DESIGNS"
        :key="d.value"
        class="btn design-option"
        :aria-pressed="design === d.value"
        @click="setDesign(d.value)"
      >
        <b>{{ d.label }}<span v-if="design === d.value"> ✓</span></b>
        <span class="muted small">{{ d.hint }}</span>
      </button>
    </div>

    <h2 class="section-title">Feedback</h2>
    <div class="group">
      <label class="perm">
        <span>Sound when completing</span>
        <input
          type="checkbox"
          class="switch"
          :checked="sound.enabled.value"
          @change="
            (e) => {
              sound.setEnabled((e.target as HTMLInputElement).checked)
              if ((e.target as HTMLInputElement).checked) sound.complete()
            }
          "
        />
      </label>
    </div>

    <template v-if="status">
      <h2 class="section-title">Reminders</h2>
      <div class="group">
      <div class="perm">
        <span>Notifications</span>
        <span v-if="status.notifications">✅</span>
        <button v-else class="btn" @click="run(reminders.requestNotifications)">Allow</button>
      </div>
      <div class="perm">
        <span>Exact alarms</span>
        <span v-if="status.exactAlarms">✅</span>
        <button v-else class="btn" @click="run(reminders.openExactAlarmSettings)">Allow</button>
      </div>
      <div class="perm">
        <span>Unrestricted battery</span>
        <span v-if="status.batteryUnrestricted">✅</span>
        <button v-else class="btn" @click="run(reminders.openBatterySettings)">Allow</button>
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
      <div class="group">
      <div v-for="(time, word) in settings.timeWords" :key="word" class="perm">
        <span class="word">{{ word }}</span>
        <input type="time" class="time" :value="time" @change="setWord(String(word), ($event.target as HTMLInputElement).value)" />
        <button class="icon-btn" :aria-label="`Remove ${word}`" @click="run(() => api.removeTimeWord(String(word)))">✕</button>
      </div>
      </div>
      <form class="row add" @submit.prevent="addWord">
        <input v-model="newWord" class="field" placeholder="dawn" />
        <input v-model="newTime" type="time" class="field time-field" />
        <button class="btn" :disabled="!newWord || !newTime">Add</button>
      </form>
    </template>

    <p v-if="version" class="muted small version">Befikr {{ version }}</p>
  </main>
</template>

<style scoped>
.perm { display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 8px 0; border-bottom: 1px solid var(--line); }
.word { flex: 1; text-transform: capitalize; }
.time { border: 1px solid var(--line); border-radius: 8px; padding: 6px 8px; background: var(--surface); }
.row { display: flex; gap: 8px; }
.add { margin-top: 12px; }
.time-field { width: 130px; flex: none; }
.small { font-size: 0.85rem; }
.error { color: var(--red); }
.version { margin-top: 40px; text-align: center; }
.switch { width: 22px; height: 22px; accent-color: var(--accent); }
.designs { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
.design-option { display: flex; flex-direction: column; align-items: flex-start; gap: 2px; text-align: left; padding: 12px 14px; border: 2px solid transparent; }
.design-option[aria-pressed='true'] { border-color: var(--accent); }
</style>
