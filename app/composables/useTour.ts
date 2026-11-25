// The app tour (`/tour`): shown once, on a first launch with an empty list.
// Anyone who already has tasks counts as having seen it. Reopen from Settings.

const SEEN_KEY = 'befikr.tourSeen'

function seen(): boolean {
  try {
    return localStorage.getItem(SEEN_KEY) != null
  } catch {
    return true
  }
}

export function markTourSeen() {
  try {
    localStorage.setItem(SEEN_KEY, String(Date.now()))
  } catch {
    /* shows again next launch */
  }
}

export async function needsTour(): Promise<boolean> {
  if (seen()) return false
  const rows = await api.listOpen()
  if (rows.length === 0) return true
  markTourSeen()
  return false
}
