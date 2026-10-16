// A sheet that lives in the URL (`?task=<id>`, `?snooze=<id>`), over
// whatever page is open, so a notification or link can open the app right
// on it. Opening pushes a history entry (Back closes it); one that arrived
// in the URL (notification) is closed by replacing the URL instead.
// The sheets themselves are mounted once, in RouteSheets (app.vue).

export function useQuerySheet(key: 'task' | 'snooze') {
  const route = useRoute()
  const router = useRouter()
  const flag = `${key}Pushed`
  const id = computed(() => (typeof route.query[key] === 'string' ? (route.query[key] as string) : null))

  function open(taskId: string) {
    router.push({ query: { ...route.query, [key]: taskId }, state: { [flag]: true } })
  }

  function close() {
    if (!id.value) return
    if (history.state?.[flag]) return router.back()
    const { [key]: _, ...query } = route.query
    router.replace({ query })
  }

  return { id, open, close }
}

export const useSnoozeSheet = () => useQuerySheet('snooze')
export const useTaskSheet = () => useQuerySheet('task')
