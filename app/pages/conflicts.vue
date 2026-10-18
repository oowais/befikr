<script setup lang="ts">
// Edits from this device that lost to a concurrent edit on another device
// (both changed the same thing before syncing; the newer one won). One card
// per task: the whole task twice, mine on the left, theirs (in use now) on
// the right, differences in red. Tap the side to keep; mine is written back
// as a new edit, so it then wins everywhere.
import { listen } from '@tauri-apps/api/event'
import { Check, Smartphone } from 'lucide-vue-next'
import type { Conflict, Task } from '~/composables/api'

const { show } = useToast()
const list = ref<Conflict[] | null>(null)
const tasks = ref<Record<string, Task | null>>({})
const myName = ref('This device')
const error = ref('')
const picked = ref<Record<string, 'mine' | 'theirs'>>({})

async function load() {
  try {
    const all = await api.conflicts()
    const ids = [...new Set(all.filter((c) => c.table === 'tasks').map((c) => c.rowId))]
    const got = await Promise.all(ids.map((id) => api.getTask(id).catch(() => null)))
    tasks.value = Object.fromEntries(ids.map((id, i) => [id, got[i] ?? null]))
    list.value = all
  } catch (e) {
    error.value = String(e)
    list.value = []
  }
}
let unlisten: (() => void) | undefined
onMounted(async () => {
  myName.value = (await api.syncStatus().then((s) => s.name, () => '')) || 'This device'
  await load()
  unlisten = await listen('sync-changed', load).catch(() => undefined)
})
onUnmounted(() => unlisten?.())

type Line = { label: string; mine: string; theirs: string; differs: boolean }
type Card = { key: string; kind: string; title: string; at: number; from: string; conflicts: Conflict[]; lines: Line[] }

const fieldNames: Record<string, string> = {
  title: 'Title',
  description: 'Notes',
  due_at: 'Date',
  rrule: 'Repeat',
  adaptive: 'Repeat mode',
  completed_at: 'Status',
  deleted_at: 'Status',
  time: 'Time',
  value: 'Value',
}
const settingNames: Record<string, string> = { reremind_after: 'Re-remind after a swipe' }

function text(key: string, v: unknown): string {
  const empty = v === null || v === undefined || v === ''
  switch (key) {
    case 'tasks.due_at':
      return typeof v === 'number' ? dayLabel(v) : 'No date'
    case 'tasks.rrule':
      return typeof v === 'string' && v ? rruleText(v) : 'Doesn’t repeat'
    case 'tasks.adaptive':
      return v ? 'Adaptive' : 'Static'
    case 'tasks.completed_at':
      return empty ? 'Open' : 'Done'
    case 'tasks.deleted_at':
    case 'time_words.deleted_at':
      return empty ? 'Kept' : 'Deleted'
    case 'settings.value':
      try {
        const parsed = JSON.parse(String(v))
        return Array.isArray(parsed) ? parsed.join(', ') : String(parsed)
      } catch {
        return String(v)
      }
    default:
      return empty ? '—' : String(v)
  }
}

/** A task's current values, keyed like conflict fields. */
const taskFields = (t: Task): Record<string, unknown> => ({
  title: t.title,
  due_at: t.dueAt,
  rrule: t.rrule,
  adaptive: t.adaptive,
  description: t.description,
  completed_at: t.completedAt,
  deleted_at: t.deletedAt,
})

const cards = computed<Card[]>(() => {
  const groups = new Map<string, Conflict[]>()
  for (const c of list.value ?? []) {
    const k = `${c.table}:${c.rowId}`
    groups.set(k, [...(groups.get(k) ?? []), c])
  }
  return [...groups.entries()].map(([key, cs]) => {
    const first = cs[0]!
    const by = (f: string) => cs.find((c) => c.field === f)
    const line = (field: string, current: unknown): Line => {
      const c = by(field)
      const k = `${first.table}.${field}`
      const mine = text(k, c ? c.lostValue : current)
      const theirs = text(k, c ? current ?? c.wonValue : current)
      return { label: fieldNames[field] ?? field, mine, theirs, differs: !!c && mine !== theirs }
    }
    let kind = 'Task'
    let title = first.label
    let lines: Line[]
    if (first.table === 'tasks') {
      const t = tasks.value[first.rowId]
      const cur: Record<string, unknown> = t
        ? taskFields(t)
        : Object.fromEntries(cs.map((c) => [c.field, c.wonValue]))
      title = t?.title ?? first.label
      // Always the basics; the rest only when set or in conflict.
      const shown = ['title', 'due_at', 'rrule']
      if (cur.rrule || by('adaptive')) shown.push('adaptive')
      if (cur.description || by('description')) shown.push('description')
      for (const f of ['completed_at', 'deleted_at']) if (by(f)) shown.push(f)
      lines = shown.map((f) => line(f, cur[f]))
    } else {
      kind = first.table === 'time_words' ? 'Time word' : 'Setting'
      title = first.table === 'settings' ? settingNames[first.label] ?? first.label : first.label
      lines = cs.map((c) => line(c.field, c.wonValue))
    }
    return {
      key,
      kind,
      title,
      at: Math.max(...cs.map((c) => c.at)),
      from: cs.find((c) => c.wonBy)?.wonBy ?? 'Other device',
      conflicts: cs,
      lines,
    }
  })
})

