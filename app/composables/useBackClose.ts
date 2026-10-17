// Android back button and Escape (computers): close the top open sheet. With
// none open, back goes back a page, or leaves the app from the first one.
// (Registering the handler turns off Tauri's default, which closed the app
// even with a sheet open.)
import { exit, onBackButtonPress } from '@tauri-apps/api/app'

const open: (() => void)[] = []

/** Whether any sheet is open (keyboard shortcuts wait until it's closed). */
export const sheetOpen = () => open.length > 0

/** Call in a sheet component: back closes it while it's mounted. */
export function useBackClose(close: () => void) {
  onMounted(() => open.push(close))
  onUnmounted(() => {
    const i = open.lastIndexOf(close)
    if (i >= 0) open.splice(i, 1)
  })
}

// Escape closes the top sheet, unless something above it (a context menu)
// already handled the key: checked after every listener has run.
if (typeof window !== 'undefined') {
  window.addEventListener('keydown', (e) => {
    if (e.key !== 'Escape' || e.isComposing) return
    setTimeout(() => {
      if (!e.defaultPrevented) open.at(-1)?.()
    })
  })
}

/** Once, at app start. Rejects off Android (no back button). */
export const handleBackButton = () =>
  onBackButtonPress(({ canGoBack }) => {
    const top = open.at(-1)
    if (top) top()
    else if (canGoBack) history.back()
    else exit(0)
  })
