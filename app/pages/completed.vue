<script setup lang="ts">
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
      <button class="icon-btn" aria-label="Back" @click="router.back()">←</button>
      <h1>Completed</h1>
    </header>
    <ul class="list">
      <li v-for="c in items" :key="c.id">
        <span>{{ c.title }}</span>
        <span class="muted small">
          {{ dayLabel(c.completedAt) }}<template v-if="late(c)"> · was due {{ dayLabel(c.dueAtThen!) }}</template>
        </span>
      </li>
    </ul>
    <p v-if="!items.length" class="muted">Nothing completed yet.</p>
  </main>
</template>

<style scoped>
.list { list-style: none; padding: 0; }
.list li { display: flex; flex-direction: column; padding: 10px 0; border-bottom: 1px solid var(--line); }
.small { font-size: 0.85rem; }
</style>
