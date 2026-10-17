<script setup lang="ts">
// Calendar for wide windows (computers, tablets): a week of day columns or a
// month grid. Tasks sit on their day in time order; later repeats are dashed
// and stay put. Drag a task to another day to move it there at the same time
// (a repeat moves only this occurrence: it's snoozed, the series stays).
// Drag from "No date" to give a task a day (09:00). Tap a task to open it,
// empty space in a day to add one there. Pointer events, not HTML drag and
// drop: the webview's file drop takes that over on Windows.
import { CalendarDays, ChevronLeft, ChevronRight, Repeat } from 'lucide-vue-next'
import type { CalendarItem, Row } from '~/composables/api'

type View = 'week' | 'month'
type Card = { id: string; title: string; at: number | null; rrule: string | null; upcoming: boolean }

const VIEW_KEY = 'befikr.calendarView'
const DEFAULT_TIME = '09:00'
const DAY_MS = 86_400_000
const MONTH_CHIPS = 3

const { open: openTask } = useTaskSheet()
const view = ref<View>(loadView())
const anchor = ref(dayStart(Date.now()))
const items = ref<CalendarItem[]>([])
const undated = ref<Row[]>([])
const now = ref(Date.now())
const error = ref('')
const adding = ref(false)

function loadView(): View {
  try {
    return localStorage.getItem(VIEW_KEY) === 'month' ? 'month' : 'week'
  } catch {
    return 'week'
  }
}
function setView(v: View) {
  view.value = v
  try {
    localStorage.setItem(VIEW_KEY, v)
  } catch {}
}

// ---- Dates (local days; weeks start on Monday)
function dayStart(ms: number) {
  const d = new Date(ms)
  return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime()
}
/** Same wall-clock day arithmetic across DST (a day isn't always 24 h). */
function addDays(ms: number, n: number) {
  const d = new Date(ms)
  return new Date(d.getFullYear(), d.getMonth(), d.getDate() + n).getTime()
}
function monday(ms: number) {
  const dow = (new Date(ms).getDay() + 6) % 7
  return addDays(ms, -dow)
}
const days = computed(() => {
  if (view.value === 'week') {
    const start = monday(anchor.value)
    return Array.from({ length: 7 }, (_, i) => addDays(start, i))
  }
  const d = new Date(anchor.value)
  const start = monday(new Date(d.getFullYear(), d.getMonth(), 1).getTime())
  return Array.from({ length: 42 }, (_, i) => addDays(start, i))
})
const from = computed(() => days.value[0]!)
const to = computed(() => addDays(days.value.at(-1)!, 1))
const today = computed(() => dayStart(now.value))
const month = computed(() => new Date(anchor.value).getMonth())

const fmt = (o: Intl.DateTimeFormatOptions) => new Intl.DateTimeFormat(undefined, o)
const title = computed(() => {
  if (view.value === 'month') return fmt({ month: 'long', year: 'numeric' }).format(anchor.value)
  const a = new Date(from.value)
  const b = new Date(addDays(to.value, -1))
  const sameMonth = a.getMonth() === b.getMonth()
  return `${fmt({ month: 'short', day: 'numeric' }).format(a)} – ${fmt(sameMonth ? { day: 'numeric' } : { month: 'short', day: 'numeric' }).format(b)}`
})
const weekday = (ms: number) => fmt({ weekday: 'short' }).format(ms)

function step(dir: 1 | -1) {
  if (view.value === 'week') anchor.value = addDays(anchor.value, 7 * dir)
  else {
    const d = new Date(anchor.value)
    anchor.value = new Date(d.getFullYear(), d.getMonth() + dir, 1).getTime()
  }
}
function goToday() {
  anchor.value = dayStart(Date.now())
}
function showWeek(day: number) {
  anchor.value = day
  setView('week')
}

// ---- Data
async function load() {
  now.value = Date.now()
  try {
    const [cal, open] = await Promise.all([api.calendar(from.value, to.value), api.listOpen()])
    items.value = cal
    undated.value = open.filter((r) => r.dueAt == null && r.snoozedUntil == null)
    error.value = ''
  } catch (e) {
    error.value = String(e)
  }
}
watch([from, to, listVersion], load, { immediate: true })
const onVisible = () => document.visibilityState === 'visible' && load()
onMounted(() => document.addEventListener('visibilitychange', onVisible))
onUnmounted(() => document.removeEventListener('visibilitychange', onVisible))

const byDay = computed(() => {
  const m = new Map<number, Card[]>()
  for (const i of items.value) {
    const k = dayStart(i.at)
    m.set(k, [...(m.get(k) ?? []), { id: i.id, title: i.title, at: i.at, rrule: i.rrule, upcoming: i.upcoming }])
  }
  return m
})
const cardsOn = (day: number) => byDay.value.get(day) ?? []
const overdue = (c: Card) => !c.upcoming && c.at != null && c.at < now.value

// ---- Drag to reschedule
const drag = reactive({ card: null as Card | null, x: 0, y: 0, over: null as number | null, moved: false })
let startX = 0
let startY = 0
// The click that ends a drag must not also open quick add on the day.
let justDragged = false

