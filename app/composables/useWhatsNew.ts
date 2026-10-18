// "What's new" after an update. The notes are bundled at build time
// (public/whats-new.json, from .github/scripts/whats-new.mjs: the last 10
// releases' New / Fixes); dev builds have none. The first start of a newer
// version shows what changed since the last one seen on this device; a fresh
// install only remembers its version. Settings → "What's new" shows them all.
import { getVersion } from '@tauri-apps/api/app'

export type Release = { version: string; date: string; new: string[]; fixes: string[] }

const KEY = 'befikr.seenVersion'
/** Open sheet: releases newest first; `since` = the version updated from. */
const shown = ref<{ releases: Release[]; since: string | null } | null>(null)

/** -1 / 0 / 1 for "0.36.2"-style versions. */
const cmp = (a: string, b: string) => {
  const pa = a.split('.').map(Number)
  const pb = b.split('.').map(Number)
  for (let i = 0; i < 3; i++) if ((pa[i] ?? 0) !== (pb[i] ?? 0)) return (pa[i] ?? 0) < (pb[i] ?? 0) ? -1 : 1
  return 0
}

async function notes(): Promise<Release[]> {
  try {
    const res = await fetch('/whats-new.json')
    return res.ok ? await res.json() : []
  } catch {
    return []
  }
}

function remember(v: string) {
  try {
    localStorage.setItem(KEY, v)
  } catch {}
}

export function useWhatsNew() {
  /** After start: show what changed since the version seen last. */
  async function checkAfterUpdate() {
    const current = await getVersion().catch(() => '')
    if (!current) return
    let seen: string | null = null
    try {
      seen = localStorage.getItem(KEY)
    } catch {}
    remember(current)
    if (!seen || cmp(seen, current) >= 0) return
    const releases = (await notes()).filter((r) => cmp(r.version, seen!) > 0 && cmp(r.version, current) <= 0)
    if (releases.length) shown.value = { releases, since: seen }
  }

  async function openAll() {
    shown.value = { releases: await notes(), since: null }
  }

  return { shown, checkAfterUpdate, openAll, close: () => (shown.value = null) }
}
