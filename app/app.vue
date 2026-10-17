<script setup lang="ts">
import { addPluginListener } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

const router = useRouter()
const route = useRoute()
const { current: toast, dismiss } = useToast()
rememberHistoryStart()
// The splash stays until we know which screen opens (see useSplash).
const releaseSplash = holdSplash()

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
  // Desktop: a notification's Open / Snooze brings the window up on a route,
  // and its Complete changes the list.
  listen<string>('open-route', (e) => router.push(e.payload)).catch(() => {})
  listen('tasks-changed', () => listChanged()).catch(() => {})
  listen('quick-add', openQuickAdd).catch(() => {})
  window.addEventListener('keydown', quickAddKey)
  await openingScreen().catch(() => {})
  releaseSplash()
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

function runAction() {
  toast.value?.action?.run()
  dismiss()
}
</script>

<template>
  <NuxtPage />
  <RouteSheets />
  <ContextMenu />
  <Transition name="toast">
    <div v-if="toast" class="toast popover" role="status">
      <span>{{ toast.message }}</span>
      <button v-if="toast.action" class="btn" @click="runAction">{{ toast.action.label }}</button>
    </div>
  </Transition>
</template>

<style>
.toast {
  position: fixed; left: 16px; right: 16px; bottom: calc(24px + env(safe-area-inset-bottom));
  max-width: 480px; margin: 0 auto; z-index: 70;
  display: flex; align-items: center; justify-content: space-between; gap: 12px;
  padding: 8px 8px 8px 16px; border-radius: var(--radius);
}
.toast-enter-active, .toast-leave-active { transition: transform 0.25s cubic-bezier(0.22, 1, 0.36, 1), opacity 0.2s; }
.toast-enter-from, .toast-leave-to { transform: translateY(12px) scale(0.985); opacity: 0; }
</style>
