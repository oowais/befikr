import { describe, expect, it } from 'vitest'
import { DEFAULT_SETTINGS, EXAMPLES, firstDue, parse, type ParseResult } from '../src/index'

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
  { input: 'get the money', title: 'get the money', due: null },
  { input: 'buy a day pass', title: 'buy a day pass', due: null },
  { input: 'get the money tomorrow', title: 'get the money', due: '2026-10-16' },

  // Dates and times
  { input: 'buy milk tomorrow', title: 'buy milk', due: '2026-10-16' },
  { input: 'buy milk tomorrow evening', title: 'buy milk', due: '2026-10-16 18:00' },
  { input: 'buy milk tomo', title: 'buy milk', due: '2026-10-16' },
  { input: 'call mom tomo 5pm', title: 'call mom', due: '2026-10-16 17:00' },
  { input: 'tomoko lunch', title: 'tomoko lunch', due: null },
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

  // Day of month
  { input: 'pay bills on the 1st', title: 'pay bills', due: '2026-11-01' },
  { input: 'pay bills on the 15th', title: 'pay bills', due: '2026-10-15' },
  { input: 'dentist on the 3rd at 4pm', title: 'dentist', due: '2026-11-03 16:00' },
  { input: 'gift on the 31st', title: 'gift', due: '2026-10-31' },
  { input: 'pay rent 21st', title: 'pay rent', due: '2026-10-21' },
  { input: 'pay rent 3rd', title: 'pay rent', due: '2026-11-03' },
  { input: 'call 28th', title: 'call', due: '2026-10-28' },
  { input: 'pay rent on 21st', title: 'pay rent', due: '2026-10-21' },
  { input: 'pay rent 21st 5pm', title: 'pay rent', due: '2026-10-21 17:00' },
  { input: 'pay rent 21st at 5pm', title: 'pay rent', due: '2026-10-21 17:00' },
  { input: 'pay rent 21st march', title: 'pay rent', due: '2027-03-21' },
  { input: 'pay rent 3rd of november', title: 'pay rent', due: '2026-11-03' },
  { input: 'buy 21 eggs', title: 'buy 21 eggs', due: null },

  // Monthly / yearly patterns
  { input: 'team call every last friday', title: 'team call', due: '2026-10-30', rrule: 'FREQ=MONTHLY;BYDAY=-1FR' },
  { input: 'every first monday book club', title: 'book club', due: '2026-11-02', rrule: 'FREQ=MONTHLY;BYDAY=+1MO' },
  { input: 'meetup every second tuesday of the month', title: 'meetup', due: '2026-11-10', rrule: 'FREQ=MONTHLY;BYDAY=+2TU' },
  { input: 'bins every 2nd tuesday', title: 'bins', due: '2026-10-20', rrule: 'FREQ=WEEKLY;INTERVAL=2;BYDAY=TU' },
  { input: 'invoice every 15th', title: 'invoice', due: '2026-10-15', rrule: 'FREQ=MONTHLY;BYMONTHDAY=15' },
  { input: 'backup every last day of the month', title: 'backup', due: '2026-10-31', rrule: 'FREQ=MONTHLY;BYMONTHDAY=-1' },
  { input: 'birthday every year on march 3', title: 'birthday', due: '2027-03-03', rrule: 'FREQ=YEARLY;BYMONTH=3;BYMONTHDAY=3' },
  { input: 'anniversary every 3rd of march', title: 'anniversary', due: '2027-03-03', rrule: 'FREQ=YEARLY;BYMONTH=3;BYMONTHDAY=3' },
  { input: 'clean every fortnight', title: 'clean', due: '2026-10-15', rrule: 'FREQ=WEEKLY;INTERVAL=2' },
  { input: 'review every quarter', title: 'review', due: '2026-10-15', rrule: 'FREQ=MONTHLY;INTERVAL=3' },
  { input: 'sync every 2 weeks on monday', title: 'sync', due: '2026-10-19', rrule: 'INTERVAL=2;FREQ=WEEKLY;BYDAY=MO' },
  { input: 'weekly review', title: 'review', due: '2026-10-15', rrule: 'FREQ=WEEKLY' },
  { input: 'biweekly sync', title: 'sync', due: '2026-10-15', rrule: 'FREQ=WEEKLY;INTERVAL=2' },
  { input: 'daily standup 9:30', title: 'standup', due: '2026-10-16 09:30', rrule: 'FREQ=DAILY' },

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

describe('hint examples', () => {
  for (const e of EXAMPLES) {
    it(e.text, () => {
      const r = parse(e.text, NOW)
      expect(r.title).toBe(e.title)
      expect(r.due).not.toBeNull()
      expect(e.text.length).toBeLessThanOrEqual(28)
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

describe('firstDue', () => {
  it('finds the first matching day for a rule', () => {
    expect(firstDue('FREQ=WEEKLY;BYDAY=MO', NOW, '09:00')).toEqual({ date: '2026-10-19', time: '09:00' })
    expect(firstDue('FREQ=MONTHLY;BYDAY=-1FR', NOW, null)).toEqual({ date: '2026-10-30', time: null })
    expect(firstDue('FREQ=DAILY', NOW, '08:00')).toEqual({ date: '2026-10-16', time: '08:00' })
  })
})
