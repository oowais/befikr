<script setup lang="ts">
import type { Row } from '~/composables/api'

const props = defineProps<{ row: Row; now: number }>()
const emit = defineEmits<{ complete: []; open: [] }>()

const label = computed(() => dueLabel(props.row.snoozedUntil ?? props.row.dueAt, props.now))
const big = computed(() => props.row.urgency === 'red' || props.row.urgency === 'yellow')
</script>

<template>
  <li class="row" :class="[row.urgency, { big }]">
    <button class="check" :aria-label="`Complete ${row.title}`" @click="emit('complete')">
      <span class="ring" />
    </button>
    <button class="body" @click="emit('open')">
      <span class="title">{{ row.title }}</span>
      <span v-if="label || row.rrule || row.subtasksTotal" class="meta">
        <span v-if="label" class="due">{{ label }}</span>
        <span v-if="row.snoozedUntil" title="Snoozed">💤</span>
        <span v-if="row.rrule" title="Repeats">↻</span>
        <span v-if="row.subtasksTotal">{{ row.subtasksDone }}/{{ row.subtasksTotal }}</span>
      </span>
    </button>
  </li>
</template>

<style scoped>
.row {
  display: flex; align-items: flex-start; gap: 4px;
  padding: 10px 4px 10px 0; border-bottom: 1px solid var(--line);
  border-left: 4px solid var(--gray); padding-left: 8px; border-radius: 2px;
  list-style: none;
}
.row.red { border-left-color: var(--red); }
.row.yellow { border-left-color: var(--yellow); }
.row.green { border-left-color: var(--green); }
.check { width: 40px; height: 40px; display: grid; place-items: center; flex: none; margin-top: -8px; }
.ring { width: 22px; height: 22px; border-radius: 50%; border: 2px solid var(--muted); transition: background 0.15s; }
.row.red .ring { border-color: var(--red); }
.check:active .ring { background: var(--accent); border-color: var(--accent); }
.body { flex: 1; text-align: left; display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.title { overflow-wrap: anywhere; }
.big .title { font-weight: 600; font-size: 1.05rem; }
.meta { display: flex; gap: 10px; font-size: 0.85rem; color: var(--muted); }
.red .due { color: var(--red); font-weight: 500; }
.yellow .due { color: var(--text); }
</style>
