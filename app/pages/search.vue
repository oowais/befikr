<script setup lang="ts">
// Search open and done tasks: title, notes and subtasks; every word must
// appear. Open ones first (list order), then done ones, newest first. Tap a
// result to open it in the task sheet.
import { Check, Repeat, Search, X } from 'lucide-vue-next'
import type { SearchHit } from '~/composables/api'

const { open: openTask } = useTaskSheet()
const query = ref('')
const hits = ref<SearchHit[]>([])
const searched = ref('')
const error = ref('')
const input = ref<HTMLInputElement>()

let seq = 0
async function run() {
  const q = query.value.trim()
  const mine = ++seq
  if (!q) {
    hits.value = []
    searched.value = ''
    return
  }
  try {
    const found = await api.search(q)
    // Typing on: only the latest answer counts.
    if (mine !== seq) return
    hits.value = found
    searched.value = q
    error.value = ''
  } catch (e) {
    error.value = friendlyError(e, 'search')
  }
}
let timer: ReturnType<typeof setTimeout> | undefined
watch(query, () => {
  clearTimeout(timer)
  timer = setTimeout(run, 120)
})
// A task changed in the sheet (or by sync): results follow.
watch(listVersion, run)

const open = computed(() => hits.value.filter((h) => !h.completedAt))
const done = computed(() => hits.value.filter((h) => h.completedAt))

function clear() {
  query.value = ''
  input.value?.focus()
}

onMounted(() => input.value?.focus())
</script>

<template>
  <Screen title="Search">
    <label class="box">
      <Search :size="20" class="muted" />
      <input
        ref="input"
        v-model="query"
        class="field"
        type="search"
        enterkeyhint="search"
        placeholder="Title, notes or subtask"
        autocomplete="off"
        @keydown.esc="query ? clear() : $router.back()"
      />
      <button v-if="query" class="icon-btn clear" aria-label="Clear" @click="clear"><X :size="18" /></button>
    </label>

    <p v-if="error" class="muted">{{ error }}</p>

    <template v-if="open.length">
      <h2 class="section-title">Open</h2>
      <ul class="pane rows list">
        <li v-for="h in open" :key="h.id">
          <button class="text" @click="openTask(h.id)">
            <span>{{ h.title }}</span>
            <span class="muted small meta">
              <Repeat v-if="h.rrule" :size="13" />
              {{ h.snoozedUntil ?? h.dueAt ? dueLabel(h.snoozedUntil ?? h.dueAt) : 'No date' }}
              <template v-if="h.rrule && h.lastDoneAt"> · last done {{ dayLabel(h.lastDoneAt) }}</template>
            </span>
          </button>
        </li>
      </ul>
    </template>

    <template v-if="done.length">
      <h2 class="section-title">Done</h2>
      <ul class="pane rows list">
        <li v-for="h in done" :key="h.id">
          <span class="check"><span class="checkbox checked"><Check :size="14" :stroke-width="3" /></span></span>
          <button class="text" @click="openTask(h.id)">
            <span>{{ h.title }}</span>
            <span class="muted small">{{ h.rrule && h.lastDoneAt !== h.completedAt ? 'Retired' : 'Done' }} {{ dayLabel(h.completedAt!) }}</span>
          </button>
        </li>
      </ul>
    </template>

    <p v-if="searched && !hits.length" class="muted empty">Nothing matches "{{ searched }}".</p>
  </Screen>
</template>

<style scoped>
.box { position: relative; display: flex; align-items: center; margin-top: 4px; }
.box > .muted { position: absolute; left: 14px; pointer-events: none; }
.box .field { padding-left: 44px; padding-right: 44px; font-size: 1.05rem; }
/* Our own clear button, not the browser's. */
.box .field::-webkit-search-cancel-button { display: none; }
.clear { position: absolute; right: 2px; }
.section-title { margin: 20px 0 0; }
.list { list-style: none; margin: 8px 0 0; }
.list li { display: flex; align-items: center; gap: 2px; padding: 6px 0; }
.check { width: 44px; height: 44px; display: grid; place-items: center; flex: none; margin-left: -10px; }
.text { flex: 1; display: flex; flex-direction: column; gap: 2px; padding: 6px 0; color: inherit; min-width: 0; text-align: left; }
.meta svg { vertical-align: -2px; margin-right: 2px; }
.empty { margin-top: 20px; text-align: center; }
</style>
