<script setup lang="ts">
// Stats on top (range switch, last 8 weeks chart), completion history below.
// Tap a row to open the task; tap the tick to undo (only the latest
// completion of a task can be undone).
import { ArrowLeft, Check } from 'lucide-vue-next'
import type { Completion, Stats, WeekStats } from '~/composables/api'

const DAY = 86_400_000
const ranges = [
  { id: 7, label: '7 days' },
  { id: 30, label: '30 days' },
  { id: 0, label: 'All time' },
] as const

const router = useRouter()
const { show } = useToast()
const items = ref<Completion[]>([])
const stats = ref<Stats | null>(null)
const range = ref<number>(30)

/** Local Monday 00:00 of this week and the 7 before it, oldest first. */
function weekStarts(): number[] {
  const d = new Date()
  d.setHours(0, 0, 0, 0)
  d.setDate(d.getDate() - ((d.getDay() + 6) % 7))
  return Array.from({ length: 8 }, (_, i) => new Date(d.getFullYear(), d.getMonth(), d.getDate() - 7 * (7 - i)).getTime())
}

const loadStats = async () =>
  (stats.value = await api.stats(range.value ? Date.now() - range.value * DAY : null, weekStarts()).catch(() => null))
const load = async () => {
  items.value = await api.listCompleted(300).catch(() => [])
  await loadStats()
}
onMounted(load)
watch(range, loadStats)

const dated = computed(() => (stats.value ? stats.value.onTime + stats.value.late : 0))
const onTimePct = computed(() => (dated.value ? Math.round((stats.value!.onTime / dated.value) * 100) : null))
const total = (w: WeekStats) => w.onTime + w.late + w.undated
const weekMax = computed(() => Math.max(1, ...(stats.value?.weeks ?? []).map(total)))
const weekLabel = (ms: number) => new Date(ms).toLocaleDateString(undefined, { day: 'numeric', month: 'short' })

const late = (c: Completion) => c.dueAtThen != null && c.completedAt > c.dueAtThen + 60_000
// Newest first, so a task's first row is its latest completion.
const undoable = computed(() => {
  const seen = new Set<string>()
  return new Set(items.value.filter((c) => !seen.has(c.taskId) && seen.add(c.taskId)).map((c) => c.id))
})

async function undo(c: Completion) {
  haptic()
  items.value = items.value.filter((x) => x.id !== c.id)
  try {
    await api.reopenTask(c.taskId)
    show({ message: 'Back in to-do' })
    loadStats()
  } catch (e) {
    show({ message: String(e) })
    load()
  }
}
</script>