async function pick(card: Card, side: 'mine' | 'theirs') {
  if (picked.value[card.key]) return
  picked.value = { ...picked.value, [card.key]: side }
  // Show the tick for a moment, then the card goes (optimistic).
  await new Promise((r) => setTimeout(r, 350))
  const ids = new Set(card.conflicts.map((c) => c.id))
  list.value = list.value?.filter((c) => !ids.has(c.id)) ?? null
  error.value = ''
  try {
    for (const c of card.conflicts) await api.resolveConflict(c.id, side === 'mine')
    listChanged()
    show({ message: side === 'mine' ? 'Kept your version' : `Kept ${card.from}’s version` })
  } catch (e) {
    error.value = String(e)
    await load()
  } finally {
    const { [card.key]: _, ...rest } = picked.value
    picked.value = rest
  }
}

const ago = (ms: number) => {
  const s = Math.max(0, Math.round((Date.now() - ms) / 1000))
  return s < 60 ? 'just now' : `${span(s * 1000)} ago`
}
</script>

<template>
  <Screen title="Conflicts">
    <p v-if="error" class="error">{{ error }}</p>

    <p v-if="list && !cards.length" class="muted small intro">
      Nothing here. When this device and another change the same thing before they sync, the newer change wins and
      this device’s version shows up here.
    </p>
    <template v-else-if="list">
      <p class="muted small intro">Changed on two devices before they synced. Tap the version to keep.</p>
      <TransitionGroup tag="div" name="list" class="cards">
        <section v-for="card in cards" :key="card.key" class="pane card">
          <div class="head">
            <span class="title">{{ card.title }}</span>
            <span class="muted small">{{ card.kind }} · {{ ago(card.at) }}</span>
          </div>
          <div class="sides">
            <button
              v-for="side in ['mine', 'theirs'] as const"
              :key="side"
              type="button"
              class="side"
              :class="{ chosen: picked[card.key] === side, faded: picked[card.key] && picked[card.key] !== side }"
              :aria-label="side === 'mine' ? `Keep your version (${myName})` : `Keep ${card.from}’s version`"
              @click="pick(card, side)"
            >
              <span class="who">
                <Check v-if="picked[card.key] === side" :size="15" class="tick" />
                <Smartphone v-else :size="15" class="muted" />
                <span class="who-name">{{ side === 'mine' ? 'Yours' : card.from }}</span>
              </span>
              <span class="muted tiny">{{ side === 'mine' ? myName : 'In use now' }}</span>
              <span v-for="l in card.lines" :key="l.label" class="line" :class="{ diff: l.differs }">
                <span class="muted tiny">{{ l.label }}</span>
                <span class="val">{{ side === 'mine' ? l.mine : l.theirs }}</span>
              </span>
            </button>
          </div>
        </section>
      </TransitionGroup>
    </template>
  </Screen>
</template>

<style scoped>
.intro { margin: 4px 0 12px; }
.cards { display: flex; flex-direction: column; gap: 14px; }
.card { display: flex; flex-direction: column; gap: 12px; padding: 14px; }
.head { display: flex; flex-direction: column; gap: 2px; padding: 0 2px; }
.title { font-weight: 600; overflow-wrap: anywhere; }
.sides { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
.side {
  display: flex; flex-direction: column; align-items: stretch; gap: 8px; text-align: left; min-width: 0;
  padding: 12px; border-radius: 12px;
  box-shadow: inset 0 0 0 1px var(--line);
  transition: opacity 0.2s, box-shadow 0.2s;
}
.side.chosen { box-shadow: inset 0 0 0 2px var(--text); }
.side.faded { opacity: 0.4; }
.who { display: flex; align-items: center; gap: 6px; font-weight: 600; }
.who-name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.tick { color: var(--text); }
.tiny { font-size: 0.75rem; }
.line { display: flex; flex-direction: column; gap: 1px; padding: 4px 6px; margin: 0 -6px; border-radius: 6px; }
.val { overflow-wrap: anywhere; white-space: pre-wrap; font-size: 0.92rem; }
.diff { background: color-mix(in oklch, var(--red) 12%, transparent); }
.diff .val { color: var(--red); font-weight: 500; }
</style>
