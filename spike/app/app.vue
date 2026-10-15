<script setup lang="ts">
import { invoke, addPluginListener } from '@tauri-apps/api/core'
import { sendNotification, Schedule } from '@tauri-apps/plugin-notification'

type Task = { id: string; title: string; dueAt: number; nextFireAt: number | null; fires: number; alarmMode: string; completedAt: number | null }
type Event = { taskId: string; title: string | null; kind: string; at: number; source: string }
type Status = { notifications: boolean; exactAlarms: boolean; batteryUnrestricted: boolean; sdk: number; device: string }

const status = ref<Status | null>(null)
const tasks = ref<Task[]>([])
const events = ref<Event[]>([])
const error = ref('')
const title = ref('Test reminder')
const inSeconds = ref(120)
const mode = ref<'exact' | 'alarmClock'>('exact')
const snoozeId = ref<string | null>(null)

const fmt = (ms: number | null) => (ms ? new Date(ms).toLocaleTimeString() : '—')
const snoozeTask = computed(() => tasks.value.find((t) => t.id === snoozeId.value))

async function run<T>(fn: () => Promise<T>) {
  error.value = ''
  try {
    return await fn()
  } catch (e) {
    error.value = String(e)
  }
}

async function refresh() {
  await run(async () => {
    status.value = await invoke<Status>('plugin:reminders|status')
    tasks.value = await invoke<Task[]>('list_tasks')
    events.value = await invoke<Event[]>('list_events')
  })
}

const plugin = (cmd: string) => run(() => invoke(`plugin:reminders|${cmd}`))

async function create() {
  await run(() => invoke('create_task', { title: title.value, inSeconds: inSeconds.value, mode: mode.value }))
  await refresh()
}

async function complete(id: string) {
  await run(() => invoke('complete_task', { id }))
  await refresh()
}

async function snooze(id: string, secs: number) {
  await run(() => invoke('snooze_task', { id, inSeconds: secs }))
  snoozeId.value = null
  await refresh()
}

// Comparison: official plugin's own scheduling.
async function official() {
  await run(async () => {
    sendNotification({
      id: Math.floor(Math.random() * 1e6),
      title: `Official plugin (${inSeconds.value}s)`,
      schedule: Schedule.at(new Date(Date.now() + inSeconds.value * 1000), false, true),
    })
  })
}

function openRoute(route: string | null | undefined) {
  const m = route?.match(/^\/snooze\?id=(.+)$/)
  if (m) snoozeId.value = m[1]
}

onMounted(async () => {
  await refresh()
  await run(async () => {
    openRoute((await invoke<{ route?: string }>('plugin:reminders|take_launch_route')).route)
    await addPluginListener('reminders', 'launchRoute', (e: { route: string }) => {
      openRoute(e.route)
      refresh()
    })
  })
  document.addEventListener('visibilitychange', () => document.visibilityState === 'visible' && refresh())
  setInterval(refresh, 5000)
})
</script>

<template>
  <main>
    <h1>Befikr spike</h1>
    <p v-if="error" class="err">{{ error }}</p>

    <section v-if="snoozeId" class="sheet">
      <h2>Snooze: {{ snoozeTask?.title ?? snoozeId }}</h2>
      <button @click="snooze(snoozeId, 60)">+1 min</button>
      <button @click="snooze(snoozeId, 120)">+2 min</button>
      <button @click="snooze(snoozeId, 600)">+10 min</button>
      <button @click="snoozeId = null">Close</button>
    </section>

    <section>
      <h2>Status</h2>
      <p v-if="status">
        {{ status.device }} · SDK {{ status.sdk }}<br />
        notifications {{ status.notifications ? '✅' : '❌' }} ·
        exact alarms {{ status.exactAlarms ? '✅' : '❌' }} ·
        battery unrestricted {{ status.batteryUnrestricted ? '✅' : '❌' }}
      </p>
      <button @click="plugin('request_notifications')">Notif. permission</button>
      <button @click="plugin('open_exact_alarm_settings')">Exact alarms</button>
      <button @click="plugin('open_battery_settings')">Battery</button>
      <button @click="refresh">Refresh</button>
    </section>

    <section>
      <h2>New reminder</h2>
      <input v-model="title" />
      <select v-model.number="inSeconds">
        <option :value="30">30 s</option>
        <option :value="120">2 min</option>
        <option :value="300">5 min</option>
        <option :value="900">15 min</option>
      </select>
      <select v-model="mode">
        <option value="exact">setExactAndAllowWhileIdle</option>
        <option value="alarmClock">setAlarmClock</option>
      </select>
      <button @click="create">Schedule (custom)</button>
      <button @click="official">Schedule (official plugin)</button>
    </section>

    <section>
      <h2>Tasks</h2>
      <ul>
        <li v-for="t in tasks" :key="t.id" :class="{ done: t.completedAt }">
          <b>{{ t.title }}</b> · due {{ fmt(t.dueAt) }} · next {{ fmt(t.nextFireAt) }} · fires {{ t.fires }} · {{ t.alarmMode }}
          <span v-if="t.completedAt"> · done {{ fmt(t.completedAt) }}</span>
          <template v-else>
            <button @click="complete(t.id)">Complete</button>
            <button @click="snoozeId = t.id">Snooze</button>
          </template>
        </li>
      </ul>
    </section>

    <section>
      <h2>Event log</h2>
      <p class="hint">source "native" = written by Kotlin→JNI→Rust without the webview.</p>
      <ul class="log">
        <li v-for="(e, i) in events" :key="i">{{ fmt(e.at) }} · {{ e.kind }} · {{ e.title }} · {{ e.source }}</li>
      </ul>
    </section>
  </main>
</template>

<style>
body { font-family: system-ui, sans-serif; margin: 0; padding: 16px; padding-top: max(16px, env(safe-area-inset-top)); }
h1 { font-size: 1.3rem; }
h2 { font-size: 1rem; margin: 0 0 6px; }
section { margin: 0 0 18px; }
button, input, select { font: inherit; margin: 2px; padding: 6px 10px; }
ul { padding-left: 18px; }
.done { opacity: 0.5; }
.err { color: #c00; }
.hint { color: #777; font-size: 0.85rem; }
.log { font-size: 0.85rem; }
.sheet { position: sticky; top: 0; background: #ffe; border: 1px solid #cc9; padding: 10px; border-radius: 10px; }
</style>
