// Cold start: the splash (spa-loading-template.html) stays up until the first
// screen has what it shows, then fades while the page rises in behind it, in
// one move rather than parts popping in. Whatever loads for the first screen
// holds the splash (`holdSplash()`) and releases it when its data is in.
// Holds taken after the reveal do nothing.

let holds = 0
let revealed = false

/** Keep the splash up until the returned function is called. */
export function holdSplash(): () => void {
  if (revealed) return () => {}
  holds++
  let released = false
  return () => {
    if (released) return
    released = true
    if (--holds === 0) settle()
  }
}

// Releasing can mount more parts that hold in turn (the list → the week
// recap): wait for that render and a frame before deciding.
function settle() {
  nextTick(() => requestAnimationFrame(() => holds === 0 && revealApp()))
}

/** Fade the splash out and let the page in. Also the safety net (plugins/splash.client.ts). */
export function revealApp() {
  if (revealed) return
  revealed = true
  const root = document.documentElement
  root.classList.remove('booting')
  const splash = document.getElementById('splash')
  if (!splash) return
  root.classList.add('revealing')
  splash.classList.add('out')
  setTimeout(() => {
    splash.remove()
    root.classList.remove('revealing')
  }, 520)
}