<template>
  <main class="page">
    <header class="topbar">
      <button class="icon-btn" aria-label="Back" @click="router.back()"><ArrowLeft :size="22" /></button>
      <h1>Stats</h1>
    </header>

    <template v-if="items.length && stats">
      <div class="seg" role="radiogroup" aria-label="Range">
        <button
          v-for="r in ranges"
          :key="r.id"
          type="button"
          role="radio"
          class="opt"
          :class="{ chosen: range === r.id }"
          :aria-checked="range === r.id"
          @click="range = r.id"
        >
          {{ r.label }}
        </button>
      </div>

      <div class="pane tiles">
        <div class="tile">
          <span class="num">{{ stats.completed }}</span>
          <span class="muted small">done</span>
        </div>
        <div class="tile">
          <span class="num">{{ onTimePct == null ? '–' : `${onTimePct}%` }}</span>
          <span class="muted small">on time<template v-if="dated"> ({{ stats.onTime }} of {{ dated }})</template></span>
        </div>
        <div class="tile">
          <span class="num">{{ stats.lateMedianMs == null ? '–' : span(stats.lateMedianMs) }}</span>
          <span class="muted small">usually late by</span>
        </div>
        <div class="tile">
          <span class="num">{{ stats.snoozes }}</span>
          <span class="muted small">snoozes<template v-if="stats.snoozedTasks"> on {{ stats.snoozedTasks }} {{ stats.snoozedTasks === 1 ? 'task' : 'tasks' }}</template></span>
        </div>
        <div v-if="stats.missed" class="tile">
          <span class="num">{{ stats.missed }}</span>
          <span class="muted small">repeats missed</span>
        </div>
      </div>

      <h2 class="section-title">Last 8 weeks</h2>
      <div class="pane chart">
        <div class="bars">
          <div v-for="w in stats.weeks" :key="w.start" class="col" :aria-label="`Week of ${weekLabel(w.start)}: ${w.onTime} on time, ${w.late} late, ${w.undated} no date`">
            <span v-if="w.onTime" class="bar on-time" :style="{ height: `${(w.onTime / weekMax) * 100}%` }" />
            <span v-if="w.late" class="bar late" :style="{ height: `${(w.late / weekMax) * 100}%` }" />
            <span v-if="w.undated" class="bar undated" :style="{ height: `${(w.undated / weekMax) * 100}%` }" />
            <span v-if="total(w)" class="muted count">{{ total(w) }}</span>
          </div>
        </div>
        <div class="labels muted">
          <span v-for="w in stats.weeks" :key="w.start">{{ weekLabel(w.start) }}</span>
        </div>
        <div class="legend muted small">
          <span><i class="on-time" />On time</span>
          <span><i class="late" />Late</span>
          <span><i class="undated" />No date</span>
        </div>
      </div>

      <h2 class="section-title">Completed</h2>
    </template>

    <ul v-if="items.length" class="pane rows list">
      <li v-for="c in items" :key="c.id">
        <button
          class="check"
          :disabled="!undoable.has(c.id)"
          :aria-label="`Undo ${c.title}`"
          @click="undo(c)"
        >
          <span class="checkbox checked"><Check :size="14" :stroke-width="3" /></span>
        </button>
        <NuxtLink :to="`/task/${c.taskId}`" class="text">
          <span>{{ c.title }}</span>
          <span class="muted small">
            {{ dayLabel(c.completedAt) }}<template v-if="late(c)"> · {{ span(c.completedAt - c.dueAtThen!) }} late</template>
          </span>
        </NuxtLink>
      </li>
    </ul>
    <p v-else class="muted">Nothing completed yet. Stats show up here once you finish tasks.</p>
  </main>
</template>

<style scoped>
.seg {
  display: inline-flex; gap: 2px; padding: 3px; border-radius: 999px; margin-top: 4px;
  box-shadow: inset 0 1px 0 var(--glass-control-rim), inset 0 0 0 1px var(--line);
}
.opt { padding: 5px 12px; border-radius: 999px; font-size: 0.9rem; color: var(--muted); white-space: nowrap; }
.opt.chosen { color: var(--text); }

.tiles { display: grid; grid-template-columns: 1fr 1fr; gap: 16px 12px; margin-top: 12px; padding: 16px; }
.tile { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.num { font-size: 1.6rem; font-weight: 600; letter-spacing: -0.02em; font-variant-numeric: tabular-nums; }

.chart { padding: 28px 12px 12px; }
.bars { display: grid; grid-template-columns: repeat(8, 1fr); gap: 6px; height: 120px; }
/* Bars stack bottom-up; the count sits on top of the stack. */
.col { display: flex; flex-direction: column-reverse; gap: 2px; height: 100%; }
.bar { display: block; border-radius: 3px; flex: none; }
.count { text-align: center; font-size: 0.7rem; font-variant-numeric: tabular-nums; line-height: 1.4; }
.labels { display: grid; grid-template-columns: repeat(8, 1fr); gap: 6px; margin-top: 6px; font-size: 0.65rem; text-align: center; white-space: nowrap; }
.legend { display: flex; gap: 14px; margin-top: 12px; }
.legend span { display: inline-flex; align-items: center; gap: 6px; }
.legend i { width: 10px; height: 10px; border-radius: 3px; }
.on-time { background: var(--green); }
.late { background: var(--yellow); }
.undated { background: var(--gray); }

.list { list-style: none; margin: 8px 0 0; }
.list li { display: flex; align-items: flex-start; gap: 2px; padding: 6px 0; }
.check { width: 44px; height: 44px; display: grid; place-items: center; flex: none; margin-left: -10px; }
.check:disabled { opacity: 0.45; }
.text { flex: 1; display: flex; flex-direction: column; gap: 2px; padding: 6px 0; color: inherit; text-decoration: none; min-width: 0; }
</style>
