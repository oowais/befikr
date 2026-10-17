/**
 * Quick-add parser: "buy milk tomorrow evening" → title + due + RRULE + spans.
 * Pure function of (text, now, settings). English only.
 *
 * All dates/times are local wall-clock values; the caller converts to UTC
 * and attaches DTSTART;TZID to the RRULE.
 */
import * as chrono from 'chrono-node'
import { RRule, type Options as RRuleOptions } from 'rrule'

export type Settings = {
  /** Time words → 'HH:MM'. Built-ins are editable; users can add their own (e.g. dawn). */
  timeWords: Record<string, string>
}

export const DEFAULT_SETTINGS: Settings = {
  timeWords: {
    morning: '09:00',
    noon: '12:00',
    afternoon: '14:00',
    evening: '18:00',
    night: '21:00',
  },
}

/** Words that resolve to another time word. */
const TIME_WORD_ALIASES: Record<string, string> = { tonight: 'night' }

export type Span = { start: number; end: number; kind: 'date' | 'recurrence' }

/** Local wall-clock due. `time` null → app prompts with 09:00 prefilled. */
export type Due = { date: string; time: string | null }

export type ParseResult = {
  title: string
  due: Due | null
  /** RRULE body without DTSTART, e.g. 'FREQ=WEEKLY;BYDAY=MO'. */
  rrule: string | null
  /** Human label for the chip, e.g. 'every week on Monday'. */
  recurrenceText: string | null
  /** Parsed ranges in the original text, for highlighting. */
  spans: Span[]
}

export type ParseOptions = {
  /** Ranges the user un-parsed (tapped a highlight): kept literal in the title. */
  ignore?: { start: number; end: number }[]
}

export function parse(
  text: string,
  now: Date,
  settings: Settings = DEFAULT_SETTINGS,
  opts: ParseOptions = {},
): ParseResult {
  let work = mask(text, opts.ignore ?? [])
  const spans: Span[] = []

  const rec = findRecurrence(work, settings)
  if (rec) {
    spans.push({ start: rec.start, end: rec.end, kind: 'recurrence' })
    work = mask(work, [rec])
  }

  const date = findDate(work, now, settings)
  if (date) spans.push({ start: date.start, end: date.end, kind: 'date' })
  spans.sort((a, b) => a.start - b.start)

  const due = rec ? firstOccurrence(rec.options, date, now) : (date?.due ?? null)

  return {
    title: buildTitle(text, spans),
    due,
    rrule: rec ? new RRule(rec.options).toString().replace(/^RRULE:/, '') : null,
    recurrenceText: rec ? new RRule(rec.options).toText() : null,
    spans,
  }
}

// ---------------------------------------------------------------- helpers

function mask(text: string, ranges: { start: number; end: number }[]): string {
  const chars = [...text]
  for (const { start, end } of ranges) for (let i = start; i < end && i < chars.length; i++) chars[i] = ' '
  return chars.join('')
}

const pad = (n: number) => String(n).padStart(2, '0')
const ymd = (y: number, m: number, d: number) => `${y}-${pad(m)}-${pad(d)}`
const escapeRe = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')

function parseHHMM(s: string): [number, number] {
  const [h, m] = s.split(':').map(Number)
  return [h ?? 0, m ?? 0]
}

/** Time words incl. aliases, lowercased → [hour, minute]. */
function timeWordMap(settings: Settings): Map<string, [number, number]> {
  const map = new Map<string, [number, number]>()
  for (const [w, t] of Object.entries(settings.timeWords)) map.set(w.toLowerCase(), parseHHMM(t))
  for (const [alias, target] of Object.entries(TIME_WORD_ALIASES)) {
    const t = map.get(target)
    if (t && !map.has(alias)) map.set(alias, t)
  }
  return map
}

function timeWordRegex(words: Iterable<string>): RegExp | null {
  const alts = [...words].sort((a, b) => b.length - a.length).map(escapeRe)
  return alts.length ? new RegExp(`\\b(${alts.join('|')})\\b`, 'i') : null
}

// ---------------------------------------------------------------- recurrence

type Recurrence = { start: number; end: number; options: Partial<RRuleOptions> }

