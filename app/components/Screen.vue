<script setup lang="ts">
// Shared page frame: back button, title and optional #actions pinned at the
// top, the page's content scrolling below. Built like the home screen
// (index.vue): fixed between the status and gesture bars, so only the body
// ever scrolls.
import { ArrowLeft } from 'lucide-vue-next'
defineProps<{ title: string }>()
const router = useRouter()
</script>

<template>
  <main class="page screen">
    <header class="topbar">
      <button class="icon-btn" aria-label="Back" @click="router.back()"><ArrowLeft :size="22" /></button>
      <h1>{{ title }}</h1>
      <slot name="actions" />
    </header>
    <div class="body"><slot /></div>
  </main>
</template>

<style scoped>
.screen {
  position: fixed; top: env(safe-area-inset-top); bottom: env(safe-area-inset-bottom); left: 0; right: 0;
  display: flex; flex-direction: column; padding-bottom: 0;
}
.topbar { flex: none; }
/* Full width so the scrollbar (desktop) sits at the window edge. */
.body {
  flex: 1; min-height: 0; overflow-y: auto; overscroll-behavior: contain;
  margin: 0 -16px; padding: 0 16px 48px;
}
</style>
