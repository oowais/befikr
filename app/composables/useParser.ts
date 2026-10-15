// Quick-add parsing with the user's time words from settings.
import { DEFAULT_SETTINGS, parse, type ParseResult } from '@befikr/parser'
import { api } from './api'

const timeWords = ref<Record<string, string>>({ ...DEFAULT_SETTINGS.timeWords })
let loaded = false

/** Time words from settings, e.g. { morning: '09:00' } (defaults until loaded). */
export const useTimeWords = () => timeWords

export async function loadTimeWords(force = false) {
  if (loaded && !force) return
  try {
    timeWords.value = (await api.settings()).timeWords
    loaded = true
  } catch {
    /* keep defaults (e.g. outside Tauri) */
  }
}

/**
 * Parse `text`, keeping `ignored` substrings literal (the user tapped a
 * chip to un-parse it). Substrings rather than ranges survive edits.
 */
export function parseWith(text: string, ignored: string[], now = new Date()): ParseResult {
  const ignore = ignored.flatMap((s) => {
    const i = text.toLowerCase().indexOf(s.toLowerCase())
    return i >= 0 ? [{ start: i, end: i + s.length }] : []
  })
  return parse(text, now, { timeWords: timeWords.value }, { ignore })
}
