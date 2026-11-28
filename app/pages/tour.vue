<script setup lang="ts">
// App tour: one card per idea, swiped (or Next / arrow keys) sideways.
// First launch: then the permission steps (Android) or the list.
// `?again` (from Settings): back to where it was opened.
import { ArrowLeftRight, Bell, Check, ChevronLeft, Repeat, Sprout, Type } from 'lucide-vue-next'

const route = useRoute()
const router = useRouter()
const goHome = useGoHome()
const desktop = !isAndroid

const cards = [
  { key: 'hello', title: 'Befikr means carefree' },
  { key: 'type', icon: Type, title: 'Just type it' },
  { key: 'remind', icon: Bell, title: 'Reminders that stay' },
  { key: 'list', title: 'Your day at a glance' },
  { key: 'repeat', icon: Repeat, title: 'Repeats that fit' },
  { key: 'sync', icon: ArrowLeftRight, title: 'Your devices, no cloud' },
  { key: 'garden', icon: Sprout, title: 'Carefree days' },
]
const last = cards.length - 1

const track = ref<HTMLElement>()
const index = ref(0)
// Set while a Next / arrow scroll is under way, so quick presses add up.
let target: number | null = null
const onScroll = () => {
  const el = track.value
  if (el && target == null) index.value = Math.round(el.scrollLeft / el.clientWidth)
}
const onScrollEnd = () => {
  target = null
  onScroll()
}
function go(i: number) {
  const el = track.value
  if (!el) return
  target = index.value = Math.max(0, Math.min(last, i))
  const still = matchMedia('(prefers-reduced-motion: reduce)').matches
  el.scrollTo({ left: target * el.clientWidth, behavior: still ? 'instant' : 'smooth' })
  if (still) onScrollEnd()
}

async function finish() {
  markTourSeen()
  if (route.query.again != null) return router.back()
  if (await needsSetup().catch(() => false)) return router.replace('/welcome')
  goHome()
}

function key(e: KeyboardEvent) {
  if (e.key === 'ArrowRight') go(index.value + 1)
  else if (e.key === 'ArrowLeft') go(index.value - 1)
  else if (e.key === 'Enter' && index.value === last) finish()
  else return
  e.preventDefault()
}
onMounted(() => window.addEventListener('keydown', key))
onUnmounted(() => window.removeEventListener('keydown', key))
</script>

<template>
  <main class="tour">
    <div class="top">
      <button v-if="index > 0" class="icon-btn" aria-label="Back" @click="go(index - 1)"><ChevronLeft :size="22" /></button>
      <span class="flex" />
      <button v-if="index < last" class="btn" @click="finish">Skip</button>
    </div>

    <div ref="track" class="track" @scroll.passive="onScroll" @scrollend="onScrollEnd">
      <section v-for="(c, i) in cards" :key="c.key" class="card" :aria-hidden="i !== index" :aria-label="`${i + 1} of ${cards.length}: ${c.title}`">
        <div class="show">
          <AppLogo v-if="c.key === 'hello'" />

          <div v-else-if="c.key === 'type'" class="demo pane">
            <p><span>call mom </span><span class="date">fri 5pm</span></p>
            <p><span>water plants </span><span class="recurrence">every 3 days</span></p>
            <p><span>stretch </span><span class="date">in 2 hours</span></p>
          </div>

          <div v-else-if="c.key === 'remind'" class="demo pane note">
            <div class="note-head"><AppLogo class="mini" /><span class="muted small">Befikr</span></div>
            <b>Call mom</b>
            <span class="muted small">Due at 17:00</span>
            <div class="note-actions"><span>Complete</span><span>Snooze</span></div>
          </div>

          <ul v-else-if="c.key === 'list'" class="demo pane rows mini-list">
            <li><span class="checkbox" /><span class="flex">Pay rent</span><span class="small late">Yesterday</span></li>
            <li><span class="checkbox" /><span class="flex">Call mom</span><span class="small muted">17:00</span></li>
            <li><span class="checkbox checked"><Check :size="15" :stroke-width="3" /></span><span class="flex done">Buy milk</span></li>
          </ul>

          <span v-else class="badge"><component :is="c.icon" :size="34" /></span>
        </div>

        <h1>{{ c.title }}</h1>

        <div class="words">
          <template v-if="c.key === 'hello'">
            <p>Write down what you need to do. Befikr reminds you on time, and keeps at it until it's done.</p>
            <p class="muted">Your tasks stay on your devices: no account, no server, nothing tracked.</p>
          </template>
          <template v-else-if="c.key === 'type'">
            <p>Dates, times and repeats are picked out as you write, in plain English.</p>
            <p class="muted">Tap a highlight to keep those words as text.<template v-if="desktop"> Press N to add a task.</template></p>
          </template>
          <template v-else-if="c.key === 'remind'">
            <p>Complete, Snooze, or Skip a repeat right from the notification.</p>
            <p class="muted">Swipe it away and it comes back later.<template v-if="desktop"> Close the window and Befikr keeps reminding from the tray.</template></p>
          </template>
          <template v-else-if="c.key === 'list'">
            <p v-if="desktop">Tick the circle to complete. Right-click a task to pin, snooze, skip or delete it.</p>
            <p v-else>Swipe right to complete, left to snooze. Long-press a task to pin, skip or delete it.</p>
            <p class="muted">Tasks are grouped by day; overdue ones stand out.</p>
          </template>
          <template v-else-if="c.key === 'repeat'">
            <p>Some repeats stay on their dates, like rent on the 1st. Others count from when you finish, like watering plants.</p>
            <p class="muted">Skip one when it doesn't apply; the next is set up for you.</p>
          </template>
          <template v-else-if="c.key === 'sync'">
            <p>Pair your phone and computer with a QR code in Settings → Sync. They talk directly, encrypted end to end.</p>
            <p class="muted">Daily backups go to a folder you pick.</p>
          </template>
          <template v-else>
            <p>Finish on time and your streak grows, and a small garden with it.</p>
            <p class="muted">Stats show how you're doing: on time, late, snoozed.</p>
          </template>
        </div>
      </section>
    </div>

    <div class="bottom">
      <div class="dots" role="tablist" aria-label="Tour cards">
        <button
          v-for="(c, i) in cards"
          :key="c.key"
          class="dot"
          :class="{ on: i === index }"
          role="tab"
          :aria-selected="i === index"
          :aria-label="c.title"
          @click="go(i)"
        />
      </div>
      <button v-if="index < last" class="btn primary" @click="go(index + 1)">Next</button>
      <button v-else class="btn primary" @click="finish">{{ route.query.again != null ? 'Done' : 'Start' }}</button>
    </div>
  </main>
