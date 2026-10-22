type Toast = { message: string; action?: { label: string; run: () => void } }

const current = ref<Toast | null>(null)
let timer: ReturnType<typeof setTimeout> | undefined

export function useToast() {
  function show(t: Toast, ms = 5000) {
    clearTimeout(timer)
    current.value = t
    timer = setTimeout(() => (current.value = null), ms)
  }
  // While a finger holds it (swiping), it doesn't time out underneath.
  function pause() {
    clearTimeout(timer)
  }
  function resume(ms = 3000) {
    clearTimeout(timer)
    timer = setTimeout(() => (current.value = null), ms)
  }
  function dismiss() {
    clearTimeout(timer)
    current.value = null
  }
  return { current, show, dismiss, pause, resume }
}
