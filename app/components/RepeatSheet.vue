<script setup lang="ts">
// Repeat builder for when typing doesn't cut it: presets, "every N
// day/week/month/year", weekdays, day of month or "the last Friday", and a
// date for yearly. Emits an RRULE body (no DTSTART) or null.
import { Check, Minus, Plus, X } from 'lucide-vue-next'
import { RRule, type Options, type Weekday } from 'rrule'

const props = withDefaults(
  defineProps<{ modelValue: string | null; allowClear?: boolean; /** Task date (ms): seeds weekday / day of month. */ start?: number | null }>(),
  { allowClear: true, start: null },
)
const emit = defineEmits<{ 'update:modelValue': [rrule: string | null]; close: [] }>()

type Unit = 'day' | 'week' | 'month' | 'year'
const UNITS: Unit[] = ['day', 'week', 'month', 'year']
const FREQ: Record<Unit, number> = { day: RRule.DAILY, week: RRule.WEEKLY, month: RRule.MONTHLY, year: RRule.YEARLY }
const WEEKDAYS = [RRule.MO, RRule.TU, RRule.WE, RRule.TH, RRule.FR, RRule.SA, RRule.SU]
const DAY_SHORT = Array.from({ length: 7 }, (_, i) => new Date(2024, 0, 1 + i).toLocaleDateString(undefined, { weekday: 'narrow' }))
const DAY_LONG = Array.from({ length: 7 }, (_, i) => new Date(2024, 0, 1 + i).toLocaleDateString(undefined, { weekday: 'long' }))
const MONTHS = Array.from({ length: 12 }, (_, i) => new Date(2024, i, 1).toLocaleDateString(undefined, { month: 'long' }))
const NTH = [
  { n: 1, label: 'first' },
  { n: 2, label: 'second' },
  { n: 3, label: 'third' },
  { n: 4, label: 'fourth' },
  { n: -1, label: 'last' },
]
const ordinal = (d: number) => (d === -1 ? 'last day' : `${d}${['th', 'st', 'nd', 'rd'][d % 100 > 10 && d % 100 < 14 ? 0 : Math.min(d % 10, 4) % 4] ?? 'th'}`)

const today = props.start != null ? new Date(props.start) : new Date()
const todayWd = (today.getDay() + 6) % 7 // Monday = 0

const unit = ref<Unit>('week')
const interval = ref(1)
const weekdays = ref<number[]>([todayWd])
const monthMode = ref<'day' | 'nth'>('day')
const monthDay = ref(today.getDate())
const nth = ref(1)
const nthWd = ref(todayWd)
const yearMonth = ref(today.getMonth() + 1)
const yearDay = ref(today.getDate())

// Prefill from an existing rule.
function load(rrule: string) {
  let o: Partial<Options>
  try {
    o = RRule.parseString(rrule)
  } catch {
    return
  }
  unit.value = UNITS.find((u) => FREQ[u] === o.freq) ?? 'week'
  interval.value = o.interval ?? 1
  const days = ([] as (Weekday | number | string)[]).concat(o.byweekday ?? [])
  const CODES = ['MO', 'TU', 'WE', 'TH', 'FR', 'SA', 'SU']
  const wd = (d: Weekday | number | string) => (typeof d === 'number' ? d : typeof d === 'string' ? CODES.indexOf(d) : d.weekday)
  const md = ([] as number[]).concat(o.bymonthday ?? [])
  if (unit.value === 'week' && days.length) weekdays.value = days.map(wd)
  if (unit.value === 'month') {
    const first = days[0]
    if (first != null && typeof first === 'object' && first.n) {
      monthMode.value = 'nth'
      nth.value = first.n
      nthWd.value = first.weekday
    } else if (md[0] != null) monthDay.value = md[0]
  }
  if (unit.value === 'year') {
    yearMonth.value = ([] as number[]).concat(o.bymonth ?? [])[0] ?? yearMonth.value
    yearDay.value = md[0] ?? yearDay.value
  }
}
if (props.modelValue) load(props.modelValue)

