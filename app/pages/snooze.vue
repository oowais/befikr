<script setup lang="ts">
// Opened from the notification's Snooze button (or the task view).
import type { Task } from '~/composables/api'

const route = useRoute()
const router = useRouter()
const { show } = useToast()
const id = computed(() => String(route.query.id ?? ''))
const task = ref<Task | null>(null)
const custom = ref('')
const typed = ref('')
const error = ref('')

onMounted(async () => {
  task.value = await api.getTask(id.value).catch(() => null)
})

const HOUR = 3_600_000
function at(daysAhead: number, h: number, m = 0) {
  const d = new Date()
  d.setDate(d.getDate() + daysAhead)
  d.setHours(h, m, 0, 0)
  return d.getTime()
}

const picks = computed(() => {
  const now = Date.now()
  const today = new Date()
  const list: { label: string; at: number }[] = [
    { label: 'In 1 hour', at: now + HOUR },
    { label: 'In 3 hours', at: now + 3 * HOUR },
  ]
  if (today.getHours() < 18) list.push({ label: 'This evening', at: at(0, 18) })
  if (today.getHours() < 21) list.push({ label: 'Tonight', at: at(0, 21) })
  list.push({ label: 'Tomorrow morning', at: at(1, 9) })
  list.push({ label: 'Tomorrow evening', at: at(1, 18) })
  const toMonday = ((8 - today.getDay()) % 7) || 7
  list.push({ label: 'Next week', at: at(toMonday, 9) })
  return list
})

const typedParse = computed(() => (typed.value.trim() ? parseWith(`x ${typed.value}`, []) : null))
const typedAt = computed(() => {
  const d = typedParse.value?.due
  return d ? localMs(d.date, d.time ?? '09:00') : null
})

async function snooze(until: number | null) {
  if (until == null) return
  if (until <= Date.now()) return (error.value = 'Pick a time in the future')
  try {
    await api.snoozeTask(id.value, until)
    haptic()
    show({ message: `Snoozed to ${dayLabel(until)}` })
    router.replace('/')
  } catch (e) {
    error.value = String(e)
  }
}
</script>

<template>
  <main class="page">
    <header class="topbar">
      <button class="icon-btn" aria-label="Close" @click="router.replace('/')">✕</button>
      <h1>Snooze</h1>
    </header>
    <p class="task-title">{{ task?.title }}</p>

    <div class="grid">
      <button v-for="p in picks" :key="p.label" class="pick" @click="snooze(p.at)">
        <span>{{ p.label }}</span>
        <span class="muted">{{ dayLabel(p.at) }}</span>
      </button>
    </div>

    <h2 class="section-title">Or type it</h2>
    <form class="row" @submit.prevent="snooze(typedAt)">
      <input v-model="typed" class="field" placeholder="fri 3pm" enterkeyhint="done" />
      <button class="btn primary" :disabled="!typedAt">Go</button>
    </form>
    <p v-if="typedAt" class="muted">{{ dayLabel(typedAt) }}</p>

    <h2 class="section-title">Pick a time</h2>
    <form class="row" @submit.prevent="snooze(fromInputValue(custom))">
      <input v-model="custom" type="datetime-local" class="field" />
      <button class="btn primary" :disabled="!custom">Go</button>
    </form>

    <p v-if="error" class="error">{{ error }}</p>
  </main>
</template>

<style scoped>
.task-title { font-size: 1.15rem; font-weight: 600; margin: 4px 0 16px; }
.grid { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
.pick {
  display: flex; flex-direction: column; align-items: flex-start; gap: 2px; text-align: left;
  padding: 14px; border-radius: var(--radius); background: var(--surface); border: 1px solid var(--line);
}
.pick:active { background: var(--chip); }
.pick .muted { font-size: 0.85rem; }
.row { display: flex; gap: 8px; }
.error { color: var(--red); }
</style>
