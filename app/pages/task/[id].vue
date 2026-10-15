<script setup lang="ts">
import { AlarmClock, ArrowLeft, CalendarDays, Check, CheckCircle2, Moon, Plus, Repeat, RotateCcw, StickyNote, Trash2, X } from 'lucide-vue-next'
import { firstDue } from '@befikr/parser'
import type { Task } from '~/composables/api'

const route = useRoute()
const router = useRouter()
const { show } = useToast()
const sound = useSound()
const id = computed(() => String(route.params.id))

const task = ref<Task | null>(null)
const subtasks = ref<Task[]>([])
const title = ref('')
const description = ref('')
const newSub = ref('')
const error = ref('')
const pickingRepeat = ref(false)
/** Date/repeat found in an edited title, waiting for the user to apply or keep as text. */
const found = ref<ReturnType<typeof parseWith> | null>(null)
const showNotes = ref(false)
const picking = ref(false)

async function load() {
  try {
    task.value = await api.getTask(id.value)
    subtasks.value = await api.listSubtasks(id.value)
    title.value = task.value.title
    description.value = task.value.description
    showNotes.value = !!task.value.description
  } catch (e) {
    error.value = String(e)
  }
}
onMounted(load)

async function patch(p: Parameters<typeof api.updateTask>[1]) {
  try {
    task.value = await api.updateTask(id.value, p)
    error.value = ''
  } catch (e) {
    error.value = String(e)
  }
}

function saveTitle() {
  if (!title.value.trim() || title.value === task.value?.title) return
  const r = parseWith(title.value, [])
  if (r.title && (r.due || r.rrule)) return (found.value = r)
  patch({ title: title.value })
}
// Apply what the edited title says ("call mom tomorrow 5pm"), dropping those words.
function applyFound() {
  const r = found.value
  if (!r) return
  found.value = null
  const due = r.due ? localMs(r.due.date, r.due.time ?? dueTime()) : task.value?.dueAt ?? null
  title.value = r.title
  patch({ title: r.title, ...(due != null && { dueAt: due, tz: deviceTz() }), ...(r.rrule && { rrule: r.rrule }) })
}
function keepTitle() {
  found.value = null
  patch({ title: title.value })
}
const foundLabel = computed(() => {
  const r = found.value
  if (!r) return ''
  const when = r.due ? dayLabel(localMs(r.due.date, r.due.time ?? dueTime())) : ''
  return [when, r.recurrenceText].filter(Boolean).join(' · ')
})
// Notes grow with their text.
const notesEl = ref<HTMLTextAreaElement>()
function growNotes() {
  const el = notesEl.value
  if (!el) return
  el.style.height = 'auto'
  el.style.height = `${el.scrollHeight}px`
}
watch([description, showNotes], () => nextTick(growNotes))
function openNotes() {
  showNotes.value = true
  nextTick(() => notesEl.value?.focus())
}
const saveNotes = () => description.value !== task.value?.description && patch({ description: description.value })
function setDue(ms: number | null) {
  patch(ms == null ? { dueAt: null, rrule: null, tz: null } : { dueAt: ms, tz: deviceTz() })
}
/** The task's time of day, 'HH:MM' (09:00 when undated). */
function dueTime() {
  const d = task.value?.dueAt
  return d == null ? '09:00' : new Date(d).toTimeString().slice(0, 5)
}
// A new rule moves the date to its first matching day, from the current date
// if that's still ahead, keeping the time of day.
function setRepeat(rule: string | null) {
  if (!rule) return patch({ rrule: null })
  const d = task.value?.dueAt
  const from = d != null && d > Date.now() ? new Date(new Date(d).setHours(0, 0, 0, 0)) : new Date()
  const f = firstDue(rule, from, dueTime())
  patch({ rrule: rule, dueAt: localMs(f.date, f.time!), tz: task.value?.tz ?? deviceTz() })
}

async function addSub() {
  const t = newSub.value.trim()
  if (!t) return
  newSub.value = ''
  await api.addTask({ title: t, parentId: id.value }).catch((e) => (error.value = String(e)))
  subtasks.value = await api.listSubtasks(id.value)
}
async function toggleSub(s: Task) {
  haptic()
  if (!s.completedAt) sound.tick()
  await (s.completedAt ? api.reopenTask(s.id) : api.completeTask(s.id))
  subtasks.value = await api.listSubtasks(id.value)
}

async function complete() {
  haptic()
  sound.complete()
  const t = await api.completeTask(id.value)
  show({
    message: t.rrule && !t.completedAt ? 'Done · next one scheduled' : 'Completed',
    action: { label: 'Undo', run: () => api.reopenTask(t.id).then(listChanged) },
  })
  router.back()
}
async function reopen() {
  haptic()
  try {
    task.value = await api.reopenTask(id.value)
    show({ message: 'Back in to-do' })
  } catch (e) {
    error.value = String(e)
  }
}
async function remove() {
  const t = await api.deleteTask(id.value)
  show({ message: 'Deleted', action: { label: 'Undo', run: () => api.restoreTask(t.id).then(listChanged) } })
  router.back()
}
</script>

