<script setup lang="ts">
// One text field. Parsed parts appear as chips; tap a chip to keep that text
// literal. Full controls hide behind "More".
const emit = defineEmits<{ close: []; saved: [] }>()

const text = ref('')
const ignored = ref<string[]>([])
const timePrompt = ref('09:00')
const adaptive = ref(false)
const expanded = ref(false)
const manualDue = ref('')
const repeatText = ref('')
const description = ref('')
const saving = ref(false)
const error = ref('')
const input = ref<HTMLTextAreaElement>()

onMounted(() => input.value?.focus())

// Auto-grow; Enter saves (dictation and long tasks wrap instead of scrolling).
function grow() {
  const el = input.value
  if (!el) return
  el.style.height = 'auto'
  el.style.height = `${el.scrollHeight}px`
}
watch(text, () => nextTick(grow))

const parsed = computed(() => parseWith(text.value, ignored.value))
const spanText = (kind: 'date' | 'recurrence') => {
  const s = parsed.value.spans.find((x) => x.kind === kind)
  return s ? text.value.slice(s.start, s.end) : null
}

const repeat = computed(() => {
  if (repeatText.value.trim()) {
    const r = parseWith(`x ${repeatText.value.trim()}`, [])
    return { rrule: r.rrule, text: r.recurrenceText }
  }
  return { rrule: parsed.value.rrule, text: parsed.value.recurrenceText }
})

const dueAt = computed<number | null>(() => {
  if (manualDue.value) return fromInputValue(manualDue.value)
  const d = parsed.value.due
  return d ? localMs(d.date, d.time ?? timePrompt.value) : null
})
const needsTime = computed(() => !manualDue.value && parsed.value.due && !parsed.value.due.time)
const dateChip = computed(() => {
  const d = parsed.value.due
  if (!d || manualDue.value) return null
  return d.time ? dayLabel(localMs(d.date, d.time)) : dateOnlyLabel(d.date)
})

function unparse(kind: 'date' | 'recurrence') {
  const s = spanText(kind)
  if (s) ignored.value.push(s)
}

async function save() {
  const title = parsed.value.title.trim()
  if (!title || saving.value) return
  saving.value = true
  error.value = ''
  try {
    // A repeat without a date starts today at the prompt time.
    let due = dueAt.value
    if (repeat.value.rrule && due == null) {
      const now = new Date()
      due = localMs(`${now.getFullYear()}-${now.getMonth() + 1}-${now.getDate()}`, timePrompt.value)
    }
    await api.addTask({
      title,
      description: description.value,
      dueAt: due,
      tz: due != null ? deviceTz() : null,
      rrule: repeat.value.rrule,
      adaptive: !!repeat.value.rrule && adaptive.value,
    })
    haptic()
    emit('saved')
    emit('close')
  } catch (e) {
    error.value = String(e)
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="backdrop" @click.self="emit('close')">
    <form class="sheet" @submit.prevent="save">
      <textarea
        ref="input"
        v-model="text"
        class="main-input"
        rows="1"
        placeholder="buy milk tomorrow evening"
        enterkeyhint="done"
        autocapitalize="sentences" autocorrect="on" spellcheck="true"
        @keydown.enter.prevent="save"
      />

      <p v-if="parsed.title && parsed.title !== text.trim()" class="preview muted">→ {{ parsed.title }}</p>

      <div class="chips">
        <button v-if="dateChip" type="button" class="chip" title="Tap to keep as text" @click="unparse('date')">
          📅 {{ dateChip }}
        </button>
        <label v-if="needsTime" class="chip">
          <input v-model="timePrompt" type="time" class="time" aria-label="Time" />
        </label>
        <button
          v-if="parsed.recurrenceText && !repeatText"
          type="button"
          class="chip"
          title="Tap to keep as text"
          @click="unparse('recurrence')"
        >
          ↻ {{ parsed.recurrenceText }}
        </button>
        <button
          v-if="repeat.rrule"
          type="button"
          class="chip"
          :class="{ off: !adaptive }"
          @click="adaptive = !adaptive"
        >
          {{ adaptive ? 'From completion' : 'On schedule' }}
        </button>
        <button v-for="(s, i) in ignored" :key="s + i" type="button" class="chip off" @click="ignored.splice(i, 1)">
          “{{ s }}” as text
        </button>
      </div>

      <div v-if="expanded" class="more">
        <label>
          <span class="muted">Date &amp; time</span>
          <input v-model="manualDue" type="datetime-local" class="field" />
        </label>
        <label>
          <span class="muted">Repeat</span>
          <input v-model="repeatText" class="field" placeholder="every monday" autocorrect="on" />
        </label>
        <p v-if="repeatText && repeat.text" class="muted small">↻ {{ repeat.text }}</p>
        <label>
          <span class="muted">Notes</span>
          <textarea v-model="description" class="field" rows="3" autocapitalize="sentences" autocorrect="on" spellcheck="true" />
        </label>
      </div>

      <p v-if="error" class="error">{{ error }}</p>

      <div class="actions">
        <button type="button" class="btn" @click="expanded = !expanded">{{ expanded ? 'Less' : 'More' }}</button>
        <button type="submit" class="btn primary" :disabled="!parsed.title.trim() || saving">Add</button>
      </div>
    </form>
  </div>
</template>

<style scoped>
.backdrop {
  position: fixed; inset: 0; z-index: 40; background: rgba(0, 0, 0, 0.3);
  display: flex; align-items: flex-end; animation: fade 0.15s;
}
.sheet {
  width: 100%; max-width: 640px; margin: 0 auto; background: var(--surface);
  border-radius: 20px 20px 0 0; padding: 16px 16px calc(16px + env(safe-area-inset-bottom));
  box-shadow: var(--shadow); display: flex; flex-direction: column; gap: 10px;
  animation: up 0.28s cubic-bezier(0.2, 0.9, 0.3, 1.15);
}
.main-input { border: 0; outline: none; background: none; font-size: 1.2rem; padding: 6px 2px; width: 100%; resize: none; overflow: hidden; }
.preview { margin: 0; font-size: 0.9rem; }
.chips { display: flex; flex-wrap: wrap; gap: 8px; }
.chips:empty { display: none; }
.time { border: 0; background: none; padding: 0; }
.more { display: flex; flex-direction: column; gap: 10px; }
.more label { display: flex; flex-direction: column; gap: 4px; font-size: 0.9rem; }
.small { margin: 0; font-size: 0.85rem; }
.actions { display: flex; justify-content: space-between; }
.error { color: var(--red); margin: 0; font-size: 0.9rem; }
@keyframes up { from { transform: translateY(100%); } }
@keyframes fade { from { opacity: 0; } }
</style>
