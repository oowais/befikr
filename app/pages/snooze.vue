<script setup lang="ts">
// Old address of the snooze screen, still in notifications scheduled before
// it became a sheet: open the sheet over the list instead.
const route = useRoute()
const router = useRouter()
onMounted(() => router.replace({ path: '/', query: { snooze: String(route.query.id ?? '') } }))
</script>

<template>
  <main class="page" />
</template>
