<script setup lang="ts">
// One text field. Parsed parts appear as chips: tap the date to change it,
// ✕ keeps the words as plain text. Full controls hide behind "More".
// Inputs update on every keystroke (no v-model): v-model waits for the
// keyboard to commit a word, so swipe/predictive typing lagged a word behind.
import { CalendarDays, ChevronDown, ChevronUp, Repeat, X } from 'lucide-vue-next'
const emit = defineEmits<{ close: []; saved: [] }>()

const text = ref('')
const ignored = ref<string[]>([])
const DEFAULT_TIME = '09:00'
const adaptive = ref(false)
const expanded = ref(false)
const manualDue = ref<number | null>(null)
const picking = ref(false)
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
const typed = (e: Event) => (e.target as HTMLInputElement | HTMLTextAreaElement).value

const parsed = computed(() => parseWith(text.value, ignored.value))
// A task that is only a date ("tomorrow") still saves, with the words as title.
const title = computed(() => parsed.value.title.trim() || text.value.trim())
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
  if (manualDue.value != null) return manualDue.value
  const d = parsed.value.due
  return d ? localMs(d.date, d.time ?? DEFAULT_TIME) : null
})

function unparse(kind: 'date' | 'recurrence') {
  const s = spanText(kind)
  if (s) ignored.value.push(s)
}
// ✕ on the date chip: drop a picked date and keep any parsed words as text.
function clearDate() {
  manualDue.value = null
  unparse('date')
}

async function save() {
  if (!title.value || saving.value) return
  saving.value = true
  error.value = ''
  try {
    // A repeat without a date starts today at the prompt time.
    let due = dueAt.value
    if (repeat.value.rrule && due == null) {
      const now = new Date()
      due = localMs(`${now.getFullYear()}-${now.getMonth() + 1}-${now.getDate()}`, DEFAULT_TIME)
    }
    await api.addTask({
      title: title.value,
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
    <form class="sheet popover" @submit.prevent="save">
      <textarea
        ref="input"
        :value="text"
        class="main-input"
        rows="1"
        placeholder="buy milk tomorrow evening"
        enterkeyhint="done"
        autocapitalize="sentences" autocorrect="on" spellcheck="true"
        @input="text = typed($event)"
        @keydown.enter.prevent="save"
      />

      <p v-if="parsed.title && parsed.title !== text.trim()" class="preview muted">→ {{ parsed.title }}</p>

      <div class="chips">
        <span v-if="dueAt != null" class="chip split">
          <button type="button" class="part" aria-label="Change date and time" @click="picking = true">
            <CalendarDays :size="16" /> {{ dayLabel(dueAt) }}
          </button>
          <button type="button" class="x" :aria-label="manualDue == null ? 'Keep as text' : 'Remove date'" @click="clearDate">
            <X :size="15" />
          </button>
        </span>
        <span v-if="parsed.recurrenceText && !repeatText" class="chip split">
          <span class="part"><Repeat :size="16" /> {{ parsed.recurrenceText }}</span>
          <button type="button" class="x" aria-label="Keep as text" @click="unparse('recurrence')"><X :size="15" /></button>
        </span>
        <button v-if="repeat.rrule" type="button" class="chip" :class="{ off: !adaptive }" @click="adaptive = !adaptive">
          {{ adaptive ? 'From completion' : 'On schedule' }}
        </button>
        <button v-for="(s, i) in ignored" :key="s + i" type="button" class="chip off" @click="ignored.splice(i, 1)">
          “{{ s }}” as text <X :size="14" />
        </button>
      </div>

      <div v-if="expanded" class="more">
        <button type="button" class="field pick-date" @click="picking = true">
          <CalendarDays :size="18" />
          <span :class="{ muted: dueAt == null }">{{ dueAt != null ? dayLabel(dueAt) : 'Pick date & time' }}</span>
        </button>
        <label class="labeled">
          <Repeat :size="18" class="muted" />
          <input :value="repeatText" class="field" placeholder="every monday" autocorrect="on" @input="repeatText = typed($event)" />
        </label>
        <p v-if="repeatText && repeat.text" class="muted small">{{ repeat.text }}</p>
        <textarea :value="description" class="field" rows="3" placeholder="Notes" @input="description = typed($event)" autocapitalize="sentences" autocorrect="on" spellcheck="true" />
      </div>

      <p v-if="error" class="error">{{ error }}</p>

      <div class="actions">
        <button type="button" class="btn" @click="expanded = !expanded">
          <component :is="expanded ? ChevronUp : ChevronDown" :size="18" /> {{ expanded ? 'Less' : 'More' }}
        </button>
        <button type="submit" class="btn primary" :disabled="!title || saving">Add</button>
      </div>
    </form>
    <DateTimeSheet v-if="picking" :model-value="dueAt" title="When" allow-clear @update:model-value="manualDue = $event" @close="picking = false" />
  </div>
</template>

<style scoped>
.backdrop {
  position: fixed; inset: 0; z-index: 40; background: oklch(0 0 0 / 18%);
  display: flex; align-items: flex-end; animation: fade-in 0.15s;
}
.sheet {
  width: calc(100% - 16px); max-width: 640px; margin: 0 auto 8px; border-radius: 22px;
  padding: 14px 14px calc(14px + env(safe-area-inset-bottom));
  display: flex; flex-direction: column; gap: 10px;
  animation: float-in 0.24s cubic-bezier(0.22, 1, 0.36, 1);
}
.main-input { border: 0; outline: none; background: none; font-size: 1.2rem; padding: 6px 2px; width: 100%; resize: none; overflow: hidden; }
.preview { margin: 0; font-size: 0.9rem; }
.chips { display: flex; flex-wrap: wrap; gap: 8px; }
.chips:empty { display: none; }
.chip.split { padding: 0; gap: 0; }
.chip.split .part { display: inline-flex; align-items: center; gap: 6px; padding: 6px 4px 6px 12px; }
.chip.split .x { display: grid; place-items: center; width: 34px; height: 32px; color: var(--muted); }
.more { display: flex; flex-direction: column; gap: 10px; }
.pick-date { display: flex; align-items: center; gap: 10px; text-align: left; }
.labeled { display: flex; align-items: center; gap: 10px; }
.small { margin: 0; }
.actions { display: flex; justify-content: space-between; }
.error { margin: 0; font-size: 0.9rem; }
</style>
