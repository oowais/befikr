// Back to the main list as the first page: unwinds history so the Android
// back button doesn't return to screens left behind (task view, snooze).

/** vue-router's history position when the app started (not always 0). */
let start = 0
export const rememberHistoryStart = () => (start = Number(history.state?.position ?? 0))

export function useGoHome() {
  const router = useRouter()
  return () => {
    const steps = Number(history.state?.position ?? start) - start
    if (steps <= 0) return router.replace('/')
    // The first page is normally the list; if not, swap it in once there.
    window.addEventListener('popstate', () => location.pathname !== '/' && router.replace('/'), { once: true })
    router.go(-steps)
  }
}