const WEEKDAYS = [RRule.MO, RRule.TU, RRule.WE, RRule.TH, RRule.FR, RRule.SA, RRule.SU]
const WD = '(mon|tue|wed|thu|fri|sat|sun)[a-z]*'
const MON = '(jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec)[a-z]*'
const MONTHS = ['jan', 'feb', 'mar', 'apr', 'may', 'jun', 'jul', 'aug', 'sep', 'oct', 'nov', 'dec']
const ORD = '(first|1st|second|2nd|third|3rd|fourth|4th|last)'
const ORDINALS: Record<string, number> = { first: 1, '1st': 1, second: 2, '2nd': 2, third: 3, '3rd': 3, fourth: 4, '4th': 4, last: -1 }
const OF_MONTH = '(\\s+of\\s+(?:the\\s+|each\\s+|every\\s+)?month)?'
const DAY_NTH = '(\\d{1,2})(?:st|nd|rd|th)'

const weekday = (w: string) => WEEKDAYS[['mon', 'tue', 'wed', 'thu', 'fri', 'sat', 'sun'].indexOf(w.slice(0, 3).toLowerCase())]!
const month = (m: string) => MONTHS.indexOf(m.slice(0, 3).toLowerCase()) + 1
const validDay = (d: number) => d >= 1 && d <= 31

type Rule = [RegExp, (m: RegExpMatchArray) => Partial<RRuleOptions> | null]

/** Phrases RRule.fromText doesn't handle (or gets wrong). Matched right after "every". */
const CUSTOM_RULES: Rule[] = [
  [/^\s+(?:the\s+)?(?:start|beginning)\s+of\s+(?:the\s+|each\s+)?month\b/i, () => ({ freq: RRule.MONTHLY, bymonthday: [1] })],
  [/^\s+(?:the\s+)?(?:end|last\s+day)\s+of\s+(?:the\s+|each\s+)?month\b/i, () => ({ freq: RRule.MONTHLY, bymonthday: [-1] })],
  [/^\s+weekend\b/i, () => ({ freq: RRule.WEEKLY, byweekday: [RRule.SA, RRule.SU] })],
  [/^\s+fortnight\b/i, () => ({ freq: RRule.WEEKLY, interval: 2 })],
  [/^\s+quarter\b/i, () => ({ freq: RRule.MONTHLY, interval: 3 })],
  // "every year on march 3", "every 3rd of march", "every jan 1"
  [
    new RegExp(`^\\s+(?:year\\s+)?(?:on\\s+)?(?:${MON}\\s+(\\d{1,2})(?:st|nd|rd|th)?|(?:the\\s+)?(\\d{1,2})(?:st|nd|rd|th)?(?:\\s+of)?\\s+${MON})\\b`, 'i'),
    (m) => {
      const [mo, d] = m[1] ? [month(m[1]), Number(m[2])] : [month(m[4]!), Number(m[3])]
      return validDay(d) ? { freq: RRule.YEARLY, bymonth: [mo], bymonthday: [d] } : null
    },
  ],
  // "every 2nd tuesday" = every other week; "of the month" makes it monthly.
  [
    new RegExp(`^\\s+(?:month\\s+on\\s+)?(?:the\\s+)?${ORD}\\s+${WD}${OF_MONTH}\\b`, 'i'),
    (m) => {
      const n = ORDINALS[m[1]!.toLowerCase()]!
      const wd = weekday(m[2]!)
      if (n === 2 && !m[3] && !/month\s+on/i.test(m[0])) return { freq: RRule.WEEKLY, interval: 2, byweekday: [wd] }
      return { freq: RRule.MONTHLY, byweekday: [wd.nth(n)] }
    },
  ],
  // "every 15th", "every 1st of the month"
  [
    new RegExp(`^\\s+(?:the\\s+)?${DAY_NTH}${OF_MONTH}\\b`, 'i'),
    (m) => (validDay(Number(m[1])) ? { freq: RRule.MONTHLY, bymonthday: [Number(m[1])] } : null),
  ],
]

