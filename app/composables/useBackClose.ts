// Android back button: closes the top open sheet; with none open, goes back a
// page, or leaves the app from the first one. (Registering the handler turns
// off Tauri's default, which closed the app even with a sheet open.)
import { exit, onBackButtonPress } from '@tauri-apps/api/app'

const open: (() => void)[] = []

/** Call in a sheet component: back closes it while it's mounted. */
export function useBackClose(close: () => void) {
  onMounted(() => open.push(close))
  onUnmounted(() => {
    const i = open.lastIndexOf(close)
    if (i >= 0) open.splice(i, 1)
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
