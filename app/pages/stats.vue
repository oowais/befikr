<script setup lang="ts">
// Stats on top (range switch, last 8 weeks chart), completion history below,
// one row per task ("Water plants · 58×"). Tap a row to open the task and
// its history; tap the tick to undo its latest completion (or bring back a
// retired task).
import { Check, Info, PanelRightOpen, RotateCcw, Sprout, Sunset } from 'lucide-vue-next'
import type { CompletedGroup, Stats, WeekStats } from '~/composables/api'

const DAY = 86_400_000
const ranges = [
  { id: 7, label: '7 days' },
  { id: 30, label: '30 days' },
  { id: 0, label: 'All time' },
] as const

const { open: openTask } = useTaskSheet()
const { show } = useToast()
const items = ref<CompletedGroup[]>([])
const stats = ref<Stats | null>(null)
const range = ref<number>(30)
const carefree = useCarefree()
// The carefree card opens how it works.
const explainingCarefree = ref(false)

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
  carefree.refresh()
  items.value = await api.completedGroups().catch(() => [])
  await loadStats()
}
onMounted(load)
watch(listVersion, load)
watch(range, loadStats)

const dated = computed(() => (stats.value ? stats.value.onTime + stats.value.late : 0))
const onTimePct = computed(() => (dated.value ? Math.round((stats.value!.onTime / dated.value) * 100) : null))
const total = (w: WeekStats) => w.onTime + w.late + w.undated
const weekMax = computed(() => Math.max(1, ...(stats.value?.weeks ?? []).map(total)))
const weekLabel = (ms: number) => new Date(ms).toLocaleDateString(undefined, { day: 'numeric', month: 'short' })

// Late = done on a later day than it was due (same rule as the stats).
const dayOf = (ms: number) => new Date(ms).toDateString()
const late = (g: CompletedGroup) =>
  !g.retired && g.dueAtThen != null && g.lastAt > g.dueAtThen && dayOf(g.lastAt) !== dayOf(g.dueAtThen)

function rowMenu(e: MouseEvent, g: CompletedGroup) {
  openMenu(e, [
    { label: 'Open', icon: PanelRightOpen, run: () => openTask(g.taskId) },
    { label: g.retired ? 'Bring back' : 'Undo completion', icon: RotateCcw, run: () => undo(g) },
  ])
}

async function undo(g: CompletedGroup) {
  haptic()
  try {
    await api.reopenTask(g.taskId)
    show({ message: g.retired ? 'Brought back' : 'Back in to-do' })
  } catch (e) {
    show({ message: friendlyError(e, 'undo it') })
  }
  load()
}
</script>

<template>
  <Screen title="Stats">

    <button
      v-if="carefree.enabled.value && carefree.data.value"
      type="button"
      class="pane carefree"
      aria-label="How do carefree days work?"
      @click="explainingCarefree = true"
    >
      <Sprout :size="22" class="muted" />
      <div class="flex">
        <p class="cf-num">
          {{ carefree.data.value.streak }} carefree {{ carefree.data.value.streak === 1 ? 'day' : 'days' }}
          <span v-if="carefree.data.value.best > carefree.data.value.streak" class="muted small">· best {{ carefree.data.value.best }}</span>
        </p>
        <p class="muted small">Days that end with nothing overdue (done by noon the next day is fine). One missed day a week is forgiven.</p>
      </div>
      <Info :size="18" class="muted" />
    </button>
    <CarefreeInfo v-if="explainingCarefree" :garden="carefree.data.value" @close="explainingCarefree = false" />

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
      <li v-for="g in items" :key="g.taskId" @contextmenu="rowMenu($event, g)">
        <button class="check" :aria-label="g.retired ? `Bring back ${g.title}` : `Undo ${g.title}`" @click="undo(g)">
          <Sunset v-if="g.retired" :size="20" class="muted" />
          <span v-else class="checkbox checked"><Check :size="14" :stroke-width="3" /></span>
        </button>
        <button class="text" @click="openTask(g.taskId)">
          <span>{{ g.title }}<span v-if="g.count > 1" class="muted times"> · {{ g.count }}×</span></span>
          <span class="muted small">
            <template v-if="g.retired">Retired {{ dayLabel(g.lastAt) }}</template>
            <template v-else>{{ g.count > 1 ? 'Last: ' : '' }}{{ dayLabel(g.lastAt) }}<template v-if="late(g)"> · {{ span(g.lastAt - g.dueAtThen!) }} late</template></template>
          </span>
        </button>
      </li>
    </ul>
    <p v-else class="muted">Nothing completed yet. Stats show up here once you finish tasks.</p>
  </Screen>
</template>

<style scoped>
.seg {
  display: inline-flex; gap: 2px; padding: 3px; border-radius: 999px; margin-top: 4px;
  box-shadow: inset 0 1px 0 var(--glass-control-rim), inset 0 0 0 1px var(--line);
}
.opt { padding: 5px 12px; border-radius: 999px; font-size: 0.9rem; color: var(--muted); white-space: nowrap; }
.opt.chosen { color: var(--text); }

.carefree { display: flex; gap: 12px; align-items: flex-start; width: 100%; margin-bottom: 16px; padding: 14px 16px; text-align: left; color: inherit; }
.carefree p { margin: 0; }
.carefree .flex { flex: 1; display: flex; flex-direction: column; gap: 4px; }
.cf-num { font-weight: 600; font-size: 1.05rem; }
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
.list li { display: flex; align-items: flex-start; gap: 2px; padding: 6px 0; user-select: none; -webkit-user-select: none; -webkit-touch-callout: none; }
.check { width: 44px; height: 44px; display: grid; place-items: center; flex: none; margin-left: -10px; }
.times { font-variant-numeric: tabular-nums; }
.text { flex: 1; display: flex; flex-direction: column; gap: 2px; padding: 6px 0; color: inherit; text-decoration: none; min-width: 0; text-align: left; }
</style>
