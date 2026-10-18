<script setup lang="ts">
// What changed in the versions since the last update (or the last 10, from
// Settings). A bottom sheet like snooze: drag down, tap outside, Back or
// "Got it" closes it. Notes: see useWhatsNew.
import { Sparkles, Wrench } from 'lucide-vue-next'
import type { Release } from '~/composables/useWhatsNew'

const props = defineProps<{ releases: Release[]; since: string | null }>()
const emit = defineEmits<{ close: [] }>()
useBackClose(() => emit('close'))
const drag = useSheetDrag(() => emit('close'))

const subtitle = computed(() => {
  const latest = props.releases[0]?.version
  if (!latest) return ''
  return props.since ? `Updated to ${latest} · since ${props.since}` : 'Recent versions'
})
/** One version after an update: no version headings needed. */
const single = computed(() => props.since != null && props.releases.length === 1)
const day = (d: string) => new Date(`${d}T12:00:00`).toLocaleDateString(undefined, { day: 'numeric', month: 'short' })
</script>

<template>
  <div class="backdrop" @click.self="emit('close')">
    <div
      class="sheet popover"
      role="dialog"
      aria-labelledby="whats-new-title"
      :class="{ dragging: drag.dragging.value }"
      :style="drag.style.value"
      v-on="drag.handlers"
    >
      <div class="grab">
        <span class="handle" />
        <span id="whats-new-title" class="title">What's new</span>
        <span v-if="subtitle" class="muted small">{{ subtitle }}</span>
      </div>

      <p v-if="!releases.length" class="muted small">This build has no release notes.</p>
      <section v-for="r in releases" :key="r.version" class="release">
        <h3 v-if="!single" class="version">{{ r.version }} <span class="muted">· {{ day(r.date) }}</span></h3>
        <div v-if="r.new.length" class="group">
          <span class="label muted small"><Sparkles :size="15" /> New</span>
          <ul><li v-for="t in r.new" :key="t">{{ t }}</li></ul>
        </div>
        <div v-if="r.fixes.length" class="group">
          <span class="label muted small"><Wrench :size="15" /> Fixes</span>
          <ul><li v-for="t in r.fixes" :key="t">{{ t }}</li></ul>
        </div>
      </section>

      <button class="btn primary" @click="emit('close')">Got it</button>
    </div>
  </div>
</template>

<style scoped>
.backdrop {
  position: fixed; inset: 0; z-index: 60; background: oklch(0 0 0 / 22%);
  display: flex; align-items: flex-end; animation: fade-in 0.15s;
}
.sheet {
  width: calc(100% - 16px); max-width: 520px; margin: 0 auto 8px; border-radius: 22px;
  padding: 0 16px calc(14px + env(safe-area-inset-bottom));
  display: flex; flex-direction: column; gap: 14px; max-height: 85vh; overflow-y: auto;
  animation: float-in 0.24s cubic-bezier(0.22, 1, 0.36, 1);
  transition: transform 0.2s cubic-bezier(0.22, 1, 0.36, 1);
}
.sheet.dragging { transition: none; user-select: none; }
.grab {
  display: flex; flex-direction: column; gap: 2px; padding: 8px 4px 2px;
  touch-action: none; cursor: grab; user-select: none;
}
.handle { width: 36px; height: 4px; border-radius: 2px; background: color-mix(in oklch, var(--muted) 45%, transparent); align-self: center; margin-bottom: 8px; }
.title { font-size: 1.1rem; font-weight: 600; }
p { margin: 0; }
.release { display: flex; flex-direction: column; gap: 10px; }
.release + .release { padding-top: 12px; border-top: 1px solid var(--line); }
.version { margin: 0; font-size: 0.95rem; font-weight: 600; }
.version .muted { font-weight: 400; }
.group { display: flex; flex-direction: column; gap: 4px; }
.label { display: flex; align-items: center; gap: 6px; }
ul { margin: 0; padding-left: 21px; display: flex; flex-direction: column; gap: 4px; }
li { line-height: 1.4; }
li::marker { color: var(--muted); }
.btn.primary { margin-top: 2px; }
</style>