<template>
  <main class="page">
    <header class="topbar">
      <button class="icon-btn" aria-label="Back" @click="router.back()"><ArrowLeft :size="22" /></button>
      <span style="flex: 1" />
      <button v-if="task?.completedAt" class="btn" @click="reopen"><RotateCcw :size="18" /> Undo</button>
      <template v-else-if="task">
        <NuxtLink v-if="task.dueAt" :to="`/snooze?id=${id}`" class="btn"><AlarmClock :size="18" /> Snooze</NuxtLink>
        <button class="btn primary" @click="complete"><Check :size="18" /> Done</button>
      </template>
    </header>

    <p v-if="error" class="error">{{ error }}</p>

    <template v-if="task">
      <input
        v-model="title"
        class="title-input"
        autocapitalize="sentences" autocorrect="on" spellcheck="true"
        @blur="saveTitle"
        @keydown.enter="($event.target as HTMLInputElement).blur()"
      />

      <div v-if="found" class="pane found">
        <span class="flex">Set <b>{{ foundLabel }}</b>?</span>
        <button class="btn" @click="keepTitle">Keep as text</button>
        <button class="btn primary" @click="applyFound">Set</button>
      </div>

      <div class="pane rows props">
        <div v-if="task.completedAt" class="prop">
          <CheckCircle2 :size="20" class="muted" /> <span class="flex">Completed {{ dayLabel(task.completedAt) }}</span>
        </div>
        <div class="prop">
          <CalendarDays :size="20" class="muted" />
          <button class="flex left" @click="picking = true">
            <span v-if="task.dueAt">{{ dayLabel(task.dueAt) }}</span>
            <span v-else class="muted">Add date</span>
          </button>
          <button v-if="task.dueAt" class="icon-btn" aria-label="Remove date" @click="setDue(null)"><X :size="18" /></button>
        </div>
        <div v-if="task.snoozedUntil" class="prop muted small">
          <Moon :size="18" /> <span class="flex">Snoozed until {{ dayLabel(task.snoozedUntil) }}</span>
        </div>
        <div v-if="task.rrule" class="prop">
          <Repeat :size="20" class="muted" />
          <button class="flex left cap" title="Change" @click="pickingRepeat = true">{{ rruleText(task.rrule) }}</button>
          <button class="icon-btn" aria-label="Stop repeating" @click="patch({ rrule: null })"><X :size="18" /></button>
        </div>
        <div v-if="task.rrule" class="prop adaptive">
          <AdaptiveToggle :model-value="task.adaptive" @update:model-value="patch({ adaptive: $event })" />
        </div>
        <div v-if="showNotes" class="prop top">
          <StickyNote :size="20" class="muted" />
          <textarea
            ref="notesEl"
            v-model="description"
            class="notes"
            rows="1"
            placeholder="Notes"
            autocapitalize="sentences" autocorrect="on" spellcheck="true"
            @blur="saveNotes"
          />
        </div>
      </div>

      <div class="add-props">
        <button v-if="!task.rrule" class="chip off" @click="pickingRepeat = true"><Plus :size="15" /> Repeat</button>
        <button v-if="!showNotes" class="chip off" @click="openNotes"><Plus :size="15" /> Notes</button>
      </div>

      <h2 class="section-title">
        Subtasks <span v-if="subtasks.length">{{ subtasks.filter((s) => s.completedAt).length }}/{{ subtasks.length }}</span>
      </h2>
      <div class="pane rows subs">
        <button v-for="s in subtasks" :key="s.id" class="sub" :class="{ done: s.completedAt }" @click="toggleSub(s)">
          <span class="checkbox square" :class="{ checked: s.completedAt }"><Check :size="14" :stroke-width="3" /></span>
          <span>{{ s.title }}</span>
        </button>
        <form class="sub" @submit.prevent="addSub">
          <Plus :size="20" class="muted add-icon" />
          <input
            v-model="newSub"
            class="bare"
            placeholder="Add subtask"
            enterkeyhint="done"
            autocapitalize="sentences" autocorrect="on" spellcheck="true"
          />
        </form>
      </div>

      <button class="btn danger delete" @click="remove"><Trash2 :size="18" /> Delete task</button>
    </template>

    <RepeatSheet v-if="pickingRepeat" :model-value="task?.rrule ?? null" :start="task?.dueAt" @update:model-value="setRepeat" @close="pickingRepeat = false" />
    <DateTimeSheet
      v-if="picking"
      :model-value="task?.dueAt ?? null"
      title="Due"
      allow-clear
      @update:model-value="setDue"
      @close="picking = false"
    />
  </main>
</template>

<style scoped>
.topbar { gap: 8px; }
.title-input { width: 100%; font-size: 1.4rem; font-weight: 600; letter-spacing: -0.01em; border: 0; outline: none; background: none; padding: 12px 0; }
.prop { display: flex; align-items: center; gap: 12px; min-height: 52px; }
/* Under the repeat row, aligned with its text; no divider between them. */
.prop.adaptive { min-height: 0; padding: 0 0 12px 32px; margin-top: -6px; border-top: 0; box-shadow: none; }
.prop.top { align-items: flex-start; padding-top: 14px; }
.flex { flex: 1; }
.left { text-align: left; min-height: 44px; }
.notes { flex: 1; border: 0; outline: none; background: none; resize: none; overflow: hidden; padding: 0 0 14px; line-height: 1.4; }
.add-props { display: flex; gap: 8px; margin-top: 12px; }
.sub { display: flex; align-items: center; gap: 12px; min-height: 48px; width: 100%; text-align: left; }
.sub.done span:last-child { text-decoration: line-through; color: var(--muted); }
.add-icon { margin: 0 2px; }
.bare { flex: 1; border: 0; outline: none; background: none; padding: 12px 0; }
.delete { margin-top: 40px; }
.cap::first-letter { text-transform: uppercase; }
.found { display: flex; align-items: center; gap: 8px; padding: 10px 12px; margin-bottom: 12px; flex-wrap: wrap; animation: float-in 0.2s; }
</style>
