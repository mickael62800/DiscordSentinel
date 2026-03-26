<script setup lang="ts">
defineProps<{
  title: string;
}>();
</script>

<template>
  <div class="settings-section">
    <h2>{{ title }}</h2>
    <slot />
  </div>
</template>

<style scoped>
.settings-section {
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 24px;
  margin-bottom: 16px;
}

.settings-section h2 {
  font-size: 16px;
  font-weight: 600;
  margin-bottom: 16px;
}
</style>
