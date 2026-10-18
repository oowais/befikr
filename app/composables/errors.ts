// Errors as the user reads them. Commands fail with raw text from Rust
// ("invalid: title is empty", "not found: <id>", "database: …"): known ones
// get a plain sentence, already-friendly ones (sync, backup folder) pass
// through, the rest become "Couldn't <doing>. Try again." The raw text
// goes to the console.

const KNOWN: [RegExp, string][] = [
  [/title is empty/, 'Add a title first.'],
  [/name is empty/, 'Add a name first.'],
  [/name must be 1–40 characters/, 'Use a name of 1 to 40 characters.'],
  [/^not found:/, 'This task was deleted, maybe on another device.'],
  [/nothing to reopen|task is not completed/, 'This task is already open.'],
  [/no later occurrence to skip to/, 'That was the last repeat; nothing to skip to.'],
  [/only open repeating tasks can be skipped/, 'Only open repeating tasks can be skipped.'],
  [/only open repeating tasks can be retired/, 'Only open repeating tasks can be stopped.'],
  [/only open top-level tasks can be snoozed/, 'Only open tasks can be snoozed.'],
  [/recurring tasks need a due date/, 'A repeat needs a date. Pick one first.'],
  [/rrule/, 'That repeat isn’t valid.'],
  [/subtasks cannot have subtasks/, 'Subtasks can’t have their own subtasks.'],
  [/subtasks follow their parent's date/, 'Subtasks follow their parent’s date.'],
  [/time must be HH:MM/, 'Use a time like 18:00.'],
  [/time word must be letters/, 'Time words use letters only.'],
  [/re-remind interval must be positive/, 'Pick a re-remind time above zero.'],
  [/not a database file|not a Befikr backup/, 'That file isn’t a Befikr backup.'],
  [/Backup folder is missing/i, 'The backup folder is missing. Choose it again.'],
  [/backup is from a newer version/, 'This backup is from a newer Befikr. Update the app first.'],
  [/No speech input app/, 'No speech input app on this phone.'],
  [/^database:/, 'Couldn’t save. Try again.'],
]

/** Raw text from Rust, still to be worded. */
const RAW = /^(invalid|not found|database|migration|sync):/

/** `doing`: what failed, for the fallback ("save the task"). */
export function friendlyError(e: unknown, doing = 'do that'): string {
  const raw = String(e instanceof Error ? e.message : e).trim()
  console.warn(`${doing}: ${raw}`)
  for (const [re, text] of KNOWN) if (re.test(raw)) return text
  // A full sentence from us (sync, backup folder, newer backup…).
  const text = raw.replace(/^invalid: /, '')
  if (!RAW.test(text) && /^[A-Z]/.test(text) && /[.!?]$/.test(text)) return text
  return `Couldn’t ${doing}. Try again.`
}

/** A failed automatic backup ("Failed: …" on the Backup page). */
export function friendlyBackupError(raw: string): string {
  if (/permission|denied|os error|no such file|read-only|EACCES|ENOENT|SecurityException/i.test(raw)) {
    return 'Couldn’t write to the backup folder. Choose it again.'
  }
  return friendlyError(raw, 'back up')
}
