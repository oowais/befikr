<script setup lang="ts">
// Re-remind and time words ("morning" = 09:00), off the main
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

// Re-remind after a swipe: on (1 h) by default, interval from a few presets.
const PRESETS = [15, 30, 60, 120]
const minLabel = (m: number) => (m % 60 === 0 ? `${m / 60} h` : m > 60 ? `${Math.floor(m / 60)} h ${m % 60} min` : `${m} min`)
const reremind = computed(() => settings.value?.reremindMinutes ?? null)
const choices = computed(() => {
  const m = reremind.value
  return m == null || PRESETS.includes(m) ? PRESETS : [...PRESETS, m].sort((a, b) => a - b)
})
function setReremind(m: number | null) {
  settings.value!.reremindMinutes = m
  run(() => api.setReremindMinutes(m))
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
      <div class="pane rows">
        <div class="item">
          <span class="flex">Remind again after a swipe</span>
          <button
            type="button"
            class="switch"
            :class="{ on: reremind != null }"
            role="switch"
            aria-label="Remind again after a swipe"
            :aria-checked="reremind != null"
            @click="setReremind(reremind == null ? 60 : null)"
          ><span class="knob" /></button>
        </div>
        <div v-if="reremind != null" class="item">
          <div class="seg" role="radiogroup" aria-label="Remind again after">
            <button
              v-for="m in choices"
              :key="m"
              type="button"
              role="radio"
              class="opt"
              :class="{ chosen: reremind === m }"
              :aria-checked="reremind === m"
              @click="setReremind(m)"
            >{{ minLabel(m) }}</button>
          </div>
        </div>
      </div>
      <p class="muted small">
        {{ reremind == null
          ? 'A reminder stays until you act on it; swiped away, it’s gone.'
          : `A reminder stays until you act on it; swiped away, it comes back every ${minLabel(reremind)}.` }}
      </p>

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
.seg {
  display: inline-flex; flex-wrap: wrap; gap: 2px; padding: 3px; border-radius: 999px;
  box-shadow: inset 0 1px 0 var(--glass-control-rim), inset 0 0 0 1px var(--line);
}
.opt { padding: 6px 14px; border-radius: 999px; font-size: 0.9rem; color: var(--muted); white-space: nowrap; font-variant-numeric: tabular-nums; }
.opt.chosen { color: var(--text); }
</style>
