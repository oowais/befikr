<script setup lang="ts">
import { ArrowLeft, Check } from 'lucide-vue-next'
import type { Completion } from '~/composables/api'

const router = useRouter()
const items = ref<Completion[]>([])
onMounted(async () => {
  items.value = await api.listCompleted(300).catch(() => [])
})
const late = (c: Completion) => c.dueAtThen != null && c.completedAt > c.dueAtThen + 60_000
</script>

<template>
  <main class="page">
    <header class="topbar">
      <button class="icon-btn" aria-label="Back" @click="router.back()"><ArrowLeft :size="22" /></button>
      <h1>Completed</h1>
    </header>
    <ul v-if="items.length" class="pane rows list">
      <li v-for="c in items" :key="c.id">
        <span class="checkbox checked"><Check :size="14" :stroke-width="3" /></span>
        <span class="text">
          <span>{{ c.title }}</span>
          <span class="muted small">
            {{ dayLabel(c.completedAt) }}<template v-if="late(c)"> · was due {{ dayLabel(c.dueAtThen!) }}</template>
          </span>
        </span>
      </li>
    </ul>
    <p v-else class="muted">Nothing completed yet.</p>
  </main>
</template>

<style scoped>
.list { list-style: none; margin: 8px 0 0; }
.list li { display: flex; align-items: flex-start; gap: 12px; padding: 12px 0; }
.text { display: flex; flex-direction: column; gap: 2px; }
</style>
