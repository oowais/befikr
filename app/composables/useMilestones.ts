// Quiet milestones: one toast, once each (device-local), for a new tree in
// the garden, a carefree streak of 7 / 30 / 100 / 365 days, and clearing a
// backlog. Waits for any toast on screen (e.g. "Completed · Undo") to go
// first. Already-reached ones are marked seen silently the first time.
import type { Carefree } from '~/composables/api'
import { TREE_AT, TREE_NAMES } from '~/utils/garden'

const KEY = 'befikr.milestones'
const STREAKS = [7, 30, 100, 365]
const queue = ref<string[]>([])
let watching = false

function readSeen(): Set<string> | null {
  try {
    const raw = localStorage.getItem(KEY)
    return raw ? new Set(JSON.parse(raw)) : null
  } catch {
    return new Set()
  }
}
function saveSeen(seen: Set<string>) {
  try {
    localStorage.setItem(KEY, JSON.stringify([...seen]))
  } catch {}
}

/** Reached milestones as key → message. */
function reached(c: Carefree): [string, string][] {
  const out: [string, string][] = []
  TREE_AT.forEach((n, i) => c.onTimeTotal >= n && out.push([`tree:${i}`, `${TREE_NAMES[i]} grew in your garden`]))
  for (const n of STREAKS) if (c.best >= n) out.push([`streak:${n}`, `${n} carefree days in a row`])
  return out
}

export function useMilestones() {
  const { current, show } = useToast()
  if (!watching) {
    watching = true
    // One at a time, each after the toast before it is gone.
    watch(
      [current, () => queue.value.length],
      () => {
        if (current.value || !queue.value.length) return
        setTimeout(() => {
          if (current.value) return
          const next = queue.value.shift()
          if (next) show({ message: next }, 4000)
        }, 600)
      },
    )
  }

  /** Queue toasts for milestones not seen before (newest data from core). */
  function check(c: Carefree | null) {
    if (!c) return
    const all = reached(c)
    let seen = readSeen()
    if (!seen) {
      // First run: whatever is reached already isn't news.
      saveSeen(new Set(all.map(([k]) => k)))
      return
    }
    const fresh = all.filter(([k]) => !seen!.has(k))
    if (!fresh.length) return
    for (const [k] of fresh) seen.add(k)
    saveSeen(seen)
    // Several at once (e.g. after a sync): only the biggest one.
    queue.value.push(fresh.at(-1)![1])
  }

  /** Overdue tasks went from several to none. */
  function comeback() {
    queue.value.push('Backlog cleared. Nice comeback.')
  }

  return { check, comeback }
}
