<script setup lang="ts">
// One line under the top bar while no device has synced for a few hours
// (the dot is orange): the list here may be missing edits made elsewhere,
// and sync is peer to peer, so the other device has to be running. Gone
// as soon as a sync lands.
const { st, state, lastSync, now } = useSyncStatus()

const text = computed(() => {
  const peers = st.value?.peers ?? []
  const where = peers.length === 1 ? peers[0]!.name || 'your other device' : 'your other devices'
  const since = lastSync.value ? `Not synced for ${span(now.value - lastSync.value)}` : 'Not synced yet'
  return `${since} · open Befikr on ${where}`
})
</script>

<template>
  <NuxtLink v-if="state === 'stale'" to="/sync" class="stale-hint muted small">
    <span class="dot orange" />{{ text }}
  </NuxtLink>
</template>

<style scoped>
.stale-hint {
  flex: none; display: flex; align-items: center; gap: 8px;
  margin: 2px 0 12px; color: var(--muted); text-decoration: none;
}
</style>
