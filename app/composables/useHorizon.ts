// How far ahead the home list looks: everything, or only tasks due by the
// end of today / this week / this month / this year (issue #24). Later ones
// fold into one "N hidden" line; overdue, pinned and undated tasks always
// show. A per-device preference (localStorage), like the theme.
import type { Horizon } from '~/composables/format'

const KEY = 'befikr.horizon'
const HORIZONS: Horizon[] = ['all', 'today', 'week', 'month', 'year']

function stored(): Horizon {
  try {
    const h = localStorage.getItem(KEY) as Horizon | null
    return h && HORIZONS.includes(h) ? h : 'all'
  } catch {
    return 'all'
  }
}

const horizon = ref<Horizon>(stored())

/** "after today", "after this week", … for the hidden line. */
export const horizonAfter: Record<Exclude<Horizon, 'all'>, string> = {
  today: 'after today',
  week: 'after this week',
  month: 'after this month',
  year: 'after this year',
}

export function useHorizon() {
  function setHorizon(h: Horizon) {
    horizon.value = h
    try {
      if (h === 'all') localStorage.removeItem(KEY)
      else localStorage.setItem(KEY, h)
    } catch {
      /* session only */
    }
  }
  return { horizon, horizons: HORIZONS, setHorizon }
}
