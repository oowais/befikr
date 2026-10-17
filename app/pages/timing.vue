<script setup lang="ts">
// Re-remind intervals and time words ("morning" = 09:00), off the main
// Settings page since they're set once and rarely touched.
import { Plus, X } from 'lucide-vue-next'
import type { Settings } from '~/composables/api'

const settings = ref<Settings | null>(null)
const newWord = ref('')
const newTime = ref('')
const error = ref('')

async function load() {
  try {
    settings.value = await api.settings()
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

// Re-remind intervals as chips: numbers only, kept sorted and unique, saved
// on every change (at least one stays).
const MAX_MINUTES = 24 * 60
const newMinutes = ref('')
const minutes = computed(() => settings.value?.reremindMinutes ?? [])
const minLabel = (m: number) => (m % 60 === 0 ? `${m / 60} h` : m > 60 ? `${Math.floor(m / 60)} h ${m % 60} min` : `${m} min`)
const reremindSummary = computed(() => {
  const m = minutes.value
  if (!m.length) return ''
  const last = minLabel(m.at(-1)!)
  if (m.length === 1) return `Comes back every ${last} until you act on it.`
  return `Comes back after ${m.slice(0, -1).map(minLabel).join(', then ')}, then every ${last}.`
})
function saveMinutes(list: number[]) {
  const sorted = [...new Set(list)].sort((a, b) => a - b)
  settings.value!.reremindMinutes = sorted
  run(() => api.setReremindMinutes(sorted))
}
function digitsOnly(e: Event) {
  const el = e.target as HTMLInputElement
  el.value = el.value.replace(/\D/g, '').slice(0, 4)
  newMinutes.value = el.value
}
const newValid = computed(() => {
  const n = Number(newMinutes.value)
  return Number.isInteger(n) && n > 0 && n <= MAX_MINUTES && !minutes.value.includes(n)
})
function addMinutes() {
  if (!newValid.value) return
  saveMinutes([...minutes.value, Number(newMinutes.value)])
  newMinutes.value = ''
}
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
  <Screen title="Times">
    <p v-if="error" class="error">{{ error }}</p>

    <template v-if="settings">
      <h2 class="section-title">Re-remind</h2>
      <div class="chips">
        <span v-for="m in minutes" :key="m" class="chip split">
          <span class="part">{{ minLabel(m) }}</span>
          <button
            v-if="minutes.length > 1"
            type="button"
            class="x"
            :aria-label="`Remove ${minLabel(m)}`"
            @click="saveMinutes(minutes.filter((x) => x !== m))"
          ><X :size="14" /></button>
        </span>
        <form class="add" @submit.prevent="addMinutes">
          <input
            :value="newMinutes"
            class="field minutes"
            inputmode="numeric"
            pattern="[0-9]*"
            placeholder="min"
            aria-label="Add an interval in minutes"
            @input="digitsOnly"
          />
          <button class="icon-btn" aria-label="Add interval" :disabled="!newValid"><Plus :size="20" /></button>
        </form>
      </div>
      <p class="muted small">{{ reremindSummary }}</p>

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
  </Screen>
</template>

<style scoped>
.item { display: flex; align-items: center; gap: 12px; min-height: 56px; width: 100%; }
.flex { flex: 1; }
.word { text-transform: capitalize; }
.time { width: 136px; flex: none; padding: 8px 10px; text-align: left; }
.chip.split { padding: 0; gap: 0; display: inline-flex; align-items: center; }
.chip.split .part { padding: 6px 12px; font-variant-numeric: tabular-nums; }
.chip.split .part:has(+ .x) { padding-right: 2px; }
.chip.split .x { display: grid; place-items: center; width: 32px; height: 32px; color: var(--muted); }
.chips { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; }
.add { display: inline-flex; align-items: center; gap: 2px; }
.field.minutes { width: 72px; padding: 6px 10px; text-align: center; }
</style>