function down(e: PointerEvent, card: Card) {
  if (card.upcoming || (e.pointerType === 'mouse' && e.button !== 0)) return
  drag.card = card
  drag.moved = false
  startX = e.clientX
  startY = e.clientY
  ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
}
function move(e: PointerEvent) {
  if (!drag.card) return
  if (!drag.moved && Math.hypot(e.clientX - startX, e.clientY - startY) < 6) return
  drag.moved = true
  drag.x = e.clientX
  drag.y = e.clientY
  const cell = document.elementFromPoint(e.clientX, e.clientY)?.closest<HTMLElement>('[data-day]')
  drag.over = cell ? Number(cell.dataset.day) : null
}
function up() {
  const { card, moved, over } = drag
  drag.card = null
  drag.over = null
  drag.moved = false
  if (moved) {
    justDragged = true
    setTimeout(() => (justDragged = false))
  }
  if (!card) return
  if (!moved) return card.upcoming ? undefined : openTask(card.id)
  if (over != null && (card.at == null || over !== dayStart(card.at))) reschedule(card, over)
}
function cancelDrag() {
  drag.card = null
  drag.over = null
  drag.moved = false
}

/** Onto `day`, keeping the time of day (09:00 for a task without a date). */
async function reschedule(card: Card, day: number) {
  const at = card.at != null ? localMs(isoDay(day), hhmm(card.at)) : localMs(isoDay(day), DEFAULT_TIME)
  // Optimistic: move it now, the reload confirms.
  const item = items.value.find((i) => i.id === card.id && !i.upcoming)
  if (item) item.at = at
  else undated.value = undated.value.filter((r) => r.id !== card.id)
  try {
    if (card.rrule) await api.snoozeTask(card.id, at)
    else await api.updateTask(card.id, { dueAt: at, tz: deviceTz() })
  } catch (e) {
    error.value = String(e)
  }
  listChanged()
}
const pad = (n: number) => String(n).padStart(2, '0')
const isoDay = (ms: number) => {
  const d = new Date(ms)
  return `${d.getFullYear()}-${d.getMonth() + 1}-${d.getDate()}`
}
const hhmm = (ms: number) => {
  const d = new Date(ms)
  return `${pad(d.getHours())}:${pad(d.getMinutes())}`
}

// ---- Tap a day to add a task on it
function addOn(day: number) {
  if (justDragged) return
  let at = localMs(isoDay(day), DEFAULT_TIME)
  // Today after 09:00: the next full hour instead of the past.
  if (at < Date.now() && day === today.value) at = dayStart(Date.now()) + (new Date().getHours() + 1) * 3_600_000
  useQuickAddDraft().manualDue = at
  adding.value = true
}
const ghost = computed(() => (drag.card && drag.moved ? drag.card : null))
</script>

<template>
  <Screen title="Calendar" class="calendar" @pointermove="move" @pointerup="up" @pointercancel="cancelDrag">
    <template #actions>
      <span class="grow" />
      <div class="seg" role="radiogroup" aria-label="View">
        <button v-for="v in (['week', 'month'] as const)" :key="v" type="button" role="radio" class="opt" :class="{ chosen: view === v }" :aria-checked="view === v" @click="setView(v)">
          {{ v === 'week' ? 'Week' : 'Month' }}
        </button>
      </div>
    </template>

    <div class="nav">
      <button class="icon-btn" :aria-label="view === 'week' ? 'Previous week' : 'Previous month'" @click="step(-1)"><ChevronLeft :size="20" /></button>
      <h2 class="period">{{ title }}</h2>
      <button class="icon-btn" :aria-label="view === 'week' ? 'Next week' : 'Next month'" @click="step(1)"><ChevronRight :size="20" /></button>
      <button class="btn" @click="goToday"><CalendarDays :size="18" /> Today</button>
    </div>
    <p v-if="error" class="error">{{ error }}</p>

    <div class="layout" :class="view">
      <div class="grid" :class="view">
        <template v-if="view === 'month'">
          <div v-for="d in days.slice(0, 7)" :key="`h${d}`" class="dow">{{ weekday(d) }}</div>
        </template>
        <section
          v-for="d in days"
          :key="d"
          class="day pane"
          :class="{ today: d === today, over: drag.over === d, past: d < today, other: view === 'month' && new Date(d).getMonth() !== month }"
          :data-day="d"
          @click.self="addOn(d)"
        >
          <header class="day-head" @click="view === 'month' ? showWeek(d) : addOn(d)">
            <span v-if="view === 'week'" class="muted">{{ weekday(d) }}</span>
            <b>{{ new Date(d).getDate() }}</b>
          </header>
          <button
            v-for="c in (view === 'month' ? cardsOn(d).slice(0, MONTH_CHIPS) : cardsOn(d))"
            :key="c.id + (c.upcoming ? c.at : '')"
            type="button"
            class="card"
            :class="{ upcoming: c.upcoming, late: overdue(c), dragged: drag.card?.id === c.id && drag.moved }"
            :title="c.upcoming ? 'Later repeat' : undefined"
            @pointerdown="down($event, c)"
          >
            <span class="when" :class="{ late: overdue(c) }">{{ overdue(c) ? 'Overdue' : timeLabel(c.at!) }}</span>
            <span class="what"><Repeat v-if="c.rrule" :size="12" class="rep" /> {{ c.title }}</span>
          </button>
          <button v-if="view === 'month' && cardsOn(d).length > MONTH_CHIPS" type="button" class="more muted" @click="showWeek(d)">
            +{{ cardsOn(d).length - MONTH_CHIPS }} more
          </button>
        </section>
      </div>

      <aside class="tray">
        <h3 class="section-title">No date</h3>
        <p v-if="!undated.length" class="muted small">Nothing undated</p>
        <button
          v-for="r in undated"
          :key="r.id"
          type="button"
          class="card"
          :class="{ dragged: drag.card?.id === r.id && drag.moved }"
          @pointerdown="down($event, { id: r.id, title: r.title, at: null, rrule: r.rrule, upcoming: false })"
        >
          <span class="what">{{ r.title }}</span>
        </button>
        <p class="muted small hint">Drag onto a day to give it a date.</p>
      </aside>
    </div>

    <div v-if="ghost" class="card ghost" :style="{ left: `${drag.x}px`, top: `${drag.y}px` }">
      <span v-if="ghost.at" class="when">{{ timeLabel(ghost.at) }}</span>
      <span class="what">{{ ghost.title }}</span>
    </div>

    <QuickAdd v-if="adding" @close="adding = false" @saved="load" />
  </Screen>
