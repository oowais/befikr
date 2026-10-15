<script setup lang="ts">
// One text field. Parsed parts appear as chips: tap the date to change it,
// ✕ keeps the words as plain text. Full controls hide behind "More".
// The draft survives closing the sheet (useQuickAddDraft); Add or Clear resets it.
// Inputs update on every keystroke (no v-model): v-model waits for the
// keyboard to commit a word, so swipe/predictive typing lagged a word behind.
import { firstDue } from '@befikr/parser'
import { CalendarDays, ChevronDown, ChevronUp, Clock, Repeat, Trash2, X } from 'lucide-vue-next'
const emit = defineEmits<{ close: []; saved: [] }>()

const { text, ignored, adaptive, expanded, manualDue, manualRule, description, chosenTime } = toRefs(useQuickAddDraft())
const DEFAULT_TIME = '09:00'
const fallbackTime = computed(() => chosenTime.value ?? DEFAULT_TIME)
const picking = ref(false)
const pickingRepeat = ref(false)
const saving = ref(false)
const error = ref('')
const input = ref<HTMLTextAreaElement>()

useBackClose(() => emit('close'))
onMounted(() => {
  input.value?.focus()
  nextTick(grow)
})

const hasDraft = computed(() => !!(text.value || description.value || manualDue.value != null || manualRule.value !== undefined))
function clearDraft() {
  clearQuickAddDraft()
  input.value?.focus()
}

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
// Text split at parsed spans, drawn behind the textarea to highlight them.
const segments = computed(() => {
  const out: { text: string; kind?: 'date' | 'recurrence' }[] = []
  let pos = 0
  for (const sp of parsed.value.spans) {
    if (sp.start > pos) out.push({ text: text.value.slice(pos, sp.start) })
    out.push({ text: text.value.slice(sp.start, sp.end), kind: sp.kind })
    pos = sp.end
  }
  out.push({ text: text.value.slice(pos) + '\u200b' })
  return out
})
const spanText = (kind: 'date' | 'recurrence') => {
  const s = parsed.value.spans.find((x) => x.kind === kind)
  return s ? text.value.slice(s.start, s.end) : null
}

const repeat = computed(() => {
  if (manualRule.value !== undefined) return { rrule: manualRule.value, text: manualRule.value && rruleText(manualRule.value) }
  return { rrule: parsed.value.rrule, text: parsed.value.recurrenceText }
})

const dueAt = computed<number | null>(() => {
  if (manualDue.value != null) return manualDue.value
  const d = parsed.value.due
  // A built repeat starts on its first matching day (at any typed time).
  if (manualRule.value) {
    const f = firstDue(manualRule.value, new Date(), d?.time ?? fallbackTime.value)
    return localMs(f.date, f.time!)
  }
  return d ? localMs(d.date, d.time ?? fallbackTime.value) : null
})

// A date was typed without a time: ask, with 09:00 picked. Add accepts it.
const promptTime = computed(() => manualDue.value == null && dueAt.value != null && !parsed.value.due?.time)
const timeWords = useTimeWords()
const timeChoices = computed(() => {
  const list = Object.entries(timeWords.value)
    .map(([word, t]) => ({ word, t }))
    .sort((a, b) => a.t.localeCompare(b.t))
  return list.some((c) => c.t === DEFAULT_TIME) ? list : [{ word: '', t: DEFAULT_TIME }, ...list]
})

