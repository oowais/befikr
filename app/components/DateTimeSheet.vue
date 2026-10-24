<script setup lang="ts">
// Date & time picker: a glass sheet with quick days, a month calendar,
// time-word chips and hour/minute wheels. Emits epoch ms (local time) or null.
// mode="time": wheels only (e.g. time words in Settings); the date part is ignored.
// The app never uses the native Android date/time pickers (<input type="date|time">).
import { Check, ChevronLeft, ChevronRight, X } from 'lucide-vue-next'

const props = withDefaults(
  defineProps<{
    modelValue: number | null
    title?: string
    allowClear?: boolean
    /** Earliest selectable instant (e.g. now, for snooze). */
    min?: number | null
    doneLabel?: string
    mode?: 'datetime' | 'time'
  }>(),
  { title: 'Date & time', allowClear: false, min: null, doneLabel: 'Done', mode: 'datetime' },
)
const emit = defineEmits<{ 'update:modelValue': [ms: number | null]; close: [] }>()
useBackClose(() => emit('close'))

const DAY = 86_400_000
const startOfDay = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate())
const sameDay = (a: Date, b: Date) => a.getTime() === b.getTime()
const addDays = (d: Date, n: number) => new Date(d.getFullYear(), d.getMonth(), d.getDate() + n)

const today = startOfDay(new Date())

// Initial value: the given one, else the next full hour today (or 09:00 tomorrow late at night).
function initial(): Date {
  if (props.modelValue != null) return new Date(props.modelValue)
  const d = new Date()
  d.setMinutes(0, 0, 0)
  d.setHours(d.getHours() + 1)
  if (d.getHours() >= 22 || d.getHours() < 6) {
    const t = addDays(today, 1)
    t.setHours(9)
    return t
  }
  return d
}
const init = initial()
const day = ref(startOfDay(init))
const hour = ref(init.getHours())
const minute = ref(Math.round(init.getMinutes() / 5) * 5 % 60)
const month = ref(new Date(day.value.getFullYear(), day.value.getMonth(), 1))

const value = computed(() => {
  const d = new Date(day.value)
  d.setHours(hour.value, minute.value, 0, 0)
  return d.getTime()
})
const tooEarly = computed(() => props.min != null && value.value < props.min)

// ---- Quick days
const nextWeekday = (wd: number) => addDays(today, ((wd - today.getDay() + 7) % 7) || 7)
const quickDays = computed(() => [
  { label: 'Today', d: today },
  { label: 'Tomorrow', d: addDays(today, 1) },
  { label: today.getDay() === 6 || today.getDay() === 0 ? 'Next weekend' : 'Weekend', d: nextWeekday(6) },
  { label: 'Next week', d: nextWeekday(1) },
])
function pickDay(d: Date) {
  day.value = d
  month.value = new Date(d.getFullYear(), d.getMonth(), 1)
}

// ---- Calendar (Monday first)
const monthLabel = computed(() => month.value.toLocaleDateString(undefined, { month: 'long', year: 'numeric' }))
const weekdays = computed(() =>
  Array.from({ length: 7 }, (_, i) => new Date(2024, 0, 1 + i).toLocaleDateString(undefined, { weekday: 'narrow' })),
)
const cells = computed(() => {
  const first = month.value
  const lead = (first.getDay() + 6) % 7
  const start = addDays(first, -lead)
  return Array.from({ length: 42 }, (_, i) => {
    const d = addDays(start, i)
    return {
      d,
      out: d.getMonth() !== first.getMonth(),
      past: props.min != null && d.getTime() + DAY <= props.min,
      today: sameDay(d, today),
      chosen: sameDay(d, day.value),
    }
  })
})
const shiftMonth = (n: number) => (month.value = new Date(month.value.getFullYear(), month.value.getMonth() + n, 1))

