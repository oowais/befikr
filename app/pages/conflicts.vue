<script setup lang="ts">
// Edits from this device that lost to a concurrent edit on another device
// (both changed the same thing before syncing; the newer one won). Keep
// theirs, or use mine: written back as a new edit, so it wins everywhere.
import { listen } from '@tauri-apps/api/event'
import { ArrowLeft } from 'lucide-vue-next'
import type { Conflict } from '~/composables/api'

const router = useRouter()
const { show } = useToast()
const list = ref<Conflict[] | null>(null)
const error = ref('')
const busy = ref<string | null>(null)

const load = async () => (list.value = await api.conflicts().catch((e) => ((error.value = String(e)), [])))
let unlisten: (() => void) | undefined
onMounted(async () => {
  await load()
  unlisten = await listen('sync-changed', load).catch(() => undefined)
})
onUnmounted(() => unlisten?.())

async function resolve(c: Conflict, useMine: boolean) {
  busy.value = c.id
  error.value = ''
  // Optimistic: gone from the list right away.
  list.value = list.value?.filter((x) => x.id !== c.id) ?? null
  try {
    await api.resolveConflict(c.id, useMine)
    listChanged()
    if (useMine) show({ message: 'Your version is back' })
  } catch (e) {
    error.value = String(e)
    await load()
  } finally {
    busy.value = null
  }
}

const fieldNames: Record<string, string> = {
  title: 'Title',
  description: 'Notes',
  due_at: 'Date',
  rrule: 'Repeat',
  adaptive: 'Repeat mode',
  completed_at: 'Done',
  deleted_at: 'Deleted',
  time: 'Time',
  value: 'Setting',
}
const settingNames: Record<string, string> = { reremind_minutes: 'Re-remind minutes' }

function what(c: Conflict): string {
  if (c.table === 'time_words') return `Time word “${c.label}”`
  if (c.table === 'settings') return settingNames[c.label] ?? c.label
  return c.label || 'A task'
}

function valueText(c: Conflict, v: unknown): string {
  const empty = v === null || v === ''
  switch (`${c.table}.${c.field}`) {
    case 'tasks.due_at':
      return typeof v === 'number' ? dayLabel(v) : 'No date'
    case 'tasks.rrule':
      return typeof v === 'string' && v ? rruleText(v) : 'Doesn’t repeat'
    case 'tasks.adaptive':
      return v ? 'From when done' : 'On schedule'
    case 'tasks.completed_at':
      return empty ? 'Not done' : 'Done'
    case 'tasks.deleted_at':
      return empty ? 'Kept' : 'Deleted'
    case 'time_words.deleted_at':
      return empty ? 'Kept' : 'Removed'
    case 'settings.value':
      try {
        const parsed = JSON.parse(String(v))
        return Array.isArray(parsed) ? parsed.join(', ') : String(parsed)
      } catch {
        return String(v)
      }
    default:
      return empty ? '(empty)' : String(v)
  }
}

const ago = (ms: number) => {
  const s = Math.max(0, Math.round((Date.now() - ms) / 1000))
  return s < 60 ? 'just now' : `${span(s * 1000)} ago`
}
</script>

<template>
  <main class="page">
    <header class="topbar">
      <button class="icon-btn" aria-label="Back" @click="router.back()"><ArrowLeft :size="22" /></button>
      <h1>Conflicts</h1>
    </header>
    <p v-if="error" class="error">{{ error }}</p>

    <p v-if="list && !list.length" class="muted small intro">
      Nothing here. When this device and another change the same thing before they sync, the newer change wins and
      this device’s version shows up here.
    </p>
    <template v-else-if="list">
      <p class="muted small intro">Changed on both devices before they synced. The newer change was kept.</p>
      <TransitionGroup tag="div" name="list" class="cards">
        <div v-for="c in list" :key="c.id" class="pane card">
          <div class="head">
            <span class="title">{{ what(c) }}</span>
            <span class="muted small">{{ fieldNames[c.field] ?? c.field }} · {{ ago(c.at) }}</span>
          </div>
          <div class="versions">
            <span class="muted small">Kept<template v-if="c.wonBy"> (from {{ c.wonBy }})</template></span>
            <span class="value">{{ valueText(c, c.wonValue) }}</span>
            <span class="muted small">Yours</span>
            <span class="value lost">{{ valueText(c, c.lostValue) }}</span>
          </div>
          <div class="row-end">
            <button class="btn" :disabled="busy === c.id" @click="resolve(c, false)">Keep</button>
            <button class="btn primary" :disabled="busy === c.id" @click="resolve(c, true)">Use mine</button>
          </div>
        </div>
      </TransitionGroup>
    </template>
  </main>
</template>

<style scoped>
.intro { margin: 4px 0 12px; }
.cards { display: flex; flex-direction: column; gap: 12px; }
.card { display: flex; flex-direction: column; gap: 12px; padding: 14px 16px; }
.head { display: flex; flex-direction: column; gap: 2px; }
.title { font-weight: 600; overflow-wrap: anywhere; }
.versions { display: grid; grid-template-columns: auto 1fr; gap: 6px 12px; align-items: baseline; }
.value { overflow-wrap: anywhere; white-space: pre-wrap; }
.lost { color: var(--muted); }
.row-end { display: flex; justify-content: flex-end; gap: 8px; }
</style>
