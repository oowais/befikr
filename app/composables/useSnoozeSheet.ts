// The snooze sheet lives in the URL (`?snooze=<task id>`) over the list or
// task view, so a notification's Snooze can open the app right on it.
// Opening pushes a history entry (Back closes it); one that arrived in the
// URL (notification) is closed by replacing the URL instead.

export function useSnoozeSheet() {
  const route = useRoute()
  const router = useRouter()
  const id = computed(() => (typeof route.query.snooze === 'string' ? route.query.snooze : null))

  function open(taskId: string) {
    router.push({ query: { ...route.query, snooze: taskId }, state: { snoozePushed: true } })
  }

  function close() {
    if (!id.value) return
    if (history.state?.snoozePushed) return router.back()
    const { snooze: _, ...query } = route.query
    router.replace({ query })
  }

  return { id, open, close }
}
