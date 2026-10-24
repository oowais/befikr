// A mouse wheel scrolls sideways-only strips (chip rows in sheets). Windows
// has no vertical-to-horizontal mapping and the strips hide their scrollbar,
// so without this a row that overflows can't be scrolled with a mouse
// (issue #14). Applies to every such strip, so new rows need nothing. Tall
// sideways scrollers (calendar week) keep the wheel for the page. Touchpads
// and Shift already scroll sideways and are left alone; at either end the
// wheel passes on.
const STRIP_MAX_HEIGHT = 100

function strip(from: EventTarget | null): HTMLElement | null {
  for (let el = from instanceof Element ? from : null; el && el !== document.body; el = el.parentElement) {
    if (!(el instanceof HTMLElement) || el.scrollWidth <= el.clientWidth) continue
    const { overflowX } = getComputedStyle(el)
    if (overflowX !== 'auto' && overflowX !== 'scroll') continue
    return el.scrollHeight <= el.clientHeight && el.clientHeight <= STRIP_MAX_HEIGHT ? el : null
  }
  return null
}

export default defineNuxtPlugin(() => {
  window.addEventListener(
    'wheel',
    (e) => {
      if (e.ctrlKey || e.shiftKey || e.deltaX || !e.deltaY) return
      const el = strip(e.target)
      if (!el) return
      const max = el.scrollWidth - el.clientWidth
      const dy = e.deltaY * (e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? el.clientWidth : 1)
      if ((dy < 0 && el.scrollLeft <= 0) || (dy > 0 && el.scrollLeft >= max - 1)) return
      e.preventDefault()
      el.scrollLeft += dy
    },
    { passive: false },
  )
})
