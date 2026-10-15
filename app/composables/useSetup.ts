// Permission tour state. Shown on launch while a permission is missing,
// unless the user chose "Later" in the last 7 days.
import type { Status } from './api'

const SKIP_KEY = 'befikr.setupSkippedAt'
const WEEK = 7 * 86_400_000

export const allGranted = (s: Status) => s.notifications && s.exactAlarms && s.batteryUnrestricted

export async function needsSetup(): Promise<boolean> {
  const s = await reminders.status()
  if (allGranted(s)) return false
  try {
    const skipped = Number(localStorage.getItem(SKIP_KEY) ?? 0)
    return Date.now() - skipped > WEEK
  } catch {
    return true
  }
}

export function skipSetup() {
  try {
    localStorage.setItem(SKIP_KEY, String(Date.now()))
  } catch {
    /* session only */
  }
}
