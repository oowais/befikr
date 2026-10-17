<script setup lang="ts">
import { AlarmClock, CalendarDays, ChartColumn, Check, PanelRightOpen, Plus, Settings, ShoppingCart, SkipForward, Sprout, Sunset, Trash2 } from 'lucide-vue-next'
import type { Row } from '~/composables/api'
import type { Section } from '~/composables/format'

const router = useRouter()
const { open: openSnooze } = useSnoozeSheet()
const { open: openTask } = useTaskSheet()
const { show } = useToast()
const update = useUpdate()
const carefree = useCarefree()
const garden = computed(() => (carefree.enabled.value ? carefree.data.value : null))
const sound = useSound()
const rows = ref<Row[]>([])
const completing = ref(new Set<string>())
const adding = ref(false)
const now = ref(Date.now())
const error = ref('')
// Nothing below the header until the first load: otherwise the empty
// "All clear" scene flashes for a moment on a cold start.
const loaded = ref(false)
const releaseSplash = holdSplash()
const milestones = useMilestones()
// Most overdue tasks seen at once since the list was last clear of them.
let overduePeak = 0
// The last task was just completed: the empty scene greets it (see Doodle).
const celebrate = ref(false)
const hint = useHint()
useHintRotation()

async function refresh() {
  now.value = Date.now()
  try {
    // Garden numbers with the list, so trees and tasks arrive together.
    const [list] = await Promise.all([api.listOpen(), carefree.refresh()])
    rows.value = list
    error.value = ''
    if (carefree.enabled.value) {
      milestones.check(carefree.data.value)
      const overdue = list.filter((r) => r.urgency === 'red').length
      if (!overdue && overduePeak >= 3) milestones.comeback()
      overduePeak = overdue ? Math.max(overduePeak, overdue) : 0
    }
  } catch (e) {
    error.value = String(e)
  }
  loaded.value = true
  releaseSplash()
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
    if (!rows.value.length) allClear()
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

// The list just emptied by a completion: a chord, a longer buzz, the sun
// (or moon) rises over the empty landscape.
function allClear() {
  celebrate.value = true
  sound.allClear()
  hapticCelebrate()
}
watch(() => rows.value.length, (n) => n && (celebrate.value = false))

// Desktop right-click on a task.
function menu(e: MouseEvent, row: Row) {
  const canSnooze = row.dueAt != null || row.snoozedUntil != null
  openMenu(e, [
    { label: 'Open', icon: PanelRightOpen, run: () => openTask(row.id) },
    { label: 'Complete', icon: Check, run: () => complete(row) },
    ...(canSnooze ? [{ label: 'Snooze…', icon: AlarmClock, run: () => openSnooze(row.id) }] : []),
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

// Widget "New task" (Android) opens the list with ?add=1.
watch(
  () => route.query.add,
  (add) => {
    if (add == null) return
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
  <main class="page home">
    <header class="topbar">
      <h1 class="brand">Befikr</h1>
      <SyncDot />
      <NuxtLink v-if="garden?.streak" to="/stats" class="streak" :aria-label="`${garden.streak} carefree ${garden.streak === 1 ? 'day' : 'days'} in a row`">
        <Sprout :size="17" /> {{ garden.streak }}
      </NuxtLink>
      <span class="grow" />
      <NuxtLink to="/calendar" class="icon-btn wide-only" aria-label="Calendar"><CalendarDays :size="22" /></NuxtLink>
      <NuxtLink to="/shopping" class="icon-btn" aria-label="Shopping list"><ShoppingCart :size="22" /></NuxtLink>
      <NuxtLink to="/stats" class="icon-btn" aria-label="Stats"><ChartColumn :size="22" /></NuxtLink>
      <NuxtLink to="/settings" class="icon-btn gear" :aria-label="update.available.value ? 'Settings (update available)' : 'Settings'">
        <Settings :size="22" /><span v-if="update.available.value" class="update-dot" />
      </NuxtLink>
    </header>

    <p v-if="error" class="muted">{{ error }}</p>

    <div class="stage">
      <WeekRecap v-if="loaded" class="ease" />
      <template v-if="loaded && rows.length">
        <div class="scroll ease">
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
                @open="openTask(r.id)"
                @snooze="openSnooze(r.id)"
                @menu="menu($event, r)"
              />
            </TransitionGroup>
          </section>
        </div>
      </template>
      <Doodle v-else-if="loaded && !error" class="scene ease" clear :garden="garden" :celebrate="celebrate" />
      <Doodle v-if="loaded && rows.length" class="below ease" :garden="garden" />
      <div v-if="!loaded" class="scene" />
      <!-- Adding lives at thumb height: tap to open quick add. -->
      <div class="dock">
        <button class="composer field" title="New task (N)" @click="adding = true">
          <Plus :size="22" /> <span>New task</span>
          <Transition name="hint" mode="out-in"><span :key="hint" class="hint" aria-hidden="true">{{ hint }}</span></Transition>
          <kbd class="key" aria-hidden="true">N</kbd>
        </button>
      </div>
    </div>

    <QuickAdd v-if="adding" @close="adding = false" @saved="refresh" />
  </main>
</template>

<style scoped>
/* Calendar: on wide windows only (computers, tablets in landscape). */
.wide-only { display: none; }
@media (min-width: 900px) { .wide-only { display: inline-flex; } }
/* Title doesn't stretch, so the sync dot sits right next to it. */
.topbar h1.brand { flex: none; }
.gear { position: relative; }
.streak {
  display: inline-flex; align-items: center; gap: 3px; margin-left: 6px; padding: 2px 8px 2px 6px;
  border-radius: 999px; font-size: 0.9rem; font-weight: 500; color: var(--muted); text-decoration: none;
}
.update-dot { position: absolute; top: 9px; right: 9px; width: 8px; height: 8px; border-radius: 50%; background: var(--accent); }
.grow { flex: 1; }
.list { margin: 8px 0 0; position: relative; overflow: hidden; }
.section .section-title { margin: 20px 0 0; }
.section:first-of-type .section-title { margin-top: 4px; }
.section-title.overdue { color: var(--red); }
.list-leave-active { transition: opacity 0.25s ease; position: absolute; left: 14px; right: 14px; }
.list-leave-to { opacity: 0; }
.list-move { transition: transform 0.3s ease; }

/* One screen tall: the list scrolls inside, the doodle takes whatever room
   is left above the "New task" bar (none once the list needs it), and with
   nothing to do the doodle is the whole screen. */
/* Pinned between the status and gesture bars: its size comes from the
   window edges (no height arithmetic to round wrong), and a fixed element
   can't make the page taller, so nothing but the list can ever scroll. */
.page.home {
  position: fixed; top: env(safe-area-inset-top); bottom: env(safe-area-inset-bottom); left: 0; right: 0;
  display: flex; flex-direction: column; padding-bottom: 0;
}
/* Fade at the bottom of the list says "more below". */
.scroll { mask-image: linear-gradient(to top, transparent 0, #000 28px); }
.stage { flex: 1; min-height: 0; display: flex; flex-direction: column; }
.scroll { flex: 0 1 auto; min-height: 0; overflow-y: auto; overscroll-behavior: contain; padding-bottom: 10px; scrollbar-width: none; }
.below { flex: 1.6 1 0; min-height: 0; margin: 0 -16px; mask-image: linear-gradient(to bottom, transparent 0, #000 56px); }
.dock { flex: none; padding: 10px 0 14px; }
.scene { flex: 1; min-height: 0; }
/* First appearance: list and landscape rise in gently (the garden then grows, see Doodle). */
.ease { animation: ease-in 0.42s cubic-bezier(0.22, 1, 0.36, 1) both; }
.below.ease { animation-delay: 0.08s; }
@keyframes ease-in { from { opacity: 0; transform: translateY(10px); } }
.composer {
  flex: none; display: flex; align-items: center; gap: 12px; min-height: 60px; text-align: left;
  font-size: 1.05rem; color: var(--muted); border-radius: 18px; cursor: text;
}
/* Example phrase after "New task": faint, one line, cut off rather than wrapped. */
.hint {
  flex: 1; min-width: 0; overflow: hidden; white-space: nowrap; text-overflow: ellipsis;
  font-size: 0.9rem; font-style: italic; opacity: 0.5; text-align: right;
}
/* Keyboard shortcut (app.vue): shown where there's a mouse and keyboard. */
.key {
  display: none; flex: none; min-width: 24px; height: 24px; padding: 0 6px; border-radius: 6px;
  place-items: center; font: 500 0.78rem/1 inherit; font-style: normal; color: var(--muted);
  box-shadow: inset 0 0 0 1px var(--line), inset 0 -1.5px 0 var(--line);
}
@media (hover: hover) and (pointer: fine) { .key { display: inline-grid; } }
.hint-enter-active, .hint-leave-active { transition: opacity 0.6s ease; }
.hint-enter-from, .hint-leave-to { opacity: 0; }
</style>
