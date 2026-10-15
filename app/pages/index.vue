<script setup lang="ts">
import type { Row } from '~/composables/api'

const router = useRouter()
const { show } = useToast()
const rows = ref<Row[]>([])
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

// Optimistic: the row leaves at once; the write happens behind it.
async function complete(row: Row) {
  haptic()
  rows.value = rows.value.filter((r) => r.id !== row.id)
  try {
    await api.completeTask(row.id)
    show({
      message: row.rrule ? 'Done · next one scheduled' : 'Completed',
      action: { label: 'Undo', run: () => api.reopenTask(row.id).then(refresh) },
    })
  } catch (e) {
    show({ message: String(e) })
  }
  refresh()
}

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
      <NuxtLink to="/completed" class="icon-btn" aria-label="Completed">✓</NuxtLink>
      <NuxtLink to="/settings" class="icon-btn" aria-label="Settings">⚙</NuxtLink>
    </header>

    <p v-if="error" class="muted">{{ error }}</p>

    <TransitionGroup v-if="rows.length" tag="ul" name="list" class="list">
      <TaskRow
        v-for="r in rows"
        :key="r.id"
        :row="r"
        :now="now"
        @complete="complete(r)"
        @open="router.push(`/task/${r.id}`)"
      />
    </TransitionGroup>
    <div v-else-if="!error" class="empty">
      <p>Nothing to do.</p>
      <p class="muted">Tap + and type something like “call mom tomorrow 6pm”.</p>
    </div>

    <button v-if="!adding" class="fab" aria-label="Add task" @click="adding = true">+</button>
    <QuickAdd v-if="adding" @close="adding = false" @saved="refresh" />
  </main>
</template>

<style scoped>
.list { padding: 0; margin: 8px 0 0; position: relative; }
.list-leave-active { transition: transform 0.3s ease, opacity 0.3s ease; position: absolute; width: 100%; }
.list-leave-to { transform: translateX(40px); opacity: 0; }
.list-move { transition: transform 0.3s ease; }
.empty { text-align: center; margin-top: 30vh; }
.fab {
  position: fixed; right: 20px; bottom: calc(24px + env(safe-area-inset-bottom)); z-index: 30;
  width: 64px; height: 64px; border-radius: 22px; background: var(--accent); color: #fff;
  font-size: 2rem; line-height: 1; box-shadow: var(--shadow);
}
.fab:active { transform: scale(0.94); }
</style>
