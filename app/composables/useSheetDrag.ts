// Drag a sheet's top edge down to close it; short drags snap back. Bind the
// handlers to the grab area (not the whole sheet, which scrolls), and
// `style` + `dragging` to the sheet.

export function useSheetDrag(close: () => void, threshold = 90) {
  const y = ref(0)
  const dragging = ref(false)
  let startY = 0

  const handlers = {
    pointerdown(e: PointerEvent) {
      // Buttons in the grab area stay buttons (capturing would eat their click).
      if ((e.target as HTMLElement).closest('button, a, input, textarea')) return
      dragging.value = true
      startY = e.clientY
      ;(e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId)
    },
    pointermove(e: PointerEvent) {
      if (dragging.value) y.value = Math.max(0, e.clientY - startY)
    },
    pointerup() {
      if (!dragging.value) return
      dragging.value = false
      if (y.value > threshold) close()
      else y.value = 0
    },
    pointercancel() {
      dragging.value = false
      y.value = 0
    },
  }
  const style = computed(() => (y.value ? { transform: `translateY(${y.value}px)` } : undefined))
  return { dragging, style, handlers }
}
