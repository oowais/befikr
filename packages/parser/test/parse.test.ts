import { describe, expect, it } from 'vitest'
import { DEFAULT_SETTINGS, parse, type ParseResult } from '../src/index'

// Thursday 2026-10-15 10:00 local (tests run with TZ=UTC).
const NOW = new Date(2026, 9, 15, 10, 0)

type Case = {
  input: string
  title: string
  due?: string | null // 'YYYY-MM-DD' or 'YYYY-MM-DD HH:MM'
  rrule?: string | null
  timeWords?: Record<string, string>
  ignore?: [number, number][]
}

const fmtDue = (r: ParseResult) => (r.due ? (r.due.time ? `${r.due.date} ${r.due.time}` : r.due.date) : null)

const CASES: Case[] = [
  // Plain
  { input: 'buy milk', title: 'buy milk', due: null },
  { input: '', title: '', due: null },
  { input: 'everyone meeting', title: 'everyone meeting', due: null },
  { input: 'every banana', title: 'every banana', due: null },

  // Dates and times
  { input: 'buy milk tomorrow', title: 'buy milk', due: '2026-10-16' },
  { input: 'buy milk tomorrow evening', title: 'buy milk', due: '2026-10-16 18:00' },
  { input: 'call mom at 5pm', title: 'call mom', due: '2026-10-15 17:00' },
  { input: 'call mom at 9am', title: 'call mom', due: '2026-10-16 09:00' },
  { input: 'dentist fri 3pm', title: 'dentist', due: '2026-10-16 15:00' },
  { input: 'report by friday', title: 'report', due: '2026-10-16' },
  { input: 'gym morning', title: 'gym', due: '2026-10-16 09:00' },
  { input: 'meet tonight', title: 'meet', due: '2026-10-15 21:00' },
  { input: 'call bob 17:30', title: 'call bob', due: '2026-10-15 17:30' },
  { input: 'stretch in 2 hours', title: 'stretch', due: '2026-10-15 12:00' },
  { input: 'lunch tomorrow at noon', title: 'lunch', due: '2026-10-16 12:00' },
  { input: 'tomorrow afternoon: submit form', title: 'submit form', due: '2026-10-16 14:00' },
  { input: 'tomorrow morning at 7 run', title: 'run', due: '2026-10-16 07:00' },

  // Time words from settings
  { input: 'walk tomorrow dawn', title: 'walk', due: '2026-10-16 05:30', timeWords: { dawn: '05:30' } },
  { input: 'buy milk tomorrow evening', title: 'buy milk', due: '2026-10-16 19:15', timeWords: { evening: '19:15' } },

  // Recurrence
  { input: 'water plants every 3 days', title: 'water plants', due: '2026-10-15', rrule: 'INTERVAL=3;FREQ=DAILY' },
  { input: 'every day read', title: 'read', due: '2026-10-15', rrule: 'FREQ=DAILY' },
  { input: 'Every Monday team sync', title: 'team sync', due: '2026-10-19', rrule: 'FREQ=WEEKLY;BYDAY=MO' },
  { input: 'standup every weekday 9am', title: 'standup', due: '2026-10-16 09:00', rrule: 'FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR' },
  { input: 'bins every monday evening', title: 'bins', due: '2026-10-19 18:00', rrule: 'FREQ=WEEKLY;BYDAY=MO' },
  { input: 'vitamins every morning', title: 'vitamins', due: '2026-10-16 09:00', rrule: 'FREQ=DAILY' },
  { input: 'rent every start of the month', title: 'rent', due: '2026-11-01', rrule: 'FREQ=MONTHLY;BYMONTHDAY=1' },
  { input: 'review budget every end of month', title: 'review budget', due: '2026-10-31', rrule: 'FREQ=MONTHLY;BYMONTHDAY=-1' },
  { input: 'pay rent every month on the 1st', title: 'pay rent', due: '2026-11-01', rrule: 'FREQ=MONTHLY;BYMONTHDAY=1' },
  { input: 'haircut every other week', title: 'haircut', due: '2026-10-15', rrule: 'INTERVAL=2;FREQ=WEEKLY' },
  { input: 'call grandma every weekend', title: 'call grandma', due: '2026-10-17', rrule: 'FREQ=WEEKLY;BYDAY=SA,SU' },
  { input: 'gym every monday and thursday 7am', title: 'gym', due: '2026-10-19 07:00', rrule: 'FREQ=WEEKLY;BYDAY=MO,TH' },
  { input: 'gym every monday and thursday 6pm', title: 'gym', due: '2026-10-15 18:00', rrule: 'FREQ=WEEKLY;BYDAY=MO,TH' },

  // Un-parse: user tapped the highlight to keep it literal
  { input: 'watch Friday Night Lights', title: 'watch Lights', due: '2026-10-16 21:00' },
  { input: 'watch Friday Night Lights', title: 'watch Friday Night Lights', due: null, ignore: [[6, 18]] },
  { input: 'read every day', title: 'read every day', due: null, rrule: null, ignore: [[5, 14]] },
]

describe('parse', () => {
  for (const c of CASES) {
    const label = `${JSON.stringify(c.input)}${c.ignore ? ' (ignore ' + JSON.stringify(c.ignore) + ')' : ''}${c.timeWords ? ' ' + JSON.stringify(c.timeWords) : ''}`
    it(label, () => {
      const settings = { ...DEFAULT_SETTINGS, timeWords: { ...DEFAULT_SETTINGS.timeWords, ...c.timeWords } }
      const r = parse(c.input, NOW, settings, { ignore: c.ignore?.map(([start, end]) => ({ start, end })) })
      expect({ title: r.title, due: fmtDue(r), rrule: r.rrule }).toEqual({
        title: c.title,
        due: c.due ?? null,
        rrule: c.rrule ?? null,
      })
    })
  }
})

describe('spans', () => {
  it('marks date and recurrence ranges in the original text', () => {
    const text = 'bins every monday evening'
    const r = parse(text, NOW)
    expect(r.spans.map((s) => [s.kind, text.slice(s.start, s.end)])).toEqual([
      ['recurrence', 'every monday'],
      ['date', 'evening'],
    ])
  })

  it('excludes connector words from the highlight but not the title strip', () => {
    const text = 'report by friday'
    const r = parse(text, NOW)
    expect(r.spans.map((s) => text.slice(s.start, s.end))).toEqual(['friday'])
    expect(r.title).toBe('report')
  })
})

describe('recurrence text', () => {
  it('gives a human label for the chip', () => {
    expect(parse('bins every monday', NOW).recurrenceText).toBe('every week on Monday')
    expect(parse('rent every start of the month', NOW).recurrenceText).toBe('every month on the 1st')
  })
})
