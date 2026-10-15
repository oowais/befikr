// Quick add keeps its draft while the app runs: closing the sheet (tap
// outside, back) doesn't lose what was typed or picked. Add or Clear resets it.
const fresh = () => ({
  text: '',
  /** Parsed words the user chose to keep as text. */
  ignored: [] as string[],
  adaptive: false,
  expanded: false,
  manualDue: null as number | null,
  /** Rule from the repeat builder; undefined = use what was typed. */
  manualRule: undefined as string | null | undefined,
  description: '',
  /** Time picked in the "no time given" prompt. */
  chosenTime: null as string | null,
})

const draft = reactive(fresh())

export const useQuickAddDraft = () => draft
export const clearQuickAddDraft = () => Object.assign(draft, fresh())

/** Set to open quick add on the list (desktop tray: "Quick add"); the list clears it. */
export const quickAddPending = ref(false)

/**
 * Fill the draft from "Share → Befikr": the page title (or the text around a
 * link) becomes the task, the link goes into notes. Replaces any draft.
 */
export function draftFromShare(text: string, subject: string) {
  const url = text.match(/https?:\/\/\S+/)?.[0] ?? ''
  const rest = text.replace(url, '').trim()
  const title = subject.trim() || rest || url || text.trim()
  const notes = title === text.trim() ? '' : url || text.trim()
  clearQuickAddDraft()
  Object.assign(draft, { text: title, description: notes, expanded: !!notes })
}
