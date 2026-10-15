// Date labels: relative near the due time, absolute further out (Flighty-style).
import { RRule } from 'rrule'

const MIN = 60_000
const HOUR = 60 * MIN
const DAY = 24 * HOUR

const time = (d: Date) => d.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' })
const startOfDay = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime()

/** "25 min", "2h", "3d". */
export function span(ms: number): string {
  if (ms < HOUR) return `${Math.max(1, Math.round(ms / MIN))} min`
  if (ms < DAY) return `${Math.round(ms / HOUR)}h`
  return `${Math.round(ms / DAY)}d`
}

/** `timeOnly`: the list already says it's today, so "18:00" instead of "Today 18:00". */
export function dueLabel(ms: number | null, now = Date.now(), timeOnly = false): string {
  if (ms == null) return ''
  const diff = ms - now
  if (diff < 0) return `${span(-diff)} late`
  if (diff < HOUR) return `in ${span(diff)}`
  if (timeOnly && isToday(ms, now)) return time(new Date(ms))
  return dayLabel(ms, now)
}

export const isToday = (ms: number, now = Date.now()) => startOfDay(new Date(ms)) === startOfDay(new Date(now))

export const timeLabel = (ms: number) => time(new Date(ms))

/** List sections, in display order. */
export type Section = 'overdue' | 'today' | 'later' | 'none'
export const sectionTitles: Record<Section, string> = { overdue: 'Overdue', today: 'Today', later: 'Upcoming', none: 'No date' }

export function sectionOf(ms: number | null, now = Date.now()): Section {
  if (ms == null) return 'none'
  if (ms < now) return 'overdue'
  return isToday(ms, now) ? 'today' : 'later'
}

/** "Today 18:00", "Tomorrow 09:00", "Fri 15:00", "23 Oct 15:00". */
export function dayLabel(ms: number, now = Date.now()): string {
  const d = new Date(ms)
  const days = Math.round((startOfDay(d) - startOfDay(new Date(now))) / DAY)
  if (days === 0) return `Today ${time(d)}`
  if (days === 1) return `Tomorrow ${time(d)}`
  if (days === -1) return `Yesterday ${time(d)}`
  if (days > 1 && days < 7) return `${d.toLocaleDateString(undefined, { weekday: 'short' })} ${time(d)}`
  return `${d.toLocaleDateString(undefined, { day: 'numeric', month: 'short' })} ${time(d)}`
}

export function dateOnlyLabel(date: string, now = Date.now()): string {
  const [y, m, d] = date.split('-').map(Number)
  const ms = new Date(y!, m! - 1, d!).getTime()
  const days = Math.round((ms - startOfDay(new Date(now))) / DAY)
  if (days === 0) return 'Today'
  if (days === 1) return 'Tomorrow'
  return new Date(ms).toLocaleDateString(undefined, { weekday: 'short', day: 'numeric', month: 'short' })
}

export function rruleText(rrule: string | null): string {
  if (!rrule) return ''
  try {
    return RRule.fromString(`RRULE:${rrule}`).toText()
  } catch {
    return rrule
  }
}

/** Local wall-clock → epoch ms. */
export function localMs(date: string, time: string): number {
  const [y, m, d] = date.split('-').map(Number)
  const [h, mi] = time.split(':').map(Number)
  return new Date(y!, m! - 1, d!, h!, mi!).getTime()
}

export const haptic = () => {
  try {
    navigator.vibrate?.(12)
  } catch {
    /* not supported */
  }
}
