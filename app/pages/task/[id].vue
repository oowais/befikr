<script setup lang="ts">
import type { Task } from '~/composables/api'

const route = useRoute()
const router = useRouter()
const { show } = useToast()
const id = computed(() => String(route.params.id))

const task = ref<Task | null>(null)
const subtasks = ref<Task[]>([])
const title = ref('')
const description = ref('')
const due = ref('')
const repeatText = ref('')
const newSub = ref('')
const error = ref('')
const showRepeat = ref(false)
const showNotes = ref(false)

async function load() {
  try {
    task.value = await api.getTask(id.value)
    subtasks.value = await api.listSubtasks(id.value)
    title.value = task.value.title
    description.value = task.value.description
    due.value = toInputValue(task.value.dueAt)
    showNotes.value = !!task.value.description
    showRepeat.value = false
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

const saveTitle = () => title.value.trim() && title.value !== task.value?.title && patch({ title: title.value })
const saveNotes = () => description.value !== task.value?.description && patch({ description: description.value })
function saveDue() {
  const ms = fromInputValue(due.value)
  patch(ms == null ? { dueAt: null, rrule: null, tz: null } : { dueAt: ms, tz: deviceTz() })
}
function saveRepeat() {
  const t = repeatText.value.trim()
  if (!t) return patch({ rrule: null })
  const r = parseWith(`x ${t.startsWith('every') ? t : `every ${t}`}`, [])
  if (!r.rrule) return (error.value = `Couldn't understand “${t}”`)
  const dueAt = task.value?.dueAt ?? Date.now()
  repeatText.value = ''
  showRepeat.value = false
  patch({ rrule: r.rrule, dueAt, tz: task.value?.tz ?? deviceTz() })
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
  await (s.completedAt ? api.reopenTask(s.id) : api.completeTask(s.id))
  subtasks.value = await api.listSubtasks(id.value)
}

async function complete() {
  haptic()
  const t = await api.completeTask(id.value)
  show({ message: 'Completed', action: { label: 'Undo', run: () => api.reopenTask(t.id) } })
  router.back()
}
async function remove() {
  await api.deleteTask(id.value)
  show({ message: 'Deleted' })
  router.back()
}
</script>

<template>
  <main class="page">
    <header class="topbar">
      <button class="icon-btn" aria-label="Back" @click="router.back()">←</button>
      <span style="flex: 1" />
      <NuxtLink v-if="task?.dueAt" :to="`/snooze?id=${id}`" class="btn">Snooze</NuxtLink>
      <button class="btn primary" @click="complete">Done</button>
    </header>

    <p v-if="error" class="error">{{ error }}</p>

    <template v-if="task">
      <input v-model="title" class="title-input" @blur="saveTitle" @keydown.enter="($event.target as HTMLInputElement).blur()" />

      <div class="props">
        <label class="prop">
          <span>📅</span>
          <input v-model="due" type="datetime-local" class="field" @change="saveDue" />
        </label>
        <p v-if="task.snoozedUntil" class="muted small">💤 Snoozed until {{ dayLabel(task.snoozedUntil) }}</p>

        <div v-if="task.rrule" class="prop">
          <span>↻</span>
          <button class="flex left" title="Change" @click="showRepeat = !showRepeat">{{ rruleText(task.rrule) }}</button>
          <button class="chip" :class="{ off: !task.adaptive }" @click="patch({ adaptive: !task.adaptive })">
            {{ task.adaptive ? 'From completion' : 'On schedule' }}
          </button>
          <button class="icon-btn" aria-label="Stop repeating" @click="patch({ rrule: null })">✕</button>
        </div>
        <form v-if="showRepeat" class="prop" @submit.prevent="saveRepeat">
          <span>↻</span>
          <input v-model="repeatText" class="field" :placeholder="task.rrule ? 'change: every 2 weeks' : 'every monday'" />
        </form>

        <textarea
          v-if="showNotes"
          v-model="description"
          class="field"
          rows="4"
          placeholder="Notes"
          @blur="saveNotes"
        />

        <div class="add-props">
          <button v-if="!showRepeat && !task.rrule" class="chip off" @click="showRepeat = true">+ Repeat</button>
          <button v-if="!showNotes" class="chip off" @click="showNotes = true">+ Notes</button>
        </div>
      </div>

      <h2 class="section-title">Subtasks <span v-if="subtasks.length">{{ subtasks.filter((s) => s.completedAt).length }}/{{ subtasks.length }}</span></h2>
      <ul class="subs">
        <li v-for="s in subtasks" :key="s.id" :class="{ done: s.completedAt }">
          <button class="sub-check" @click="toggleSub(s)">{{ s.completedAt ? '☑' : '☐' }}</button>
          <span>{{ s.title }}</span>
        </li>
      </ul>
      <form @submit.prevent="addSub">
        <input v-model="newSub" class="field" placeholder="+ Add subtask" enterkeyhint="done" />
      </form>

      <button class="btn danger delete" @click="remove">Delete task</button>
    </template>
  </main>
</template>

<style scoped>
.title-input { width: 100%; font-size: 1.4rem; font-weight: 600; border: 0; outline: none; background: none; padding: 12px 0; }
.props { display: flex; flex-direction: column; gap: 10px; }
.prop { display: flex; align-items: center; gap: 10px; }
.flex { flex: 1; }
.left { text-align: left; }
.small { margin: 0; font-size: 0.85rem; }
.add-props { display: flex; gap: 8px; }
.subs { list-style: none; padding: 0; margin: 0 0 8px; }
.subs li { display: flex; align-items: center; gap: 8px; padding: 6px 0; }
.subs li.done span { text-decoration: line-through; color: var(--muted); }
.sub-check { font-size: 1.3rem; width: 32px; }
.delete { margin-top: 40px; }
.error { color: var(--red); }
</style>
