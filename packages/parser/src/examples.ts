/**
 * Phrases shown as hints in the app's "New task" bar: one per thing the
 * parser understands. Each is checked in the test table (title + due/repeat),
 * so a hint never shows something that doesn't work. Short enough to fit
 * the bar on a phone (28 characters at most).
 */
export const EXAMPLES: { text: string; title: string }[] = [
  { text: 'call mom fri 5pm', title: 'call mom' },
  { text: 'buy milk tomorrow evening', title: 'buy milk' },
  { text: 'stretch in 2 hours', title: 'stretch' },
  { text: 'water plants every 3 days', title: 'water plants' },
  { text: 'standup every weekday 9am', title: 'standup' },
  { text: 'bins every monday evening', title: 'bins' },
  { text: 'invoice every 15th', title: 'invoice' },
  { text: 'gym every mon and thu 7am', title: 'gym' },
  { text: 'team call every last friday', title: 'team call' },
  { text: 'birthday every 3rd of march', title: 'birthday' },
  { text: 'haircut every other week', title: 'haircut' },
  { text: 'vitamins every morning', title: 'vitamins' },
]
