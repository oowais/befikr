<script setup lang="ts">
import { addPluginListener } from '@tauri-apps/api/core'

const router = useRouter()
const { current: toast, dismiss } = useToast()

// Notification "Snooze" opens the app on a route (cold or warm start).
onMounted(async () => {
  loadTimeWords()
  try {
    const { route } = await reminders.takeLaunchRoute()
    if (route && route !== '/') router.push(route)
    await addPluginListener('reminders', 'launchRoute', (e: { route: string }) => router.push(e.route))
  } catch {
    /* desktop / browser: no reminders plugin */
  }
})
</script>

<template>
  <NuxtPage />
  <Transition name="toast">
    <div v-if="toast" class="toast" role="status">
      <span>{{ toast.message }}</span>
      <button
        v-if="toast.action"
        class="toast-action"
        @click="
          () => {
            toast!.action!.run()
            dismiss()
          }
        "
      >
        {{ toast.action.label }}
      </button>
    </div>
  </Transition>
</template>

<style>
.toast {
  position: fixed; left: 16px; right: 16px; bottom: calc(24px + env(safe-area-inset-bottom));
  max-width: 480px; margin: 0 auto; z-index: 50;
  display: flex; align-items: center; justify-content: space-between; gap: 12px;
  padding: 12px 16px; border-radius: var(--radius);
  background: #2b2b2e; color: #fff; box-shadow: var(--shadow);
}
.toast-action { color: #9be29f; font-weight: 600; padding: 4px 8px; }
.toast-enter-active, .toast-leave-active { transition: transform 0.25s cubic-bezier(0.2, 0.9, 0.3, 1.2), opacity 0.2s; }
.toast-enter-from, .toast-leave-to { transform: translateY(20px); opacity: 0; }
</style>
