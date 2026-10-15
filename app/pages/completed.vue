<script setup lang="ts">
// Completion history. Tap a row to open the task; tap the tick to undo
// (only the latest completion of a task can be undone).
import { ArrowLeft, Check } from 'lucide-vue-next'
import type { Completion } from '~/composables/api'

const router = useRouter()
const { show } = useToast()
const items = ref<Completion[]>([])
const load = async () => (items.value = await api.listCompleted(300).catch(() => []))
onMounted(load)

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
      <h1>Completed</h1>
    </header>
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
            {{ dayLabel(c.completedAt) }}<template v-if="late(c)"> · was due {{ dayLabel(c.dueAtThen!) }}</template>
          </span>
        </NuxtLink>
      </li>
    </ul>
    <p v-else class="muted">Nothing completed yet.</p>
  </main>
</template>

<style scoped>
.list { list-style: none; margin: 8px 0 0; }
.list li { display: flex; align-items: flex-start; gap: 2px; padding: 6px 0; }
.check { width: 44px; height: 44px; display: grid; place-items: center; flex: none; margin-left: -10px; }
.check:disabled { opacity: 0.45; }
.text { flex: 1; display: flex; flex-direction: column; gap: 2px; padding: 6px 0; color: inherit; text-decoration: none; min-width: 0; }
</style>
