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
