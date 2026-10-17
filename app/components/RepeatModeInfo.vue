<script setup lang="ts">
// What Static and Adaptive mean (AdaptiveToggle's info button). Mirrors
// recurrence::next_due in the core.
import { CalendarDays, RotateCcw, X } from 'lucide-vue-next'

const emit = defineEmits<{ close: [] }>()
useBackClose(() => emit('close'))
</script>

<template>
  <Teleport to="body">
    <div class="backdrop" @click.self="emit('close')">
      <div class="dialog popover" role="dialog" aria-labelledby="repeat-mode-title">
        <header class="head">
          <b id="repeat-mode-title">Static or adaptive?</b>
          <button class="icon-btn" aria-label="Close" @click="emit('close')"><X :size="22" /></button>
        </header>
        <p class="muted">Both decide when the next one is due after you tick this one off.</p>

        <section class="mode">
          <h3><CalendarDays :size="18" /> Static</h3>
          <p>Stays on its dates. The next one comes from the schedule, however early or late you finish this one. Missed ones are skipped, not stacked up.</p>
          <p class="example">Every Monday, done on Wednesday → next is still Monday.</p>
          <p class="muted small">For things tied to the calendar: bins, rent, a weekly call.</p>
        </section>

        <section class="mode">
          <h3><RotateCcw :size="18" /> Adaptive</h3>
          <p>Counts from when you finish. The next one is the repeat's gap after the day you tick it off, at the same time of day.</p>
          <p class="example">Every 7 days, done on Wednesday → next is the Wednesday after.</p>
          <p class="muted small">For things that need a gap between them: watering plants, a haircut, changing sheets.</p>
        </section>

        <button class="btn primary" @click="emit('close')">Got it</button>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.backdrop {
  position: fixed; inset: 0; z-index: 80; background: oklch(0 0 0 / 28%);
  display: grid; place-items: center; padding: 16px; animation: fade-in 0.15s;
}
.dialog {
  width: 100%; max-width: 420px; max-height: 90vh; overflow-y: auto; border-radius: 22px;
  padding: 8px 18px 18px; display: flex; flex-direction: column; gap: 12px;
  animation: float-in 0.24s cubic-bezier(0.22, 1, 0.36, 1);
}
.head { display: flex; align-items: center; justify-content: space-between; margin-right: -8px; }
.head b { font-size: 1.05rem; }
p { margin: 0; }
.mode { display: flex; flex-direction: column; gap: 6px; padding-top: 12px; border-top: 1px solid var(--line); }
.mode h3 { display: flex; align-items: center; gap: 8px; margin: 0; font-size: 1rem; font-weight: 600; }
.example {
  padding: 8px 12px; border-radius: 12px; font-size: 0.9rem;
  background: color-mix(in oklch, var(--text) 5%, transparent);
}
.btn.primary { margin-top: 4px; }
</style>
