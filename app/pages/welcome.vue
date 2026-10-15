<script setup lang="ts">
// Permission tour: one step at a time; each ticks off as Android reports it
// granted (polled, since Settings screens return without a callback).
import { AlarmClock, BatteryFull, Bell, Check } from 'lucide-vue-next'
import type { Status } from '~/composables/api'

const router = useRouter()
const status = ref<Status | null>(null)

const steps = computed(() => {
  const s = status.value
  return [
    {
      key: 'notifications',
      icon: Bell,
      title: 'Notifications',
      why: 'So reminders can appear at all.',
      ok: !!s?.notifications,
      allow: reminders.requestNotifications,
    },
    {
      key: 'exactAlarms',
      icon: AlarmClock,
      title: 'Exact alarms',
      why: 'So a 9:00 reminder comes at 9:00, not “around then”.',
      ok: !!s?.exactAlarms,
      allow: reminders.openExactAlarmSettings,
    },
    {
      key: 'battery',
      icon: BatteryFull,
      title: 'Unrestricted battery',
      why: 'So Android never puts Befikr to sleep and holds reminders back.',
      ok: !!s?.batteryUnrestricted,
      allow: reminders.openBatterySettings,
    },
  ]
})
const current = computed(() => steps.value.findIndex((s) => !s.ok))
const done = computed(() => status.value != null && current.value === -1)

async function refresh() {
  status.value = await reminders.status().catch(() => null)
}
let poll: ReturnType<typeof setInterval>
const onVisible = () => document.visibilityState === 'visible' && refresh()
onMounted(() => {
  refresh()
  poll = setInterval(refresh, 1000)
  document.addEventListener('visibilitychange', onVisible)
})
onUnmounted(() => {
  clearInterval(poll)
  document.removeEventListener('visibilitychange', onVisible)
})
watch(done, (d) => d && haptic())

function later() {
  skipSetup()
  router.replace('/')
}
</script>

<template>
  <main class="page welcome">
    <div class="hero">
      <svg viewBox="0 0 108 108" class="logo" aria-hidden="true">
        <rect width="108" height="108" rx="26" class="logo-bg" />
        <g transform="translate(54 54) scale(0.95) translate(-54 -54)" fill="none" stroke-linecap="round" stroke-linejoin="round">
          <path d="M26 38 l3.6 3.6 l7.2 -8 M43 38 q9 -7 18 0 t19 0" class="l1" stroke-width="5.5" />
          <path d="M26 54 l3.6 3.6 l7.2 -8 M43 54 q10 -8 21 0 t22 0" class="l2" stroke-width="6.5" />
          <circle cx="31.5" cy="70" r="4.8" class="l3" stroke-width="3.6" />
          <path d="M43 70 q8 -7 16.5 0 t16.5 0" class="l1" stroke-width="5.5" />
        </g>
      </svg>
      <h1>Befikr</h1>
      <p class="muted">Three permissions, so reminders never get lost.</p>
    </div>

    <ol class="pane rows steps">
      <li v-for="(s, i) in steps" :key="s.key" class="step" :class="{ active: i === current }">
        <span class="icon"><component :is="s.icon" :size="22" /></span>
        <span class="text">
          <b>{{ s.title }}</b>
          <span class="muted small">{{ s.why }}</span>
        </span>
        <span v-if="s.ok" class="checkbox checked"><Check :size="15" :stroke-width="3" /></span>
        <button v-else-if="i === current" class="btn primary" @click="s.allow().then(refresh)">Allow</button>
        <span v-else class="checkbox" />
      </li>
    </ol>

    <p v-if="status === null" class="muted small center">Permissions can only be set on the phone.</p>

    <div class="actions">
      <button v-if="done" class="btn primary wide" @click="router.replace('/')">All set — start</button>
      <template v-else>
        <span class="muted small">{{ Math.max(0, current) }} of 3 done</span>
        <button class="btn" @click="later">Later</button>
      </template>
    </div>
  </main>
</template>

<style scoped>
.welcome { display: flex; flex-direction: column; gap: 20px; padding-top: 6vh; }
.hero { text-align: center; }
.hero h1 { margin: 12px 0 4px; font-size: 1.8rem; letter-spacing: -0.02em; }
.logo { width: 88px; height: 88px; filter: drop-shadow(0 8px 18px var(--glass-shade)); }
.logo-bg { fill: #e3ecdc; }
.l1 { stroke: #7a9a76; }
.l2 { stroke: #24452a; }
.l3 { stroke: #e07a5f; }
@media (prefers-color-scheme: dark) {
  .logo-bg { fill: #23262b; }
  .l1 { stroke: #b9f6ca; }
  .l2 { stroke: #ffffff; }
  .l3 { stroke: #69f0ae; }
}
.steps { list-style: none; margin: 0; }
.step { display: flex; align-items: center; gap: 12px; padding: 14px 0; opacity: 0.55; }
.step.active, .step:has(.checked) { opacity: 1; }
.icon {
  width: 40px; height: 40px; border-radius: 12px; display: grid; place-items: center; flex: none;
  background: var(--glass-control);
  box-shadow: inset 0 1px 0 var(--glass-control-rim), inset 0 0 0 1px var(--glass-edge);
}
.text { flex: 1; display: flex; flex-direction: column; gap: 2px; }
.actions { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
.wide { flex: 1; }
.center { text-align: center; }
</style>
