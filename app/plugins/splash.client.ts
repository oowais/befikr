// Lift the splash out of Nuxt's loader, which Nuxt drops as soon as the app
// mounts, so it can stay up until the first screen is ready (see useSplash).
// Never longer than a few seconds, whatever happens.
export default defineNuxtPlugin(() => {
  const splash = document.getElementById('splash')
  if (!splash) return
  document.body.appendChild(splash)
  document.documentElement.classList.add('booting')
  setTimeout(revealApp, 3000)
})
