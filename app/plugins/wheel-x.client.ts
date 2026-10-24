// v-wheel-x: a mouse wheel scrolls a sideways-only row (chip rows in sheets).
// Windows has no vertical-to-horizontal mapping, so without this a row that
// overflows can't be scrolled with a mouse (issue #14). Touchpads and Shift
// already scroll sideways and are left alone; at either end the wheel passes on.
export default defineNuxtPlugin((nuxtApp) => {
  nuxtApp.vueApp.directive<HTMLElement>('wheel-x', {
    mounted(el) {
      el.addEventListener(
        'wheel',
        (e) => {
          if (e.ctrlKey || e.shiftKey || e.deltaX || !e.deltaY) return
          const max = el.scrollWidth - el.clientWidth
          if (max <= 0) return
          const dy = e.deltaY * (e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? el.clientWidth : 1)
          if ((dy < 0 && el.scrollLeft <= 0) || (dy > 0 && el.scrollLeft >= max - 1)) return
          e.preventDefault()
          el.scrollLeft += dy
        },
        { passive: false },
      )
    },
  })
})
