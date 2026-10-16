<script setup lang="ts">
// A task in the main list. Tap the circle to complete, tap the row to open.
// Swipe right to complete, left to snooze (dated tasks only).
import { Check, ListChecks, Moon, Repeat } from 'lucide-vue-next'
import type { Row } from '~/composables/api'

const props = defineProps<{ row: Row; now: number; done?: boolean; timeOnly?: boolean }>()
const emit = defineEmits<{ complete: []; open: []; snooze: []; menu: [e: MouseEvent] }>()

const label = computed(() => dueLabel(props.row.snoozedUntil ?? props.row.dueAt, props.now, props.timeOnly))
const big = computed(() => props.row.urgency === 'red' || props.row.urgency === 'yellow')
const canSnooze = computed(() => props.row.dueAt != null || props.row.snoozedUntil != null)

// ---- Swipe
const AXIS_LOCK = 10 // px before deciding horizontal vs vertical
const el = ref<HTMLElement>()
const dx = ref(0)
const dragging = ref(false)
const leaving = ref(false)
let startX = 0
let startY = 0
let axis: 'x' | 'y' | null = null
let armed = false
let swiped = false

const threshold = () => Math.min(110, (el.value?.offsetWidth ?? 360) * 0.3)
const past = computed(() => Math.abs(dx.value) >= threshold())

function down(e: PointerEvent) {
  if (props.done || e.pointerType === 'mouse' && e.button !== 0) return
  startX = e.clientX
  startY = e.clientY
  axis = null
  armed = false
  swiped = false
  dragging.value = true
}

function move(e: PointerEvent) {
  if (!dragging.value) return
  const x = e.clientX - startX
  const y = e.clientY - startY
  if (!axis) {
    if (Math.abs(x) < AXIS_LOCK && Math.abs(y) < AXIS_LOCK) return
    axis = Math.abs(x) > Math.abs(y) ? 'x' : 'y'
    if (axis === 'y') return (dragging.value = false)
    swiped = true
    ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
  }
  // Left only when there is something to snooze; resist past the edge.
  let v = x < 0 && !canSnooze.value ? x / 6 : x
  const t = threshold()
  if (Math.abs(v) > t) v = Math.sign(v) * (t + (Math.abs(v) - t) * 0.4)
  dx.value = v
  const isPast = Math.abs(v) >= t && (v > 0 || canSnooze.value)
  if (isPast !== armed) {
    armed = isPast
    if (armed) haptic()
  }
}

function up() {
  if (!dragging.value) return
  dragging.value = false
  if (armed && dx.value > 0) {
    leaving.value = true
    dx.value = el.value?.offsetWidth ?? 400
    emit('complete')
  } else if (armed && dx.value < 0) {
    dx.value = 0
    emit('snooze')
  } else {
    dx.value = 0
  }
  armed = false
}

// A swipe ends with a click on the row; don't treat it as a tap.
function open() {
  if (swiped) return (swiped = false)
  emit('open')
}
</script>

<template>
  <li ref="el" class="row" :class="[row.urgency, { big }]" @contextmenu="emit('menu', $event)">
    <div v-if="dx > 0" class="reveal left" :class="{ past }"><Check :size="22" :stroke-width="2.5" /></div>
    <div v-if="dx < 0" class="reveal right" :class="{ past: past && canSnooze }"><Moon :size="22" /></div>

    <div
      class="swipe"
      :class="{ dragging, moved: dx !== 0, leaving }"
      :style="{ transform: dx ? `translateX(${dx}px)` : undefined }"
      @pointerdown="down"
      @pointermove="move"
      @pointerup="up"
      @pointercancel="up"
    >
      <button class="check" :aria-label="`Complete ${row.title}`" @click="emit('complete')">
        <span class="checkbox" :class="{ checked: done }"><Check :size="15" :stroke-width="3" /></span>
      </button>
      <button class="body" @click="open">
        <span class="title" :class="{ struck: done }">{{ row.title }}</span>
        <span v-if="label || row.rrule || row.subtasksTotal" class="meta">
          <span v-if="label" class="due"><span class="dot" :class="row.urgency" />{{ label }}</span>
          <Moon v-if="row.snoozedUntil" :size="14" aria-label="Snoozed" />
          <Repeat v-if="row.rrule" :size="14" aria-label="Repeats" />
          <span v-if="row.subtasksTotal" class="subs"><ListChecks :size="14" />{{ row.subtasksDone }}/{{ row.subtasksTotal }}</span>
        </span>
      </button>
    </div>
  </li>
</template>

<style scoped>
/* Full pane width, so the reveal fills edge to edge. */
.row { position: relative; list-style: none; margin: 0 -14px; overflow: hidden; }
.swipe {
  position: relative; display: flex; align-items: flex-start; gap: 2px; padding: 8px 14px;
  touch-action: pan-y; user-select: none; -webkit-user-select: none;
  transition: transform 0.32s cubic-bezier(0.22, 1, 0.36, 1);
}
.swipe.dragging { transition: none; }
/* While moved, the row is a solid pane over what it reveals. */
.swipe.moved { background: var(--popover); }
.swipe.leaving { transition-duration: 0.25s; }

.reveal {
  position: absolute; inset: 0; display: flex; align-items: center; padding: 0 22px;
  color: var(--muted); background: var(--glass-state);
  transition: background-color 0.15s, color 0.15s;
}
.reveal.left { justify-content: flex-start; }
.reveal.right { justify-content: flex-end; }
.reveal.left.past {
  color: var(--on-accent);
  background: color-mix(in oklch, var(--accent) var(--primary-solid), transparent);
}
.reveal.right.past { color: oklch(0.2 0 0); background: var(--yellow); }

.check { width: 44px; height: 44px; display: grid; place-items: center; flex: none; margin: -6px 0 -6px -10px; }
.body { flex: 1; text-align: left; display: flex; flex-direction: column; gap: 3px; min-width: 0; padding: 2px 0; }
.title { overflow-wrap: anywhere; transition: color 0.2s; }
.title.struck { color: var(--muted); text-decoration: line-through; }
.big .title { font-weight: 600; }
.meta { display: flex; align-items: center; gap: 10px; font-size: 0.85rem; color: var(--muted); }
.due, .subs { display: inline-flex; align-items: center; gap: 6px; }
.subs { gap: 4px; }
.red .due { color: var(--red); font-weight: 500; }
.yellow .due { color: var(--text); }
</style>