// ---- Time
const timeWords = useTimeWords()
const timeChips = computed(() =>
  Object.entries(timeWords.value)
    .map(([word, t]) => ({ word, t, h: Number(t.slice(0, 2)), m: Number(t.slice(3, 5)) }))
    .sort((a, b) => a.h * 60 + a.m - (b.h * 60 + b.m)),
)
function pickTime(h: number, m: number) {
  hour.value = h
  minute.value = Math.round(m / 5) * 5 % 60
  scrollWheels(true)
}

// ---- Wheels (scroll-snap columns)
const ITEM = 40
const hours = Array.from({ length: 24 }, (_, i) => i)
const minutes = Array.from({ length: 12 }, (_, i) => i * 5)
const hourWheel = ref<HTMLElement>()
const minuteWheel = ref<HTMLElement>()
const pad = (n: number) => String(n).padStart(2, '0')

let programmatic = false
function scrollWheels(smooth = false) {
  programmatic = true
  const behavior = smooth ? 'smooth' : 'instant'
  hourWheel.value?.scrollTo({ top: hour.value * ITEM, behavior: behavior as ScrollBehavior })
  minuteWheel.value?.scrollTo({ top: (minute.value / 5) * ITEM, behavior: behavior as ScrollBehavior })
  setTimeout(() => (programmatic = false), smooth ? 400 : 50)
}
let settle: ReturnType<typeof setTimeout> | undefined
function onWheelScroll() {
  if (programmatic) return
  clearTimeout(settle)
  settle = setTimeout(() => {
    if (hourWheel.value) hour.value = Math.min(23, Math.max(0, Math.round(hourWheel.value.scrollTop / ITEM)))
    if (minuteWheel.value) minute.value = Math.min(55, Math.max(0, Math.round(minuteWheel.value.scrollTop / ITEM) * 5))
  }, 90)
}
onMounted(() => nextTick(() => scrollWheels()))

const summary = computed(() => {
  if (props.mode === 'time') return `${pad(hour.value)}:${pad(minute.value)}`
  const v = new Date(value.value)
  return `${v.toLocaleDateString(undefined, { weekday: 'short', day: 'numeric', month: 'short' })} · ${pad(hour.value)}:${pad(minute.value)}`
})

function done() {
  if (tooEarly.value) return
  emit('update:modelValue', value.value)
  emit('close')
}
function clear() {
  emit('update:modelValue', null)
  emit('close')
}
</script>

<template>
  <div class="backdrop" @click.self="emit('close')">
    <div class="sheet popover" role="dialog" :aria-label="title">
      <header class="head">
        <b>{{ title }}</b>
        <button class="icon-btn" aria-label="Close" @click="emit('close')"><X :size="22" /></button>
      </header>

      <template v-if="mode !== 'time'">
      <div class="chips">
        <button
          v-for="q in quickDays"
          :key="q.label"
          class="chip"
          :class="{ chosen: sameDay(q.d, day) }"
          :disabled="min != null && q.d.getTime() + DAY <= min"
          @click="pickDay(q.d)"
        >
          {{ q.label }}
        </button>
      </div>

      <div class="cal">
        <div class="cal-head">
          <button class="icon-btn" aria-label="Previous month" @click="shiftMonth(-1)"><ChevronLeft :size="22" /></button>
          <span>{{ monthLabel }}</span>
          <button class="icon-btn" aria-label="Next month" @click="shiftMonth(1)"><ChevronRight :size="22" /></button>
        </div>
        <div class="grid">
          <span v-for="(w, i) in weekdays" :key="'w' + i" class="wd">{{ w }}</span>
          <button
            v-for="c in cells"
            :key="c.d.getTime()"
            class="cell"
            :class="{ out: c.out, today: c.today, ink: c.chosen }"
            :disabled="c.past"
            @click="pickDay(c.d)"
          >
            {{ c.d.getDate() }}
          </button>
        </div>
      </div>

      <div v-wheel-x class="chips">
        <button
          v-for="t in timeChips"
          :key="t.word"
          class="chip"
          :class="{ chosen: t.h === hour && t.m === minute }"
          @click="pickTime(t.h, t.m)"
        >
          <span class="cap">{{ t.word }}</span> <span class="muted">{{ t.t }}</span>
        </button>
      </div>

      </template>

      <div class="wheels">
        <div class="band" />
        <div ref="hourWheel" class="wheel" @scroll.passive="onWheelScroll">
          <div v-for="h in hours" :key="h" class="item" :class="{ on: h === hour }" @click="pickTime(h, minute)">{{ pad(h) }}</div>
        </div>
        <span class="colon">:</span>
        <div ref="minuteWheel" class="wheel" @scroll.passive="onWheelScroll">
          <div v-for="m in minutes" :key="m" class="item" :class="{ on: m === minute }" @click="pickTime(hour, m)">{{ pad(m) }}</div>
        </div>
      </div>

      <footer class="foot">
        <span class="summary" :class="{ error: tooEarly }">{{ tooEarly ? 'That time has passed' : summary }}</span>
        <button v-if="allowClear" class="btn" @click="clear">No date</button>
        <button class="btn primary" :disabled="tooEarly" @click="done"><Check :size="18" /> {{ doneLabel }}</button>
      </footer>
    </div>
  </div>