</template>

<style scoped>
.calendar { max-width: 1400px; }
.grow { flex: 1; }
.nav { display: flex; align-items: center; gap: 6px; margin: 4px 0 12px; }
.period { font-size: 1.1rem; font-weight: 500; margin: 0 6px; min-width: 9ch; }
.nav .btn { margin-left: auto; }

.layout { display: grid; grid-template-columns: minmax(0, 1fr) 200px; gap: 12px; align-items: start; }
@media (max-width: 860px) { .layout { grid-template-columns: minmax(0, 1fr); } }

.grid { display: grid; gap: 6px; }
.grid.week { grid-template-columns: repeat(7, minmax(110px, 1fr)); overflow-x: auto; }
.grid.month { grid-template-columns: repeat(7, minmax(0, 1fr)); }
.dow { font-size: 0.75rem; color: var(--muted); padding: 0 6px; }

.day { padding: 6px; display: flex; flex-direction: column; gap: 4px; cursor: pointer; transition: box-shadow 0.15s; }
.grid.week .day { min-height: 60vh; }
.grid.month .day { min-height: 104px; }
.day.past { opacity: 0.8; }
.day.other { opacity: 0.45; }
.day.today { box-shadow: inset 0 0 0 1.5px var(--accent); }
.day.over { box-shadow: inset 0 0 0 2px var(--accent); background: color-mix(in oklch, var(--accent) 6%, transparent); }
.day-head { display: flex; align-items: baseline; gap: 6px; padding: 2px 4px 4px; font-size: 0.85rem; }
.day-head b { font-weight: 500; }

.card {
  display: flex; flex-direction: column; align-items: flex-start; gap: 1px; width: 100%; text-align: left;
  padding: 5px 7px; border-radius: 8px; font-size: 0.85rem; line-height: 1.3; cursor: grab;
  box-shadow: inset 0 0 0 1px var(--line); touch-action: none; user-select: none;
}
.card .when { font-size: 0.72rem; color: var(--muted); }
.card .when.late { color: var(--red); }
.card .what { overflow: hidden; text-overflow: ellipsis; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; }
.card .rep { vertical-align: -1px; opacity: 0.7; }
.card.late { box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--red) 45%, transparent); }
.card.upcoming { cursor: default; color: var(--muted); box-shadow: none; outline: 1px dashed var(--line); outline-offset: -1px; }
.card.dragged { opacity: 0.35; }
.grid.month .card { padding: 2px 6px; }
.grid.month .card .when { display: none; }
.grid.month .card .what { -webkit-line-clamp: 1; }
.more { font-size: 0.75rem; text-align: left; padding: 0 6px; }

.tray { display: flex; flex-direction: column; gap: 6px; position: sticky; top: 0; }
.tray .section-title { margin: 4px 0 2px; }
.hint { margin-top: 4px; }

.ghost {
  position: fixed; z-index: 50; pointer-events: none; width: 160px; transform: translate(-50%, -60%) rotate(-2deg);
  background: var(--bg, Canvas); box-shadow: 0 6px 18px oklch(0 0 0 / 18%), inset 0 0 0 1px var(--line);
}

.seg {
  display: inline-flex; gap: 2px; padding: 3px; border-radius: 999px;
  box-shadow: inset 0 1px 0 var(--glass-control-rim), inset 0 0 0 1px var(--line);
}
.opt { padding: 6px 14px; border-radius: 999px; font-size: 0.9rem; color: var(--muted); }
.opt.chosen { color: var(--text); }
</style>
