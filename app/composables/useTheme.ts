// Theme: follow the system, or force light / dark. A per-device preference
// (localStorage), applied as `data-theme` on <html> (see main.css).
import { ref } from 'vue'

export type Theme = 'system' | 'light' | 'dark'
const KEY = 'befikr.theme'

function stored(): Theme {
  try {
    const t = localStorage.getItem(KEY)
    return t === 'light' || t === 'dark' ? t : 'system'
  } catch {
    return 'system'
  }
}

const theme = ref<Theme>('system')

/** Set the attribute and, on desktop, the window frame's theme. */
export function applyTheme(t: Theme) {
  theme.value = t
  const el = document.documentElement
  if (t === 'system') el.removeAttribute('data-theme')
  else el.setAttribute('data-theme', t)
  import('@tauri-apps/api/window')
    .then(({ getCurrentWindow }) => getCurrentWindow().setTheme(t === 'system' ? null : t))
    .catch(() => {})
}

export function useTheme() {
  function setTheme(t: Theme) {
    try {
      if (t === 'system') localStorage.removeItem(KEY)
      else localStorage.setItem(KEY, t)
    } catch {
      /* session only */
    }
    applyTheme(t)
  }
  return { theme, setTheme }
}

export const loadTheme = () => applyTheme(stored())
