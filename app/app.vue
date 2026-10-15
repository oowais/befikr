<script setup lang="ts">
import { addPluginListener } from '@tauri-apps/api/core'

const router = useRouter()
const route = useRoute()
const { current: toast, dismiss } = useToast()

onMounted(async () => {
  loadTimeWords()
  // Each step on its own: one failing (desktop/browser has no reminders
  // plugin) must not skip the others.
  // Notification "Snooze" opens the app on a route (cold or warm start).
  addPluginListener('reminders', 'launchRoute', (e: { route: string }) => router.push(e.route)).catch(() => {})
  const launch = await reminders.takeLaunchRoute().then((r) => r.route, () => undefined)
  if (launch && launch !== '/') return router.push(launch)
  // First launch, or a permission revoked: the setup tour.
  if (route.path === '/' && (await needsSetup().catch(() => false))) router.replace('/welcome')
})

function runAction() {
  toast.value?.action?.run()
  dismiss()
}
</script>

<template>
  <NuxtPage />
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