</template>

<style scoped>
/* Pinned between the status and gesture bars, like the home screen
   (index.vue): body already pads the safe areas, so a 100dvh box with its
   own inset padding came out taller than the screen, with the gap on top
   and the Next button below the fold (#29). */
.tour {
  position: fixed; top: env(safe-area-inset-top); bottom: env(safe-area-inset-bottom); left: 0; right: 0;
  max-width: 640px; margin: 0 auto; box-sizing: border-box;
  display: flex; flex-direction: column; padding: 8px 0 16px;
}
.top, .bottom { display: flex; align-items: center; gap: 12px; padding: 0 16px; min-height: 48px; }
.flex { flex: 1; }
.track {
  flex: 1; min-height: 0; display: flex; overflow-x: auto; overflow-y: hidden;
  scroll-snap-type: x mandatory; scrollbar-width: none; overscroll-behavior-x: contain;
}
.track::-webkit-scrollbar { display: none; }
.card {
  flex: 0 0 100%; scroll-snap-align: center; scroll-snap-stop: always; box-sizing: border-box;
  display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 16px;
  padding: 0 24px; text-align: center; overflow-y: auto;
}
.show { min-height: 150px; display: grid; place-items: center; }
h1 { margin: 0; font-size: 1.5rem; letter-spacing: -0.02em; }
.words { max-width: 380px; }
.words p { margin: 0 0 8px; line-height: 1.45; }

.badge {
  width: 84px; height: 84px; border-radius: 26px; display: grid; place-items: center;
  background: var(--glass-control);
  box-shadow: inset 0 1px 0 var(--glass-control-rim), inset 0 0 0 1px var(--glass-edge), 0 8px 18px var(--glass-shade);
}

.demo { text-align: left; width: min(300px, 80vw); box-sizing: border-box; }
.demo p { margin: 6px 0; font-size: 1.05rem; }
.date, .recurrence { border-radius: 6px; padding: 0 2px; }
.date { background: color-mix(in oklch, var(--accent) 22%, transparent); }
.recurrence { background: color-mix(in oklch, var(--yellow) 35%, transparent); }

.note { display: flex; flex-direction: column; gap: 2px; }
.note-head { display: flex; align-items: center; gap: 6px; margin-bottom: 4px; }
.mini { width: 18px; height: 18px; filter: none; }
.note-actions { display: flex; gap: 20px; margin-top: 10px; font-weight: 600; font-size: 0.9rem; }

.mini-list { list-style: none; margin: 0; }
.mini-list li { display: flex; align-items: center; gap: 12px; padding: 10px 0; }
.mini-list .checkbox { flex: none; }
.done { text-decoration: line-through; opacity: 0.55; }
.late { color: var(--red); font-weight: 600; }

.dots { flex: 1; display: flex; gap: 8px; }
.dot {
  width: 8px; height: 8px; padding: 0; border: 0; border-radius: 50%; cursor: pointer;
  background: color-mix(in oklch, var(--text) 22%, transparent);
}
.dot.on { background: var(--accent); width: 20px; border-radius: 4px; }
</style>