</template>

<style scoped>
.backdrop {
  position: fixed; inset: 0; z-index: 60; background: oklch(0 0 0 / 22%);
  display: flex; align-items: flex-end; animation: fade-in 0.15s;
}
.sheet {
  width: calc(100% - 16px); max-width: 520px; margin: 0 auto 8px; border-radius: 22px;
  padding: 8px 14px calc(14px + env(safe-area-inset-bottom));
  display: flex; flex-direction: column; gap: 12px; max-height: 92vh; overflow-y: auto;
  animation: float-in 0.24s cubic-bezier(0.22, 1, 0.36, 1);
}
.head { display: flex; align-items: center; justify-content: space-between; padding-left: 4px; }
.head b { font-size: 1.05rem; }
.chips { display: flex; gap: 8px; overflow-x: auto; scrollbar-width: none; padding: 2px; }
.chip:disabled { opacity: 0.35; }
.cap { text-transform: capitalize; }

.cal-head { display: flex; align-items: center; justify-content: space-between; font-weight: 500; }
.grid { display: grid; grid-template-columns: repeat(7, 1fr); gap: 2px; text-align: center; }
.wd { font-size: 0.75rem; color: var(--muted); padding: 4px 0; }
.cell {
  height: 38px; border-radius: 10px; font-variant-numeric: tabular-nums;
  border: 1px solid transparent;
}
.cell.out { color: var(--muted); opacity: 0.6; }
.cell.today:not(.ink) {
  background: var(--glass-control);
  box-shadow: inset 0 1px 0 var(--glass-control-rim), inset 0 0 0 1px var(--glass-edge);
  font-weight: 600;
}
.cell:disabled { opacity: 0.25; }
.cell:active:not(:disabled):not(.ink) { background: var(--glass-state); }

.wheels {
  position: relative; display: flex; justify-content: center; align-items: center; gap: 4px;
  height: 200px;
}
.band {
  position: absolute; left: 0; right: 0; top: 80px; height: 40px; border-radius: var(--radius-control);
  background: var(--glass-state);
  box-shadow: inset 0 1px 0 var(--glass-state-rim), 0 0 0 1px var(--glass-state-edge);
  pointer-events: none;
}
.wheel {
  position: relative; width: 84px; height: 200px; overflow-y: auto; scroll-snap-type: y mandatory;
  padding: 80px 0; scrollbar-width: none; overscroll-behavior: contain;
  mask-image: linear-gradient(transparent, #000 30%, #000 70%, transparent);
}
.wheel::-webkit-scrollbar { display: none; }
.item {
  height: 40px; line-height: 40px; text-align: center; scroll-snap-align: center;
  font-size: 1.35rem; font-variant-numeric: tabular-nums; color: var(--muted);
}
.item.on { color: var(--text); font-weight: 600; }
.colon { font-size: 1.35rem; font-weight: 600; position: relative; }

.foot { display: flex; align-items: center; gap: 8px; }
.summary { flex: 1; font-weight: 500; }
</style>
