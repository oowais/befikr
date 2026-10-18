// Drag a sheet down from anywhere to close it; short drags snap back. Bind
// the handlers and `style` + `dragging` to the sheet (the element that
// scrolls). A drag only starts going down while the sheet (and any list
// under the finger) is scrolled to the top, and never from a text field, so
// taps, scrolling and typing work as before. Touch uses touch events (a
// pointer drag would be taken over by the browser's scrolling); the mouse
// uses pointer events.

const SLOP = 8
const FIELDS = 'input, textarea, select, [contenteditable]'

export function useSheetDrag(close: () => void, threshold = 90) {
  const y = ref(0)
  const dragging = ref(false)
  let start: { x: number; y: number } | null = null
  let swallowClick = false

  /** Something between the finger and the sheet is scrolled down. */
  function scrolled(target: Element, sheet: Element) {
    for (let el: Element | null = target; el && el !== sheet.parentElement; el = el.parentElement) {
      if (el.scrollTop > 0) return true
    }
    return false
  }
  function begin(x: number, yy: number, target: EventTarget | null, sheet: EventTarget | null) {
    const t = target as Element
    start = t.closest(FIELDS) || scrolled(t, sheet as Element) ? null : { x, y: yy }
  }
  /** True once this gesture is a drag (the caller then stops the default). */
  function move(x: number, yy: number) {
    if (!start) return false
    const dx = x - start.x
    const dy = yy - start.y
    if (!dragging.value) {
      if (Math.abs(dx) < SLOP && Math.abs(dy) < SLOP) return false
      // Up or sideways: not ours (scroll, select).
      if (dy <= 0 || Math.abs(dx) > dy) {
        start = null
        return false
      }
      dragging.value = true
    }
    y.value = Math.max(0, dy)
    return true
  }
  function end() {
    start = null
    if (!dragging.value) return
    dragging.value = false
    // The release isn't a tap on whatever is under it.
    swallowClick = true
    setTimeout(() => (swallowClick = false))
    if (y.value > threshold) close()
    else y.value = 0
  }

  const handlers = {
    touchstart(e: TouchEvent) {
      const t = e.touches[0]
      if (e.touches.length === 1 && t) begin(t.clientX, t.clientY, e.target, e.currentTarget)
      else start = null
    },
    touchmove(e: TouchEvent) {
      const t = e.touches[0]
      if (t && move(t.clientX, t.clientY)) e.preventDefault()
    },
    touchend: end,
    touchcancel: end,
    pointerdown(e: PointerEvent) {
      if (e.pointerType === 'mouse' && e.button === 0) begin(e.clientX, e.clientY, e.target, e.currentTarget)
    },
    pointermove(e: PointerEvent) {
      if (e.pointerType !== 'mouse' || !move(e.clientX, e.clientY)) return
      ;(e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId)
      e.preventDefault()
    },
    pointerup(e: PointerEvent) {
      if (e.pointerType === 'mouse') end()
    },
    clickCapture(e: MouseEvent) {
      if (!swallowClick) return
      e.stopPropagation()
      e.preventDefault()
    },
  }
  const style = computed(() => (y.value ? { transform: `translateY(${y.value}px)` } : undefined))
  return { dragging, style, handlers }
}
