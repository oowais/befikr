<script setup lang="ts">
// Snooze a task: a bottom sheet over the list or task view (opened with
// `?snooze=<id>`, see useQuerySheet). Quick picks, a date & time picker,
// typing ("fri 3pm"), and Skip for repeating tasks. Closes by dragging it
// down, tapping outside, Back, or once a choice is made.
import { CalendarDays, Keyboard, SkipForward } from 'lucide-vue-next'
import type { Task } from '~/composables/api'

const props = defineProps<{ taskId: string }>()
const emit = defineEmits<{ close: []; done: [] }>()
useBackClose(() => emit('close'))

const { show } = useToast()
const task = ref<Task | null>(null)
const picking = ref(false)
const typing = ref(false)
const typed = ref('')
const error = ref('')

onMounted(async () => {
  task.value = await api.getTask(props.taskId).catch(() => null)
})

// Quick picks come from core, the same ones the Android overlay offers.
const picks = ref<{ label: string; at: number }[]>([])
onMounted(async () => {
  picks.value = (await api.snoozePicks().catch(() => [])).map((p) => ({ label: p.label, at: p.atMs }))
})

const typedParse = computed(() => (typed.value.trim() ? parseWith(`x ${typed.value}`, []) : null))
const typedAt = computed(() => {
  const d = typedParse.value?.due
  return d ? localMs(d.date, d.time ?? '09:00') : null
})

// The label already names the day ("Tomorrow morning"): show only the time.
const pickSub = (p: { label: string; at: number }) =>
  isToday(p.at) || p.label.startsWith('Tomorrow') ? timeLabel(p.at) : dayLabel(p.at)

async function snooze(until: number | null) {
  if (until == null) return
  if (until <= Date.now()) return (error.value = 'Pick a time in the future')
  try {
    await api.snoozeTask(props.taskId, until)
    haptic()
    show({ message: `Snoozed to ${dayLabel(until)}` })
    listChanged()
    emit('done')
  } catch (e) {
    error.value = friendlyError(e, 'snooze')
  }
}

async function skip() {
  const due = task.value?.dueAt ?? null
  try {
    const t = await api.skipTask(props.taskId)
    haptic()
    show({
      message: `Skipped · next ${dayLabel(t.dueAt!)}`,
      action: { label: 'Undo', run: () => api.updateTask(t.id, { dueAt: due }).then(listChanged) },
    })
    listChanged()
    emit('done')
  } catch (e) {
    error.value = friendlyError(e, 'move the task')
  }
}

const drag = useSheetDrag(() => emit('close'))
</script>

<template>
  <div class="backdrop" @click.self="emit('close')">
    <div
      class="sheet popover"
      role="dialog"
      aria-label="Snooze"
      :class="{ dragging: drag.dragging.value }"
      :style="drag.style.value"
      v-on="drag.handlers"
    >
      <div class="grab">
        <span class="handle" />
        <span class="title">{{ task?.title ?? '' }}</span>
        <span class="muted small">Snooze until…</span>
      </div>

      <div class="grid">
        <button v-for="p in picks" :key="p.label" class="pick pane tap" @click="snooze(p.at)">
          <span>{{ p.label }}</span>
          <span class="muted small">{{ pickSub(p) }}</span>
        </button>
      </div>

      <div class="actions">
        <button class="btn" @click="picking = true"><CalendarDays :size="18" /> Pick</button>
        <button class="btn" :class="{ chosen: typing }" @click="typing = !typing"><Keyboard :size="18" /> Type</button>
        <button v-if="task?.rrule" class="btn" @click="skip"><SkipForward :size="18" /> Skip</button>
      </div>

      <form v-if="typing" class="typed" @submit.prevent="snooze(typedAt)">
        <input
          v-model="typed"
          class="field"
          placeholder="fri 3pm"
          enterkeyhint="done"
          autocorrect="on"
          autofocus
        />
        <button class="btn primary" :disabled="!typedAt">Snooze</button>
      </form>
      <p v-if="typing && typedAt" class="muted small">{{ dayLabel(typedAt) }}</p>
      <p v-if="error" class="error">{{ error }}</p>
    </div>

    <DateTimeSheet
      v-if="picking"
      :model-value="null"
      title="Snooze until"
      :min="Date.now()"
      done-label="Snooze"
      @update:model-value="snooze"
      @close="picking = false"
    />
  </div>
</template>

<style scoped>
.backdrop {
  position: fixed; inset: 0; z-index: 55; background: oklch(0 0 0 / 22%);
  display: flex; align-items: flex-end; animation: fade-in 0.15s;
}
.sheet {
  width: calc(100% - 16px); max-width: 520px; margin: 0 auto 8px; border-radius: 22px;
  padding: 0 14px calc(14px + env(safe-area-inset-bottom));
  display: flex; flex-direction: column; gap: 12px; max-height: 92vh; overflow-y: auto;
  animation: float-in 0.24s cubic-bezier(0.22, 1, 0.36, 1);
  transition: transform 0.2s cubic-bezier(0.22, 1, 0.36, 1);
}
.sheet.dragging { transition: none; user-select: none; }
.grab {
  display: flex; flex-direction: column; gap: 2px; padding: 8px 4px 2px;
  touch-action: none; cursor: grab; user-select: none;
}
.handle { width: 36px; height: 4px; border-radius: 2px; background: color-mix(in oklch, var(--muted) 45%, transparent); align-self: center; margin-bottom: 8px; }
.title { font-size: 1.1rem; font-weight: 600; overflow-wrap: anywhere; }
.grid { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
.pick { display: flex; flex-direction: column; align-items: flex-start; gap: 2px; text-align: left; padding: 12px 14px; }
.actions { display: flex; gap: 8px; }
.actions .btn { flex: 1; }
.typed { display: flex; gap: 8px; }
</style>
