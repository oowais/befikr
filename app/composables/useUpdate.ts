// In-app updates (issue #7). Checks GitHub's latest release at most once a
// day (or on "Check for updates"); the result is kept so the Settings dot and
// row survive restarts. Android downloads the APK and hands it to the system
// installer (same signing key required); Windows uses the signed Tauri
// updater; Linux only says a new version is out (packages update it).
import { getVersion } from '@tauri-apps/api/app'
import { invoke } from '@tauri-apps/api/core'

const LATEST_URL = 'https://api.github.com/repos/oowais/befikr/releases/latest'
const DAY_MS = 86_400_000
const KEY = 'befikr.update'

type Latest = { version: string; apkUrl: string | null; checkedAt: number }

const isWindows = typeof navigator !== 'undefined' && /Windows/i.test(navigator.userAgent)
const current = ref('')
const latest = ref<Latest | null>(load())
const checking = ref(false)
const installing = ref(false)

function load(): Latest | null {
  try {
    return JSON.parse(localStorage.getItem(KEY) ?? 'null')
  } catch {
    return null
  }
}

/** a > b for "0.x.y" versions. */
export function newer(a: string, b: string) {
  const pa = a.split('.').map(Number)
  const pb = b.split('.').map(Number)
  for (let i = 0; i < 3; i++) if ((pa[i] ?? 0) !== (pb[i] ?? 0)) return (pa[i] ?? 0) > (pb[i] ?? 0)
  return false
}

export function useUpdate() {
  const available = computed(() =>
    current.value && latest.value && newer(latest.value.version, current.value) ? latest.value.version : null,
  )
  const canInstall = computed(() => !!available.value && (isAndroid ? !!latest.value?.apkUrl : isWindows))

  /** Fetches the latest release; without `force`, at most once a day. */
  async function check(force = false) {
    current.value ||= await getVersion().catch(() => '')
    if (!current.value || checking.value) return
    if (!force && latest.value && Date.now() - latest.value.checkedAt < DAY_MS) return
    checking.value = true
    try {
      const res = await fetch(LATEST_URL, { headers: { Accept: 'application/vnd.github+json' } })
      if (!res.ok) throw new Error(`GitHub: HTTP ${res.status}`)
      const r = (await res.json()) as { tag_name: string; assets: { name: string; browser_download_url: string }[] }
      latest.value = {
        version: r.tag_name.replace(/^v/, ''),
        apkUrl: r.assets.find((a) => a.name === 'befikr.apk')?.browser_download_url ?? null,
        checkedAt: Date.now(),
      }
      try {
        localStorage.setItem(KEY, JSON.stringify(latest.value))
      } catch {}
    } finally {
      checking.value = false
    }
  }

  /** Android: `needsPermission` = "Install unknown apps" was opened; tap again after allowing. */
  async function install(): Promise<{ needsPermission?: boolean }> {
    if (!canInstall.value || installing.value) return {}
    installing.value = true
    try {
      if (isAndroid) return await invoke('plugin:reminders|install_update', { url: latest.value!.apkUrl })
      await invoke<boolean>('install_update')
      return {}
    } finally {
      installing.value = false
    }
  }

  return { current, available, canInstall, checking, installing, check, install }
}
