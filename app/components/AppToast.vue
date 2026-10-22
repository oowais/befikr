<script setup lang="ts">
// The toast at the bottom ("Deleted · Undo"). Swipe it away left or right;
// it can sit over the quick-add bar.
const { current: toast, dismiss, pause, resume } = useToast()

const AXIS_LOCK = 10 // px before deciding horizontal vs vertical
const el = ref<HTMLElement>()
const dx = ref(0)
const dragging = ref(false)
let startX = 0
let startY = 0
let axis: 'x' | 'y' | null = null
let swiped = false

const threshold = () => Math.min(90, (el.value?.offsetWidth ?? 360) * 0.25)

function down(e: PointerEvent) {
  if (e.pointerType === 'mouse' && e.button !== 0) return
  startX = e.clientX
  startY = e.clientY
  axis = null
  swiped = false
  dragging.value = true
}

function move(e: PointerEvent) {
  if (!dragging.value) return
  const x = e.clientX - startX
  const y = e.clientY - startY
  if (!axis) {
    if (Math.abs(x) < AXIS_LOCK && Math.abs(y) < AXIS_LOCK) return
    axis = Math.abs(x) > Math.abs(y) ? 'x' : 'y'
    if (axis === 'y') return (dragging.value = false)
    swiped = true
    pause()
    ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
  }
  dx.value = x
}

function up() {
  if (!dragging.value) return
  dragging.value = false
  if (Math.abs(dx.value) >= threshold()) {
    // Fly out the way it was thrown, then go (no second leave animation).
    dx.value = Math.sign(dx.value) * ((el.value?.offsetWidth ?? 400) + 32)
    setTimeout(dismiss, 180)
  } else {
    dx.value = 0
    if (swiped) resume()
  }
  // The click (if any) follows right after pointerup; later taps are taps.
  if (swiped) setTimeout(() => (swiped = false))
}

// A swipe that started on Undo ends with a click on it: not an Undo.
function runAction() {
  if (swiped) return
  toast.value?.action?.run()
  dismiss()
}

// Each new toast starts in place.
watch(toast, () => {
  dx.value = 0
  dragging.value = false
})
</script>

<template>
  <Transition name="toast">
    <div
      v-if="toast"
      ref="el"
      class="toast popover"
      :class="{ dragging, out: dx !== 0 }"
      :style="dx ? { transform: `translateX(${dx}px)`, opacity: Math.max(0, 1 - Math.abs(dx) / ((el?.offsetWidth ?? 400) * 0.9)) } : undefined"
      role="status"
      @pointerdown="down"
      @pointermove="move"
      @pointerup="up"
      @pointercancel="up"
    >
      <span>{{ toast.message }}</span>
      <button v-if="toast.action" class="btn" @click="runAction">{{ toast.action.label }}</button>
    </div>
  </Transition>
</template>

<style>
.toast {
  position: fixed; left: 16px; right: 16px; bottom: calc(24px + env(safe-area-inset-bottom));
  max-width: 480px; margin: 0 auto; z-index: 70;
  display: flex; align-items: center; justify-content: space-between; gap: 12px;
  padding: 8px 8px 8px 16px; border-radius: var(--radius);
  touch-action: pan-y; user-select: none; -webkit-user-select: none;
  transition: transform 0.2s cubic-bezier(0.22, 1, 0.36, 1), opacity 0.2s;
}
.toast.dragging { transition: none; }
.toast-enter-active, .toast-leave-active { transition: transform 0.25s cubic-bezier(0.22, 1, 0.36, 1), opacity 0.2s; }
.toast-enter-from, .toast-leave-to { transform: translateY(12px) scale(0.985); opacity: 0; }
/* Swiped out: already gone sideways, don't drop down as well. */
.toast.out.toast-leave-active { transition: none; }
</style>
