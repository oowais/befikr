<script setup lang="ts">
// Re-remind intervals and time words ("morning" = 09:00), off the main
// Settings page since they're set once and rarely touched.
import { ArrowLeft, Plus, X } from 'lucide-vue-next'
import type { Settings } from '~/composables/api'

const router = useRouter()
const settings = ref<Settings | null>(null)
const reremind = ref('')
const newWord = ref('')
const newTime = ref('')
const error = ref('')

async function load() {
  try {
    settings.value = await api.settings()
    reremind.value = settings.value.reremindMinutes.join(', ')
  } catch (e) {
    error.value = String(e)
  }
}
onMounted(load)

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
// Time words use our own picker (time mode), never the native one.
/** Word being edited (null = the new word's time), or nothing open. */
const pickingFor = ref<{ word: string | null; time: string } | null>(null)
const todayAt = (t: string) => {
  const d = new Date()
  return localMs(`${d.getFullYear()}-${d.getMonth() + 1}-${d.getDate()}`, t)
}
const hhmm = (ms: number) => new Date(ms).toTimeString().slice(0, 5)
function pickedTime(ms: number | null) {
  const p = pickingFor.value
  if (ms == null || !p) return
  if (p.word == null) newTime.value = hhmm(ms)
  else setWord(p.word, hhmm(ms))
}

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
      <h1>Times</h1>
    </header>
    <p v-if="error" class="error">{{ error }}</p>

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
          <button type="button" class="field time" @click="pickingFor = { word: String(word), time }">{{ timeLabel(todayAt(time)) }}</button>
          <button class="icon-btn" :aria-label="`Remove ${word}`" @click="run(() => api.removeTimeWord(String(word)))"><X :size="18" /></button>
        </div>
        <form class="item" @submit.prevent="addWord">
          <input v-model="newWord" class="field" placeholder="e.g. dawn" autocapitalize="none" />
          <button type="button" class="field time" :class="{ muted: !newTime }" @click="pickingFor = { word: null, time: newTime || '09:00' }">
            {{ newTime ? timeLabel(todayAt(newTime)) : 'Time' }}
          </button>
          <button class="icon-btn" aria-label="Add time word" :disabled="!newWord || !newTime"><Plus :size="20" /></button>
        </form>
      </div>
      <p class="muted small">Type them in a task, e.g. “call mum tomorrow evening”.</p>
    </template>

    <DateTimeSheet
      v-if="pickingFor"
      mode="time"
      :model-value="todayAt(pickingFor.time)"
      :title="pickingFor.word ? `${pickingFor.word[0]!.toUpperCase()}${pickingFor.word.slice(1)}` : 'Time'"
      @update:model-value="pickedTime"
      @close="pickingFor = null"
    />
  </main>
</template>

<style scoped>
.item { display: flex; align-items: center; gap: 12px; min-height: 56px; width: 100%; }
.flex { flex: 1; }
.word { text-transform: capitalize; }
.time { width: 136px; flex: none; padding: 8px 10px; text-align: left; }
.row { display: flex; gap: 8px; }
</style>