const options = computed<Partial<Options>>(() => {
  const o: Partial<Options> = { freq: FREQ[unit.value], ...(interval.value > 1 && { interval: interval.value }) }
  if (unit.value === 'week') o.byweekday = [...weekdays.value].sort().map((i) => WEEKDAYS[i]!)
  if (unit.value === 'month') {
    if (monthMode.value === 'day') o.bymonthday = [monthDay.value]
    else o.byweekday = [WEEKDAYS[nthWd.value]!.nth(nth.value)]
  }
  if (unit.value === 'year') {
    o.bymonth = [yearMonth.value]
    o.bymonthday = [yearDay.value]
  }
  return o
})
const rrule = computed(() => new RRule(options.value).toString().replace(/^RRULE:/, ''))
const summary = computed(() => new RRule(options.value).toText())

// ---- Presets: each sets unit, interval and weekdays; chosen when it gives the current rule.
type Preset = { label: string; unit: Unit; n?: number; days?: number[] }
const presets: Preset[] = [
  { label: 'Daily', unit: 'day' },
  { label: 'Weekdays', unit: 'week', days: [0, 1, 2, 3, 4] },
  { label: 'Weekly', unit: 'week' },
  { label: 'Fortnightly', unit: 'week', n: 2 },
  { label: 'Monthly', unit: 'month' },
  { label: 'Yearly', unit: 'year' },
]
function usePreset(p: Preset) {
  unit.value = p.unit
  interval.value = p.n ?? 1
  weekdays.value = p.days ?? [todayWd]
  monthMode.value = 'day'
  monthDay.value = today.getDate()
  yearMonth.value = today.getMonth() + 1
  yearDay.value = today.getDate()
}
const presetRule = (p: Preset) => {
  const o: Partial<Options> = { freq: FREQ[p.unit], ...(p.n && { interval: p.n }) }
  if (p.unit === 'week') o.byweekday = (p.days ?? [todayWd]).map((i) => WEEKDAYS[i]!)
  if (p.unit === 'month') o.bymonthday = [today.getDate()]
  if (p.unit === 'year') Object.assign(o, { bymonth: [today.getMonth() + 1], bymonthday: [today.getDate()] })
  return new RRule(o).toString()
}
const presetOn = (p: Preset) => presetRule(p) === `RRULE:${rrule.value}`

function toggleDay(i: number) {
  const s = new Set(weekdays.value)
  if (s.has(i)) {
    if (s.size > 1) s.delete(i)
  } else s.add(i)
  weekdays.value = [...s]
}
const unitLabel = (u: Unit) => (interval.value === 1 ? u : `${u}s`)

function done() {
  emit('update:modelValue', rrule.value)
  emit('close')
}
function clear() {
  emit('update:modelValue', null)
  emit('close')
}
</script>

