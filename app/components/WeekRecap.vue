<script setup lang="ts">
// Weekly recap: one card on the home screen from Sunday 17:00 until Monday
// noon, about the Monday–Sunday week just ending. Shown once per week (✕
// hides it), only with Carefree days on and something done that week.
import { Sprout, X } from 'lucide-vue-next'
import type { WeekStats } from '~/composables/api'

const KEY = 'befikr.recap'
const carefree = useCarefree()

/** Local Monday 00:00 of the week being recapped, or null outside the window. */
function recapWeek(now = new Date()): number | null {
  const day = now.getDay() // 0 = Sunday
  const sundayEvening = day === 0 && now.getHours() >= 17
  const mondayMorning = day === 1 && now.getHours() < 12
  if (!sundayEvening && !mondayMorning) return null
  const d = new Date(now)
  d.setHours(0, 0, 0, 0)
  d.setDate(d.getDate() - (sundayEvening ? 6 : 7))
  return d.getTime()
}

const start = recapWeek()
const dismissed = ref(read() === String(start))
const week = ref<WeekStats | null>(null)
const prev = ref<WeekStats | null>(null)

function read() {
  try {
    return localStorage.getItem(KEY)
  } catch {
    return null
  }
}
function dismiss() {
  dismissed.value = true
  try {
    localStorage.setItem(KEY, String(start))
  } catch {}
}

// Arrives with the list rather than pushing it down a moment later.
const releaseSplash = holdSplash()
onMounted(async () => {
  if (start == null || dismissed.value) return releaseSplash()
  const d = new Date(start)
  const prevStart = new Date(d.getFullYear(), d.getMonth(), d.getDate() - 7).getTime()
  const end = new Date(d.getFullYear(), d.getMonth(), d.getDate() + 7).getTime()
  // Buckets: last week, this week, (Monday morning) today so far.
  const starts = [prevStart, start, ...(Date.now() >= end ? [end] : [])]
  const s = await api.stats(prevStart, starts).catch(() => null)
  prev.value = s?.weeks[0] ?? null
  week.value = s?.weeks[1] ?? null
  releaseSplash()
})

const total = (w: WeekStats) => w.onTime + w.late + w.undated
const done = computed(() => (week.value ? total(week.value) : 0))
const onTimePct = computed(() => {
  const w = week.value
  return w && w.onTime + w.late ? Math.round((w.onTime / (w.onTime + w.late)) * 100) : null
})
const change = computed(() => {
  if (!prev.value) return ''
  const diff = done.value - total(prev.value)
  if (!total(prev.value)) return ''
  return diff === 0 ? 'Same as last week' : `${Math.abs(diff)} ${diff > 0 ? 'more' : 'fewer'} than last week`
})
const visible = computed(() => carefree.enabled.value && start != null && !dismissed.value && done.value > 0)
const streak = computed(() => carefree.data.value?.streak ?? 0)
</script>

<template>
  <section v-if="visible" class="pane recap" aria-label="Your week">
    <div class="flex">
      <p class="head">Your week</p>
      <p>
        <strong>{{ done }}</strong> done<template v-if="onTimePct != null"> · <strong>{{ onTimePct }}%</strong> on time</template>
      </p>
      <p v-if="change || streak" class="muted small">
        {{ change }}<template v-if="change && streak"> · </template>
        <span v-if="streak" class="cf"><Sprout :size="13" /> {{ streak }} carefree {{ streak === 1 ? 'day' : 'days' }}</span>
      </p>
    </div>
    <button class="icon-btn" aria-label="Hide until next week" @click="dismiss"><X :size="18" /></button>
  </section>
</template>

<style scoped>
.recap { flex: none; display: flex; align-items: flex-start; gap: 8px; padding: 12px 8px 12px 16px; margin: 4px 0 8px; }
.recap p { margin: 0; }
.flex { flex: 1; display: flex; flex-direction: column; gap: 3px; }
.head { font-size: 0.8rem; font-weight: 500; color: var(--muted); }
.cf { display: inline-flex; align-items: center; gap: 3px; }
</style>
