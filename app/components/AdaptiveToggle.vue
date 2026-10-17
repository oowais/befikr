<script setup lang="ts">
// How a repeat counts its next date: Static (on the schedule, like bins) or
// Adaptive (from when done, like plants). The info button explains both.
import { Info } from 'lucide-vue-next'

defineProps<{ modelValue: boolean }>()
const emit = defineEmits<{ 'update:modelValue': [adaptive: boolean] }>()
const explaining = ref(false)
</script>

<template>
  <span class="mode">
    <span class="seg" role="radiogroup" aria-label="Next date counts">
      <button type="button" role="radio" :aria-checked="!modelValue" class="opt" :class="{ chosen: !modelValue }" @click="emit('update:modelValue', false)">
        Static
      </button>
      <button type="button" role="radio" :aria-checked="modelValue" class="opt" :class="{ chosen: modelValue }" @click="emit('update:modelValue', true)">
        Adaptive
      </button>
    </span>
    <button type="button" class="icon-btn info" aria-label="What do Static and Adaptive mean?" @click="explaining = true">
      <Info :size="18" />
    </button>
    <RepeatModeInfo v-if="explaining" @close="explaining = false" />
  </span>
</template>

<style scoped>
.mode { display: inline-flex; align-items: center; gap: 2px; }
.seg {
  display: inline-flex; gap: 2px; padding: 3px; border-radius: 999px;
  box-shadow: inset 0 1px 0 var(--glass-control-rim), inset 0 0 0 1px var(--line);
}
.opt { padding: 5px 12px; border-radius: 999px; font-size: 0.9rem; color: var(--muted); white-space: nowrap; }
.opt.chosen { color: var(--text); }
.info { color: var(--muted); }
</style>