// ✕ on the repeat chip: no repeat; typed words stay as text.
function clearRepeat() {
  unparse('recurrence')
  manualRule.value = undefined
}

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
    // A repeat without a date starts on its first day at the default time.
    let due = dueAt.value
    if (repeat.value.rrule && due == null) {
      const f = firstDue(repeat.value.rrule, new Date(), fallbackTime.value)
      due = localMs(f.date, f.time!)
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
    clearQuickAddDraft()
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
      <div class="input-wrap">
      <div class="mirror main-input" aria-hidden="true"><span v-for="(g, i) in segments" :key="i" :class="g.kind">{{ g.text }}</span></div>
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
      </div>

      <div class="chips">
        <!-- Expanded, the fields below show date and repeat. -->
        <span v-if="dueAt != null && !expanded" class="chip split">
          <button type="button" class="part" aria-label="Change date and time" @click="picking = true">
            <CalendarDays :size="16" /> {{ dayLabel(dueAt) }}
          </button>
          <button type="button" class="x" :aria-label="manualDue == null ? 'Keep as text' : 'Remove date'" @click="clearDate">
            <X :size="15" />
          </button>
        </span>
        <span v-if="repeat.rrule && !expanded" class="chip split">
          <button type="button" class="part" aria-label="Change repeat" @click="pickingRepeat = true">
            <Repeat :size="16" /> {{ repeat.text }}
          </button>
          <button type="button" class="x" aria-label="Don't repeat" @click="clearRepeat"><X :size="15" /></button>
        </span>
        <button v-if="repeat.rrule" type="button" class="chip" :class="{ off: !adaptive }" @click="adaptive = !adaptive">
          {{ adaptive ? 'From completion' : 'On schedule' }}
        </button>
        <button v-for="(s, i) in ignored" :key="s + i" type="button" class="chip off" @click="ignored.splice(i, 1)">
          “{{ s }}” as text <X :size="14" />
        </button>
      </div>

      <div v-if="promptTime" class="time-prompt">
        <span class="muted label"><Clock :size="16" /> No time given</span>
        <div class="times">
          <button
            v-for="c in timeChoices"
            :key="c.t + c.word"
            type="button"
            class="chip"
            :class="{ chosen: c.t === fallbackTime }"
            @click="chosenTime = c.t"
          >
            <span v-if="c.word" class="cap">{{ c.word }}</span> <span :class="{ muted: c.word }">{{ timeLabel(localMs('2000-01-01', c.t)) }}</span>
          </button>
          <button type="button" class="chip off" @click="picking = true">Other…</button>
        </div>
      </div>

      <div v-if="expanded" class="more">
        <button type="button" class="field pick-date" @click="picking = true">
          <CalendarDays :size="18" />
          <span :class="{ muted: dueAt == null }">{{ dueAt != null ? dayLabel(dueAt) : 'Pick date & time' }}</span>
        </button>
        <button type="button" class="field pick-date" @click="pickingRepeat = true">
          <Repeat :size="18" />
          <span :class="{ muted: !repeat.rrule }" class="cap">{{ repeat.rrule ? repeat.text : 'Repeat…' }}</span>
        </button>
        <textarea :value="description" class="field" rows="3" placeholder="Notes" @input="description = typed($event)" autocapitalize="sentences" autocorrect="on" spellcheck="true" />
      </div>

      <p v-if="error" class="error">{{ error }}</p>

      <div class="actions">
        <button type="button" class="btn" @click="expanded = !expanded">
          <component :is="expanded ? ChevronUp : ChevronDown" :size="18" /> {{ expanded ? 'Less' : 'More' }}
        </button>
        <button v-if="hasDraft" type="button" class="btn danger" @click="clearDraft"><Trash2 :size="18" /> Clear</button>
        <span class="flex" />
        <button type="submit" class="btn primary" :disabled="!title || saving">Add</button>
      </div>
    </form>
    <RepeatSheet v-if="pickingRepeat" :model-value="repeat.rrule" :start="manualDue ?? (parsed.due && !manualRule ? dueAt : null)" @update:model-value="manualRule = $event" @close="pickingRepeat = false" />
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
.main-input {
  border: 0; outline: none; background: none; font: inherit; font-size: 1.2rem; line-height: 1.4;
  padding: 6px 2px; width: 100%; resize: none; overflow: hidden; letter-spacing: normal;
}
/* Highlights sit behind the (transparent) textarea, in the same text layout. */
.input-wrap { position: relative; }
.input-wrap textarea { position: relative; display: block; }
.mirror {
  position: absolute; inset: 0; color: transparent; pointer-events: none;
  white-space: pre-wrap; overflow-wrap: break-word; box-sizing: border-box;
}
.mirror .date, .mirror .recurrence {
  border-radius: 6px; box-decoration-break: clone; -webkit-box-decoration-break: clone;
  background: color-mix(in oklch, var(--accent) 22%, transparent);
}
.mirror .recurrence { background: color-mix(in oklch, var(--yellow) 35%, transparent); }
.chips { display: flex; flex-wrap: wrap; gap: 8px; }
.chips:empty { display: none; }
.chip.split { padding: 0; gap: 0; }
.chip.split .part { display: inline-flex; align-items: center; gap: 6px; padding: 6px 4px 6px 12px; }
.chip.split .x { display: grid; place-items: center; width: 34px; height: 32px; color: var(--muted); }
.more { display: flex; flex-direction: column; gap: 10px; }
.time-prompt { display: flex; flex-direction: column; gap: 6px; }
.time-prompt .label { display: inline-flex; align-items: center; gap: 6px; font-size: 0.85rem; }
.times { display: flex; flex-wrap: wrap; gap: 8px; }
.pick-date { display: flex; align-items: center; gap: 10px; text-align: left; }
.cap::first-letter { text-transform: uppercase; }
.small { margin: 0; }
.actions { display: flex; gap: 8px; }
.flex { flex: 1; }
.error { margin: 0; font-size: 0.9rem; }
</style>
