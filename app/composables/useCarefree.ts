// Gamification: carefree days (a day that ends with nothing overdue) and the
// landscape that grows with on-time completions. Numbers come from core
// (`Store::carefree`); the on/off switch is device-local.
import type { Carefree } from '~/composables/api'

const KEY = 'befikr.garden'
const enabled = ref(read())
const data = ref<Carefree | null>(null)

function read(): boolean {
  try {
    return localStorage.getItem(KEY) !== 'off'
  } catch {
    return true
  }
}

export function useCarefree() {
  function setEnabled(on: boolean) {
    enabled.value = on
    try {
      localStorage.setItem(KEY, on ? 'on' : 'off')
    } catch {}
  }
  async function refresh() {
    data.value = await api.carefree().catch(() => data.value)
  }
  return { enabled, data, setEnabled, refresh }
}
