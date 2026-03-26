<script setup lang="ts">
defineProps<{
  status: "online" | "offline" | "warning";
}>();
</script>

<template>
  <span :class="['status-dot', `status-dot--${status}`]"></span>
</template>

<style scoped>
.status-dot {
  display: inline-block;
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.status-dot--online {
  background-color: var(--success);
  box-shadow: 0 0 6px var(--success);
}

.status-dot--offline {
  background-color: var(--danger);
  box-shadow: 0 0 6px var(--danger);
}

.status-dot--warning {
  background-color: var(--warning);
  box-shadow: 0 0 6px var(--warning);
}
</style>
