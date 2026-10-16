<script setup lang="ts">
// Old address of the task view, now a sheet: open it over the list.
const route = useRoute()
const router = useRouter()
onMounted(() => router.replace({ path: '/', query: { task: String(route.params.id) } }))
</script>

<template>
  <main class="page" />
</template>
