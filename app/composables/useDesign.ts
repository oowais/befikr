// UI design (Classic / Glass). Device-local preference; temporary while both
// designs are tried side by side.
export type Design = 'classic' | 'glass'

const KEY = 'befikr.design'
export const DESIGNS: { value: Design; label: string; hint: string }[] = [
  { value: 'classic', label: 'Classic', hint: 'Sage & mint, colour bars' },
  { value: 'glass', label: 'Glass', hint: 'Smoky glass panes, ink buttons' },
]

const design = ref<Design>(read())

function read(): Design {
  try {
    return localStorage.getItem(KEY) === 'glass' ? 'glass' : 'classic'
  } catch {
    return 'classic'
  }
}

export function useDesign() {
  function set(d: Design) {
    design.value = d
    document.documentElement.dataset.design = d
    try {
      localStorage.setItem(KEY, d)
    } catch {
      /* storage unavailable: applies for this session only */
    }
  }
  return { design, set }
}