<template>
  <div class="backdrop" @click.self="emit('close')">
    <div class="sheet popover" role="dialog" aria-label="Repeat">
      <header class="head">
        <b>Repeat</b>
        <button class="icon-btn" aria-label="Close" @click="emit('close')"><X :size="22" /></button>
      </header>

      <div class="chips">
        <button v-for="p in presets" :key="p.label" class="chip" :class="{ chosen: presetOn(p) }" @click="usePreset(p)">
          {{ p.label }}
        </button>
      </div>

      <div class="every">
        <span class="muted">Every</span>
        <div class="stepper">
          <button class="icon-btn" aria-label="Fewer" :disabled="interval <= 1" @click="interval--"><Minus :size="18" /></button>
          <span class="n">{{ interval }}</span>
          <button class="icon-btn" aria-label="More" :disabled="interval >= 99" @click="interval++"><Plus :size="18" /></button>
        </div>
      </div>
      <div class="segmented">
        <button v-for="u in UNITS" :key="u" class="seg" :class="{ chosen: unit === u }" @click="unit = u">{{ unitLabel(u) }}</button>
      </div>

      <div v-if="unit === 'week'" class="days">
        <button
          v-for="(d, i) in DAY_SHORT"
          :key="i"
          class="day"
          :class="{ ink: weekdays.includes(i) }"
          :aria-label="DAY_LONG[i]"
          :aria-pressed="weekdays.includes(i)"
          @click="toggleDay(i)"
        >
          {{ d }}
        </button>
      </div>

      <div v-if="unit === 'month'" class="pane rows modes">
        <label class="mode">
          <input v-model="monthMode" type="radio" value="day" />
          <span>On the</span>
          <select v-model.number="monthDay" class="field" @focus="monthMode = 'day'">
            <option v-for="d in 31" :key="d" :value="d">{{ ordinal(d) }}</option>
            <option :value="-1">last day</option>
          </select>
        </label>
        <label class="mode">
          <input v-model="monthMode" type="radio" value="nth" />
          <span>On the</span>
          <select v-model.number="nth" class="field" @focus="monthMode = 'nth'">
            <option v-for="o in NTH" :key="o.n" :value="o.n">{{ o.label }}</option>
          </select>
          <select v-model.number="nthWd" class="field" @focus="monthMode = 'nth'">
            <option v-for="(d, i) in DAY_LONG" :key="i" :value="i">{{ d }}</option>
          </select>
        </label>
      </div>

      <div v-if="unit === 'year'" class="mode">
        <span>On</span>
        <select v-model.number="yearMonth" class="field">
          <option v-for="(m, i) in MONTHS" :key="i" :value="i + 1">{{ m }}</option>
        </select>
        <select v-model.number="yearDay" class="field">
          <option v-for="d in 31" :key="d" :value="d">{{ d }}</option>
        </select>
      </div>

      <footer class="foot">
        <span class="summary">{{ summary }}</span>
        <button v-if="allowClear && modelValue" class="btn" @click="clear">Don't repeat</button>
        <button class="btn primary" @click="done"><Check :size="18" /> Done</button>
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
  display: flex; flex-direction: column; gap: 14px; max-height: 92vh; overflow-y: auto;
  animation: float-in 0.24s cubic-bezier(0.22, 1, 0.36, 1);
}
.head { display: flex; align-items: center; justify-content: space-between; padding-left: 4px; }
.head b { font-size: 1.05rem; }
.chips { display: flex; gap: 8px; overflow-x: auto; scrollbar-width: none; padding: 2px; }

.every { display: flex; align-items: center; justify-content: space-between; padding: 0 4px; }
.stepper { display: flex; align-items: center; gap: 4px; }
.stepper .icon-btn:disabled { opacity: 0.3; }
.n { min-width: 2ch; text-align: center; font-size: 1.3rem; font-weight: 600; font-variant-numeric: tabular-nums; }

.segmented {
  display: grid; grid-template-columns: repeat(4, 1fr); gap: 4px; padding: 4px; border-radius: 14px;
  box-shadow: inset 0 1.5px 2px var(--well-shade), inset 0 -1px 0 var(--well-rim), inset 0 0 0 1px var(--glass-edge);
}
.seg { padding: 8px 0; border-radius: 10px; text-transform: capitalize; font-size: 0.95rem; }
.seg.chosen { font-weight: 600; }

.days { display: grid; grid-template-columns: repeat(7, 1fr); gap: 6px; }
.day { aspect-ratio: 1; max-height: 44px; border-radius: 50%; font-weight: 500; box-shadow: inset 0 0 0 1px var(--line); }

.modes { padding: 0 12px; }
.mode { display: flex; align-items: center; gap: 8px; min-height: 52px; }
.mode .field { width: auto; flex: 1; min-width: 0; padding: 8px 10px; }
.mode input[type='radio'] { accent-color: var(--accent); width: 18px; height: 18px; margin: 0; }

.foot { display: flex; align-items: center; gap: 8px; }
.summary { flex: 1; font-weight: 500; }
.summary::first-letter { text-transform: uppercase; }
</style>
