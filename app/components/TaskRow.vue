<script setup lang="ts">
import { Check, ListChecks, Moon, Repeat } from 'lucide-vue-next'
import type { Row } from '~/composables/api'

const props = defineProps<{ row: Row; now: number; done?: boolean }>()
const emit = defineEmits<{ complete: []; open: [] }>()

const label = computed(() => dueLabel(props.row.snoozedUntil ?? props.row.dueAt, props.now))
const big = computed(() => props.row.urgency === 'red' || props.row.urgency === 'yellow')
</script>

<template>
  <li class="row" :class="[row.urgency, { big }]">
    <button class="check" :aria-label="`Complete ${row.title}`" @click="emit('complete')">
      <span class="checkbox" :class="{ checked: done }"><Check :size="15" :stroke-width="3" /></span>
    </button>
    <button class="body" @click="emit('open')">
      <span class="title" :class="{ struck: done }">{{ row.title }}</span>
      <span v-if="label || row.rrule || row.subtasksTotal" class="meta">
        <span v-if="label" class="due"><span class="dot" :class="row.urgency" />{{ label }}</span>
        <Moon v-if="row.snoozedUntil" :size="14" aria-label="Snoozed" />
        <Repeat v-if="row.rrule" :size="14" aria-label="Repeats" />
        <span v-if="row.subtasksTotal" class="subs"><ListChecks :size="14" />{{ row.subtasksDone }}/{{ row.subtasksTotal }}</span>
      </span>
    </button>
  </li>
</template>

<style scoped>
.row { display: flex; align-items: flex-start; gap: 2px; padding: 8px 0; list-style: none; }
.check { width: 44px; height: 44px; display: grid; place-items: center; flex: none; margin: -6px 0 -6px -10px; }
.body { flex: 1; text-align: left; display: flex; flex-direction: column; gap: 3px; min-width: 0; padding: 2px 0; }
.title { overflow-wrap: anywhere; transition: color 0.2s; }
.title.struck { color: var(--muted); text-decoration: line-through; }
.big .title { font-weight: 600; }
.meta { display: flex; align-items: center; gap: 10px; font-size: 0.85rem; color: var(--muted); }
.due, .subs { display: inline-flex; align-items: center; gap: 6px; }
.subs { gap: 4px; }
.red .due { color: var(--red); font-weight: 500; }
.yellow .due { color: var(--text); }
</style>