/** Single words that mean a repeat ("weekly review"), used when there's no "every". */
const ADVERBS: [RegExp, Partial<RRuleOptions>][] = [
  [/\bdaily\b/i, { freq: RRule.DAILY }],
  [/\b(?:fortnightly|biweekly|bi-weekly)\b/i, { freq: RRule.WEEKLY, interval: 2 }],
  [/\bweekly\b/i, { freq: RRule.WEEKLY }],
  [/\bmonthly\b/i, { freq: RRule.MONTHLY }],
  [/\bquarterly\b/i, { freq: RRule.MONTHLY, interval: 3 }],
  [/\b(?:yearly|annually)\b/i, { freq: RRule.YEARLY }],
]

const MAX_RULE_TOKENS = 8

function findRecurrence(text: string, settings: Settings): Recurrence | null {
  const timeWords = timeWordRegex(timeWordMap(settings).keys())
  for (const m of text.matchAll(/\bevery\b/gi)) {
    const start = m.index!
    const after = start + m[0].length
    const rest = text.slice(after)

    for (const [re, build] of CUSTOM_RULES) {
      const cm = rest.match(re)
      const options = cm && build(cm)
      if (options) return { start, end: after + cm[0].length, options }
    }

    // "every morning" → daily; the time word stays for the date parser.
    const tw = timeWords && rest.match(new RegExp(`^\\s+${timeWords.source}`, 'i'))
    if (tw) return { start, end: after, options: { freq: RRule.DAILY } }

    // RRule.fromText ignores trailing words, so grow the phrase token by token
    // and keep the shortest prefix that yields the final rule.
    let best: Recurrence | null = null
    let prev = ''
    let count = 0
    for (const t of rest.matchAll(/\S+/g)) {
      if (++count > MAX_RULE_TOKENS) break
      const end = after + t.index! + t[0].length
      const phrase = ('every' + rest.slice(0, end - after)).replace(/^every\s+other\b/i, 'every 2')
      const options = ruleFromText(phrase)
      if (!options) continue
      const key = new RRule(options).toString()
      if (key !== prev) {
        prev = key
        best = { start, end, options }
      }
    }
    if (best) return best
  }
  for (const [re, options] of ADVERBS) {
    const m = text.match(re)
    if (m) return { start: m.index!, end: m.index! + m[0].length, options }
  }
  return null
}

function ruleFromText(phrase: string): Partial<RRuleOptions> | null {
  try {
    // Times come from the date parser, not the rule.
    const { byhour: _h, byminute: _m, bysecond: _s, ...options } = RRule.parseText(phrase)
    return options.freq === undefined ? null : options
  } catch {
    return null
  }
}

/** First day a rule (RRULE body) falls on, at `time`, not before now. For the repeat builder. */
export function firstDue(rrule: string, now: Date, time: string | null): Due {
  const { dtstart: _d, ...options } = RRule.parseString(rrule)
  return firstOccurrence(options, time ? { start: 0, end: 0, dateCertain: false, due: { date: '', time } } : null, now)
}

/** First occurrence on/after now. Explicit start dates win. */
function firstOccurrence(options: Partial<RRuleOptions>, date: DateMatch | null, now: Date): Due {
  const time = date?.due.time ?? null
  if (date?.dateCertain) return date.due

  // RRule works in UTC; treat local wall-clock as fake UTC.
  const today = Date.UTC(now.getFullYear(), now.getMonth(), now.getDate())
  const nowWall = Date.UTC(now.getFullYear(), now.getMonth(), now.getDate(), now.getHours(), now.getMinutes())
  // The first matching day, not the first of an interval counted from today
  // ("every 2 weeks on monday" starts next Monday; the series anchors there).
  const rule = new RRule({ ...options, interval: 1, dtstart: new Date(today) })
  const [h, m] = time ? parseHHMM(time) : [0, 0]

  let cursor = new Date(today)
  let inclusive = true
  for (let i = 0; i < 1000; i++) {
    const d = rule.after(cursor, inclusive)
    if (!d) break
    if (!time || d.getTime() + (h * 60 + m) * 60_000 >= nowWall) {
      return { date: ymd(d.getUTCFullYear(), d.getUTCMonth() + 1, d.getUTCDate()), time }
    }
    cursor = d
    inclusive = false
  }
  return { date: ymd(now.getFullYear(), now.getMonth() + 1, now.getDate()), time }
}

