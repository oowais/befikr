<script setup lang="ts">
// Sheets opened by the URL (`?task=`, `?snooze=`), over whatever page is
// open. Snooze comes after the task, so it sits on top when both are open.
const { id: taskId, close: closeTask } = useTaskSheet()
const { id: snoozeId, close: closeSnooze } = useSnoozeSheet()
</script>

<template>
  <TaskSheet v-if="taskId" :key="taskId" :task-id="taskId" @close="closeTask" />
  <SnoozeSheet v-if="snoozeId" :key="snoozeId" :task-id="snoozeId" @close="closeSnooze" @done="closeSnooze" />
</template>
