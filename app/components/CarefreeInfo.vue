<script setup lang="ts">
// How carefree days and the garden work (the streak beside the title, and
// Settings). Mirrors Store::carefree in the core and Doodle's thresholds.
import { Bird, Flower2, Sprout, TreePine, X } from 'lucide-vue-next'
import type { Carefree } from '~/composables/api'
import { TREE_AT } from '~/utils/garden'

defineProps<{ garden?: Carefree | null }>()
const emit = defineEmits<{ close: [] }>()
useBackClose(() => emit('close'))
const days = (n: number) => `${n} ${n === 1 ? 'day' : 'days'}`
</script>

<template>
  <Teleport to="body">
    <div class="backdrop" @click.self="emit('close')">
      <div class="dialog popover" role="dialog" aria-labelledby="carefree-title">
        <header class="head">
          <b id="carefree-title">Carefree days</b>
          <button class="icon-btn" aria-label="Close" @click="emit('close')"><X :size="22" /></button>
        </header>
        <p v-if="garden" class="you">
          <Sprout :size="18" /> {{ days(garden.streak) }} in a row · best {{ days(garden.best) }} · {{ garden.onTimeTotal }} done on time
        </p>

        <section class="part">
          <h3><Sprout :size="18" /> The streak</h3>
          <p>A day is carefree when nothing due that day, or earlier, is still undone by noon the next day. So you have until midday tomorrow to catch up, and today counts once that's passed.</p>
          <p>Rescheduled and deleted tasks don't count against you, and tasks without a date never do. One task left overdue stops the streak every day until it's done, moved or deleted.</p>
          <p class="example">Missed one day? It's forgiven if you're on a streak and the last forgiven day was at least a week ago.</p>
        </section>

        <section class="part">
          <h3><TreePine :size="18" /> The garden</h3>
          <p>The landscape under your list grows as you finish tasks on time (by their due day). Nothing is taken away for missed days; it just stops growing.</p>
          <ul>
            <li><TreePine :size="16" /> A tree after {{ TREE_AT.slice(0, 4).join(', ') }}… on-time tasks in all, {{ TREE_AT.length }} in the end.</li>
            <li><Flower2 :size="16" /> A flower for each on-time task this week, fresh every Monday.</li>
            <li><Bird :size="16" /> Birds (stars at night) at a 3, 7, 14 and 30 day streak.</li>
          </ul>
          <p class="muted small">It follows the season: blossom, summer green, autumn colours, snow.</p>
        </section>

        <p class="muted small">Turn it off in Settings → Carefree days. History and best streak are on Stats.</p>
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
.you {
  display: flex; align-items: center; gap: 8px; padding: 8px 12px; border-radius: 12px; font-size: 0.9rem;
  background: color-mix(in oklch, var(--accent) 12%, transparent); font-variant-numeric: tabular-nums;
}
.part { display: flex; flex-direction: column; gap: 6px; padding-top: 12px; border-top: 1px solid var(--line); }
.part h3 { display: flex; align-items: center; gap: 8px; margin: 0; font-size: 1rem; font-weight: 600; }
ul { margin: 0; padding: 0; list-style: none; display: flex; flex-direction: column; gap: 6px; }
li { display: flex; align-items: flex-start; gap: 8px; }
li svg { flex: none; margin-top: 3px; color: var(--muted); }
.example {
  padding: 8px 12px; border-radius: 12px; font-size: 0.9rem;
  background: color-mix(in oklch, var(--text) 5%, transparent);
}
.btn.primary { margin-top: 4px; }
</style>
