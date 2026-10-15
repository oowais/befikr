<script setup lang="ts">
// How a repeat counts its next date. Both options show, so the choice
// explains itself: bins go on schedule, plants count from when done.
defineProps<{ modelValue: boolean }>()
const emit = defineEmits<{ 'update:modelValue': [adaptive: boolean] }>()
</script>

<template>
  <div class="seg" role="radiogroup" aria-label="Next date counts">
    <button type="button" role="radio" :aria-checked="!modelValue" class="opt" :class="{ chosen: !modelValue }" @click="emit('update:modelValue', false)">
      On schedule
    </button>
    <button type="button" role="radio" :aria-checked="modelValue" class="opt" :class="{ chosen: modelValue }" @click="emit('update:modelValue', true)">
      From when done
    </button>
  </div>
</template>

<style scoped>
.seg {
  display: inline-flex; gap: 2px; padding: 3px; border-radius: 999px;
  box-shadow: inset 0 1px 0 var(--glass-control-rim), inset 0 0 0 1px var(--line);
}
.opt { padding: 5px 12px; border-radius: 999px; font-size: 0.9rem; color: var(--muted); white-space: nowrap; }
.opt.chosen { color: var(--text); }
</style>
