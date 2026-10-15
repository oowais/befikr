<script setup lang="ts">
// Shopping list: separate from tasks, no reminders. Top: what to buy (tap to
// tick off; it clears away, Undo in the toast). Below: chips of things
// usually bought; tap one to add it. Edit shows ✕ on chips and a field for
// new ones.
import { ArrowLeft, Check, Pencil, Plus, X } from 'lucide-vue-next'
import type { ShoppingItem, Staple } from '~/composables/api'

const router = useRouter()
const { show } = useToast()
const sound = useSound()
const items = ref<ShoppingItem[]>([])
const staples = ref<Staple[]>([])
const typed = ref('')
const newStaple = ref('')
const editing = ref(false)
/** Ticked, fading out before they leave the list. */
const ticked = ref(new Set<string>())
const error = ref('')

const load = async () => {
  try {
    const l = await api.shoppingList()
    items.value = l.items
    staples.value = l.staples
  } catch (e) {
    error.value = String(e)
  }
}
onMounted(load)

const key = (s: string) => s.trim().toLowerCase()
const onList = computed(() => new Set(items.value.map((i) => key(i.name))))

async function add(name: string) {
  name = name.replace(/\s+/g, ' ').trim()
  if (!name || onList.value.has(key(name))) return
  haptic()
  const temp: ShoppingItem = { id: `temp-${Date.now()}`, name, boughtAt: null }
  items.value = [...items.value, temp]
  try {
    const real = await api.addShoppingItem(name)
    items.value = items.value.map((i) => (i.id === temp.id ? real : i))
  } catch (e) {
    error.value = String(e)
    load()
  }
}

function submit() {
  add(typed.value)
  typed.value = ''
}

async function tick(item: ShoppingItem) {
  if (ticked.value.has(item.id)) return
  haptic()
  sound.tick()
  ticked.value = new Set(ticked.value).add(item.id)
  try {
    await api.setShoppingBought(item.id, true)
  } catch (e) {
    error.value = String(e)
  }
  setTimeout(() => {
    items.value = items.value.filter((i) => i.id !== item.id)
    ticked.value.delete(item.id)
  }, 350)
  show({ message: `Got ${item.name}`, action: { label: 'Undo', run: () => api.setShoppingBought(item.id, false).then(load) } })
}

async function addStaple() {
  const name = newStaple.value.replace(/\s+/g, ' ').trim()
  newStaple.value = ''
  if (!name || staples.value.some((s) => key(s.name) === key(name))) return
  try {
    const s = await api.addStaple(name)
    staples.value = [...staples.value, s].sort((a, b) => a.name.localeCompare(b.name, undefined, { sensitivity: 'base' }))
  } catch (e) {
    error.value = String(e)
  }
}

async function removeStaple(s: Staple) {
  haptic()
  staples.value = staples.value.filter((x) => x.id !== s.id)
  await api.removeStaple(s.id).catch((e) => {
    error.value = String(e)
    load()
  })
}
</script>

<template>
  <main class="page">
    <header class="topbar">
      <button class="icon-btn" aria-label="Back" @click="router.back()"><ArrowLeft :size="22" /></button>
      <h1>Shopping</h1>
    </header>

    <form class="add" @submit.prevent="submit">
      <input
        :value="typed"
        class="field"
        placeholder="Add item"
        enterkeyhint="done"
        autocorrect="on"
        autocapitalize="sentences"
        @input="typed = ($event.target as HTMLInputElement).value"
      />
      <button class="btn primary" :disabled="!typed.trim()" aria-label="Add"><Plus :size="18" /></button>
    </form>

    <p v-if="error" class="error">{{ error }}</p>

    <TransitionGroup v-if="items.length" tag="ul" name="list" class="pane rows list">
      <li v-for="i in items" :key="i.id">
        <button class="item" :class="{ done: ticked.has(i.id) }" :aria-label="`Got ${i.name}`" @click="tick(i)">
          <span class="checkbox" :class="{ checked: ticked.has(i.id) }"><Check :size="14" :stroke-width="3" /></span>
          <span class="name">{{ i.name }}</span>
        </button>
      </li>
    </TransitionGroup>
    <p v-else class="muted empty">Nothing to buy.</p>

    <div class="usual-head">
      <h2 class="section-title">Usually</h2>
      <button v-if="staples.length || editing" class="icon-btn" :aria-label="editing ? 'Done editing' : 'Edit chips'" @click="editing = !editing">
        <Check v-if="editing" :size="18" /><Pencil v-else :size="18" />
      </button>
    </div>
    <div class="chips">
      <template v-if="editing">
        <button v-for="s in staples" :key="s.id" class="chip" :aria-label="`Remove ${s.name}`" @click="removeStaple(s)">
          {{ s.name }} <X :size="14" />
        </button>
      </template>
      <template v-else>
        <button
          v-for="s in staples"
          :key="s.id"
          class="chip"
          :class="{ off: onList.has(key(s.name)) }"
          :aria-label="onList.has(key(s.name)) ? `${s.name} is on the list` : `Add ${s.name}`"
          @click="add(s.name)"
        >
          <Check v-if="onList.has(key(s.name))" :size="14" />{{ s.name }}
        </button>
        <button v-if="!staples.length" class="chip off" @click="editing = true"><Plus :size="14" /> Add things you usually buy</button>
      </template>
    </div>
    <form v-if="editing" class="add staple" @submit.prevent="addStaple">
      <input
        :value="newStaple"
        class="field"
        placeholder="New chip, e.g. milk"
        enterkeyhint="done"
        autocorrect="on"
        @input="newStaple = ($event.target as HTMLInputElement).value"
      />
      <button class="btn" :disabled="!newStaple.trim()" aria-label="Add chip"><Plus :size="18" /></button>
    </form>
  </main>
</template>

<style scoped>
.add { display: flex; gap: 8px; margin-top: 4px; }
.add .btn { flex: none; padding: 8px 12px; }
.list { list-style: none; margin: 12px 0 0; }
.item { display: flex; align-items: center; gap: 12px; width: 100%; padding: 12px 0; text-align: left; transition: opacity 0.3s; }
.item.done { opacity: 0.45; }
.item.done .name { text-decoration: line-through; }
.name { flex: 1; min-width: 0; overflow-wrap: anywhere; }
.empty { margin: 16px 0 0; }
.usual-head { display: flex; align-items: center; justify-content: space-between; margin-top: 16px; }
.usual-head .section-title { margin: 8px 0; }
.chips { display: flex; flex-wrap: wrap; gap: 8px; }
.staple { margin-top: 12px; }
.list-leave-active { transition: opacity 0.2s, transform 0.2s; }
.list-leave-to { opacity: 0; transform: translateX(16px); }
</style>
