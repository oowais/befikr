// Example phrases for the "New task" bar: one faint hint at a time, so the
// parser's tricks get noticed without anything to read or dismiss.
import { EXAMPLES } from '@befikr/parser'

const index = ref(Math.floor(Math.random() * EXAMPLES.length))

/** The hint on screen now; quick add uses it as its placeholder too. */
export const useHint = () => computed(() => EXAMPLES[index.value]!.text)

/**
 * Rotate while the home screen is up: a new phrase every 9 s, paused while
 * the app is hidden. With reduced motion the phrase only changes between
 * visits. Call from a component's setup.
 */
export function useHintRotation() {
  let timer: ReturnType<typeof setInterval> | undefined
  const next = () => (index.value = (index.value + 1) % EXAMPLES.length)
  const still = () => matchMedia('(prefers-reduced-motion: reduce)').matches
  const start = () => {
    stop()
    if (!still() && document.visibilityState === 'visible') timer = setInterval(next, 9_000)
  }
  const stop = () => timer && (clearInterval(timer), (timer = undefined))
  const onVisible = () => (document.visibilityState === 'visible' ? start() : stop())
  onMounted(() => {
    next()
    start()
    document.addEventListener('visibilitychange', onVisible)
  })
  onUnmounted(() => {
    stop()
    document.removeEventListener('visibilitychange', onVisible)
  })
}
