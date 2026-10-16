<script setup lang="ts">
import { AlarmClock, ChartColumn, Check, PanelRightOpen, Plus, Settings, ShoppingCart, SkipForward, Sunset, Trash2 } from 'lucide-vue-next'
import type { Row } from '~/composables/api'
import type { Section } from '~/composables/format'

const router = useRouter()
const { show } = useToast()
const sound = useSound()
const rows = ref<Row[]>([])
const completing = ref(new Set<string>())
const adding = ref(false)
const now = ref(Date.now())
const error = ref('')

async function refresh() {
  now.value = Date.now()
  try {
    rows.value = await api.listOpen()
    error.value = ''
  } catch (e) {
    error.value = String(e)
  }
}

// Optimistic: the box ticks at once, the row slides out a beat later.
async function complete(row: Row) {
  if (completing.value.has(row.id)) return
  haptic()
  sound.complete()
  completing.value = new Set(completing.value).add(row.id)
  const write = api.completeTask(row.id)
  setTimeout(() => {
    rows.value = rows.value.filter((r) => r.id !== row.id)
    const next = new Set(completing.value)
    next.delete(row.id)
    completing.value = next
  }, 320)
  try {
    await write
    show({
      message: row.rrule ? 'Done · next one scheduled' : 'Completed',
      action: { label: 'Undo', run: () => api.reopenTask(row.id).then(refresh) },
    })
  } catch (e) {
    show({ message: String(e) })
  }
  setTimeout(refresh, 400)
}

// Desktop right-click on a task.
function menu(e: MouseEvent, row: Row) {
  const canSnooze = row.dueAt != null || row.snoozedUntil != null
  openMenu(e, [
    { label: 'Open', icon: PanelRightOpen, run: () => router.push(`/task/${row.id}`) },
    { label: 'Complete', icon: Check, run: () => complete(row) },
    ...(canSnooze ? [{ label: 'Snooze…', icon: AlarmClock, run: () => router.push(`/snooze?id=${row.id}`) }] : []),
    ...(row.rrule ? [{ label: 'Skip this one', icon: SkipForward, run: () => skip(row) }] : []),
    ...(row.rrule ? [{ label: 'Retire', icon: Sunset, run: () => retire(row) }] : []),
    'separator',
    { label: 'Delete', icon: Trash2, danger: true, run: () => remove(row) },
  ])
}

async function skip(row: Row) {
  try {
    const t = await api.skipTask(row.id)
    show({
      message: `Skipped · next ${dayLabel(t.dueAt!)}`,
      action: { label: 'Undo', run: () => api.updateTask(row.id, { dueAt: row.dueAt }).then(refresh) },
    })
  } catch (e) {
    show({ message: String(e) })
  }
  refresh()
}

// Repeating task done for good: off the list, history kept.
async function retire(row: Row) {
  haptic()
  rows.value = rows.value.filter((r) => r.id !== row.id)
  try {
    await api.retireTask(row.id)
    show({ message: 'Retired · history kept', action: { label: 'Undo', run: () => api.reopenTask(row.id).then(refresh) } })
  } catch (e) {
    show({ message: String(e) })
    refresh()
  }
}

async function remove(row: Row) {
  rows.value = rows.value.filter((r) => r.id !== row.id)
  try {
    await api.deleteTask(row.id)
    show({ message: 'Deleted', action: { label: 'Undo', run: () => api.restoreTask(row.id).then(refresh) } })
  } catch (e) {
    show({ message: String(e) })
    refresh()
  }
}

// Overdue / Today / Tomorrow / This week / This month / Later / No date.
const sections = computed(() => {
  const by = new Map<Section, Row[]>()
  for (const r of rows.value) {
    const s = sectionOf(r.snoozedUntil ?? r.dueAt, now.value)
    by.set(s, [...(by.get(s) ?? []), r])
  }
  return (Object.keys(sectionTitles) as Section[]).filter((s) => by.has(s)).map((s) => ({ id: s, rows: by.get(s)! }))
})

watch(listVersion, refresh)
// "Share → Befikr" (Android) opens the list with the shared text in the query.
const route = useRoute()
watch(
  () => route.query.shareText,
  (text) => {
    if (typeof text !== 'string') return
    draftFromShare(text, String(route.query.shareSubject ?? ''))
    adding.value = true
    router.replace('/')
  },
  { immediate: true },
)

// Also when the list mounts after the request (opened from another page).
watch(
  quickAddPending,
  (v) => {
    if (!v) return
    adding.value = true
    quickAddPending.value = false
  },
  { immediate: true },
)

let tick: ReturnType<typeof setInterval>
const onVisible = () => document.visibilityState === 'visible' && refresh()
onMounted(() => {
  refresh()
  tick = setInterval(refresh, 30_000)
  document.addEventListener('visibilitychange', onVisible)
})
onUnmounted(() => {
  clearInterval(tick)
  document.removeEventListener('visibilitychange', onVisible)
})
</script>

<template>
  <main class="page">
    <header class="topbar">
      <h1 class="brand">Befikr</h1>
      <SyncDot />
      <span class="grow" />
      <NuxtLink to="/shopping" class="icon-btn" aria-label="Shopping list"><ShoppingCart :size="22" /></NuxtLink>
      <NuxtLink to="/stats" class="icon-btn" aria-label="Stats"><ChartColumn :size="22" /></NuxtLink>
      <NuxtLink to="/settings" class="icon-btn" aria-label="Settings"><Settings :size="22" /></NuxtLink>
    </header>

    <p v-if="error" class="muted">{{ error }}</p>

    <section v-for="s in sections" :key="s.id" class="section">
      <h2 class="section-title" :class="s.id">{{ sectionTitles[s.id] }}</h2>
      <TransitionGroup tag="ul" name="list" class="list pane rows">
        <TaskRow
          v-for="r in s.rows"
          :key="r.id"
          :row="r"
          :now="now"
          :time-only="s.id === 'today' || s.id === 'tomorrow'"
          :done="completing.has(r.id)"
          @complete="complete(r)"
          @open="router.push(`/task/${r.id}`)"
          @snooze="router.push(`/snooze?id=${r.id}`)"
          @menu="menu($event, r)"
        />
      </TransitionGroup>
    </section>
    <div v-if="!rows.length && !error" class="empty">
      <p>Nothing to do.</p>
      <p class="muted">Tap + and type something like “call mom tomorrow 6pm”.</p>
    </div>

    <button v-if="!adding" class="fab ink" aria-label="Add task" @click="adding = true"><Plus :size="28" /></button>
    <QuickAdd v-if="adding" @close="adding = false" @saved="refresh" />
  </main>
</template>

<style scoped>
/* Title doesn't stretch, so the sync dot sits right next to it. */
.topbar h1.brand { flex: none; }
.grow { flex: 1; }
.list { margin: 8px 0 0; position: relative; overflow: hidden; }
.section .section-title { margin: 20px 0 0; }
.section:first-of-type .section-title { margin-top: 4px; }
.section-title.overdue { color: var(--red); }
.list-leave-active { transition: opacity 0.25s ease; position: absolute; left: 14px; right: 14px; }
.list-leave-to { opacity: 0; }
.list-move { transition: transform 0.3s ease; }
.empty { text-align: center; margin-top: 30vh; }
.fab {
  position: fixed; right: 20px; bottom: calc(24px + env(safe-area-inset-bottom)); z-index: 30;
  width: 64px; height: 64px; border-radius: 20px; display: grid; place-items: center;
}
</style>