// ---------------------------------------------------------------- dates

type DateMatch = { start: number; end: number; due: Due; dateCertain: boolean }

function findDate(text: string, now: Date, settings: Settings): DateMatch | null {
  const words = timeWordMap(settings)
  const wordRe = timeWordRegex(words.keys())
  const c = chrono.casual.clone()

  if (wordRe) {
    // Recognise custom words ("dawn") that chrono doesn't know.
    c.parsers.push({
      pattern: () => new RegExp(wordRe.source, 'i'),
      extract: (_ctx, match) => {
        const [hour, minute] = words.get(match[1]!.toLowerCase())!
        return { hour, minute }
      },
    })
    // Override chrono's own defaults (evening = 20:00) unless a clock time was given.
    c.refiners.push({
      refine: (_ctx, results) => {
        for (const r of results) {
          const w = r.text.match(wordRe)
          if (w && !r.start.isCertain('hour')) {
            const [hour, minute] = words.get(w[1]!.toLowerCase())!
            r.start.assign('hour', hour)
            r.start.assign('minute', minute)
            r.start.assign('second', 0)
          }
        }
        return results
      },
    })
  }

  // "21st", "on the 1st": next such day of the month (a month name after it is chrono's).
  c.parsers.push({
    pattern: () => /\b(?:the\s+)?(\d{1,2})(?:st|nd|rd|th)\b(?!\s+of\b)(?!\s+(?:jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec))/i,
    extract: (ctx, match) => {
      const day = Number(match[1])
      if (!validDay(day)) return null
      const ref = ctx.reference.instant
      let y = ref.getFullYear()
      let mo = ref.getMonth()
      // Skip past days and months too short for it (the 31st in November).
      for (let i = 0; i < 13; i++) {
        const fits = new Date(y, mo + 1, 0).getDate() >= day
        if (fits && (i > 0 || day >= ref.getDate())) break
        mo++
        if (mo > 11) [y, mo] = [y + 1, 0]
      }
      return { day, month: mo + 1, year: y }
    },
  })

  const [r] = c.parse(text, now, { forwardDate: true })
  if (!r) return null

  const s = r.start
  const dateCertain = s.isCertain('day') || s.isCertain('weekday')
  const timeKnown = s.isCertain('hour')
  let y = s.get('year')!
  let mo = s.get('month')!
  let d = s.get('day')!

  if (timeKnown && !dateCertain) {
    // Time only: next occurrence of that time, today or tomorrow.
    const h = s.get('hour')!
    const mi = s.get('minute') ?? 0
    const at = new Date(now.getFullYear(), now.getMonth(), now.getDate(), h, mi)
    if (at < now) at.setDate(at.getDate() + 1)
    ;[y, mo, d] = [at.getFullYear(), at.getMonth() + 1, at.getDate()]
  }

  // chrono can include leading whitespace (e.g. next to a masked range).
  const lead = r.text.length - r.text.trimStart().length
  return {
    start: r.index + lead,
    end: r.index + r.text.trimEnd().length,
    dateCertain,
    due: {
      date: ymd(y, mo, d),
      time: timeKnown ? `${pad(s.get('hour')!)}:${pad(s.get('minute') ?? 0)}` : null,
    },
  }
}

// ---------------------------------------------------------------- title

/** Dropped from the title when directly before a parsed span ("report by friday"). */
const CONNECTOR = /(?:^|\s)(?:on|at|by|due|before)\s+$/i

function buildTitle(text: string, spans: Span[]): string {
  let out = ''
  let pos = 0
  for (const s of spans) {
    let chunk = text.slice(pos, s.start)
    const m = chunk.match(CONNECTOR)
    if (m) chunk = chunk.slice(0, m.index)
    out += chunk + ' '
    pos = s.end
  }
  out += text.slice(pos)
  return out
    .replace(/\s+/g, ' ')
    .replace(/^[\s,:;–-]+|[\s,:;–-]+$/g, '')
}

export { EXAMPLES } from './examples'
