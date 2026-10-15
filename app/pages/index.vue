<script setup lang="ts">
import { ListChecks, Plus, Settings } from 'lucide-vue-next'
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
      <h1>Befikr</h1>
      <NuxtLink to="/completed" class="icon-btn" aria-label="Completed"><ListChecks :size="22" /></NuxtLink>
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
