<script setup lang="ts">
import { addPluginListener } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

const router = useRouter()
const route = useRoute()
rememberHistoryStart()
// The splash stays until we know which screen opens (see useSplash).
const releaseSplash = holdSplash()
const whatsNew = useWhatsNew()

onMounted(async () => {
  // Desktop: our own right-click menus only (see useContextMenu).
  document.addEventListener('contextmenu', suppressDefaultMenu)
  loadTimeWords()
  useUpdate().check().catch(() => {})
  // Long-running desktop windows: the check's own limits keep this cheap.
  window.addEventListener('focus', () => useUpdate().check().catch(() => {}))
  handleBackButton().catch(() => {})
  // Each step on its own: one failing (desktop/browser has no reminders
  // plugin) must not skip the others.
  // Notification "Snooze" opens the app on a route (cold or warm start).
  addPluginListener('reminders', 'launchRoute', (e: { route: string }) => router.push(e.route)).catch(() => {})
  // Android: Complete/Skip from a notification or the widget, or a background
  // sync, changed tasks while the app was open.
  addPluginListener('reminders', 'tasksChanged', () => listChanged()).catch(() => {})
  // Desktop: a notification's Open / Snooze brings the window up on a route,
  // and its Complete changes the list.
  listen<string>('open-route', (e) => router.push(e.payload)).catch(() => {})
  listen('tasks-changed', () => listChanged()).catch(() => {})
  listen('quick-add', openQuickAdd).catch(() => {})
  window.addEventListener('keydown', quickAddKey)
  await openingScreen().catch(() => {})
  releaseSplash()
  // After an update, on the list (not over the setup tour or a notification's screen).
  if (route.path === '/') whatsNew.checkAfterUpdate()
})

async function openingScreen() {
  const launch = await reminders.takeLaunchRoute().then((r) => r?.route, () => undefined)
  if (launch && launch !== '/') return router.push(launch)
  // First launch, or a permission revoked: the setup tour.
  if (route.path === '/' && (await needsSetup().catch(() => false))) await router.replace('/welcome')
}

// Quick add from the tray menu or the keyboard: the list (or the calendar,
// which has its own) opens it.
async function openQuickAdd() {
  quickAddPending.value = true
  const path = route.path.replace(/(.)\/$/, '$1')
  if (path !== '/' && path !== '/calendar') await router.push('/')
}

/** N or Ctrl+N (computers), unless typing or a sheet is already open. */
function quickAddKey(e: KeyboardEvent) {
  if (e.key.toLowerCase() !== 'n' || e.altKey || e.shiftKey || e.repeat) return
  const el = e.target as HTMLElement | null
  if (el?.closest('input, textarea, select, [contenteditable="true"]') || sheetOpen()) return
  e.preventDefault()
  openQuickAdd()
}
</script>

<template>
  <NuxtPage />
  <RouteSheets />
  <ContextMenu />
  <WhatsNewSheet
    v-if="whatsNew.shown.value"
    :releases="whatsNew.shown.value.releases"
    :since="whatsNew.shown.value.since"
    @close="whatsNew.close"
  />
  <AppToast />
</template>
