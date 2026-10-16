<script setup lang="ts">
// The one right-click menu (see useContextMenu). Opens at the pointer,
// stays inside the window; Escape, a click elsewhere, scroll or resize close it.
const menu = useContextMenu()
const el = ref<HTMLElement>()
const pos = ref({ left: 0, top: 0 })

watch(
  () => menu.open && [menu.x, menu.y],
  async (open) => {
    if (!open) return
    pos.value = { left: menu.x, top: menu.y }
    await nextTick()
    const r = el.value?.getBoundingClientRect()
    if (!r) return
    const pad = 8
    pos.value = {
      left: Math.max(pad, Math.min(menu.x, window.innerWidth - r.width - pad)),
      top: Math.max(pad, Math.min(menu.y, window.innerHeight - r.height - pad)),
    }
    el.value?.querySelector<HTMLElement>('button')?.focus()
  },
)

function run(item: Exclude<MenuItem, 'separator'>) {
  closeMenu()
  item.run()
}

function key(e: KeyboardEvent) {
  if (!menu.open) return
  if (e.key === 'Escape') return closeMenu()
  if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return
  e.preventDefault()
  const buttons = [...(el.value?.querySelectorAll<HTMLElement>('button') ?? [])]
  const i = buttons.indexOf(document.activeElement as HTMLElement)
  buttons[(i + (e.key === 'ArrowDown' ? 1 : -1) + buttons.length) % buttons.length]?.focus()
}

onMounted(() => {
  window.addEventListener('keydown', key)
  window.addEventListener('resize', closeMenu)
  window.addEventListener('scroll', closeMenu, true)
  window.addEventListener('blur', closeMenu)
})
onUnmounted(() => {
  window.removeEventListener('keydown', key)
  window.removeEventListener('resize', closeMenu)
  window.removeEventListener('scroll', closeMenu, true)
  window.removeEventListener('blur', closeMenu)
})
</script>

<template>
  <div v-if="menu.open" class="scrim" @pointerdown="closeMenu" @contextmenu.prevent="closeMenu" />
  <div v-if="menu.open" ref="el" class="menu popover" role="menu" :style="{ left: `${pos.left}px`, top: `${pos.top}px` }">
    <template v-for="(item, i) in menu.items" :key="i">
      <hr v-if="item === 'separator'" />
      <button v-else role="menuitem" class="entry" :class="{ danger: item.danger }" @click="run(item)">
        <component :is="item.icon" v-if="item.icon" :size="16" />
        <span>{{ item.label }}</span>
      </button>
    </template>
  </div>
</template>

<style scoped>
.scrim { position: fixed; inset: 0; z-index: 80; }
.menu {
  position: fixed; z-index: 81; min-width: 180px; padding: 5px; border-radius: var(--radius-control);
  display: flex; flex-direction: column;
}
.entry {
  display: flex; align-items: center; gap: 10px; padding: 7px 10px; border-radius: 7px;
  font-size: 0.92rem; text-align: left; color: var(--text);
}
.entry :deep(svg) { color: var(--muted); flex: none; }
.entry:hover, .entry:focus-visible { background-color: var(--glass-state); outline: none; }
.entry.danger, .entry.danger :deep(svg) { color: var(--red); }
hr { border: 0; border-top: 1px solid var(--line); margin: 4px 6px; }
</style>
