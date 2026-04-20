<script setup lang="ts">
defineProps<{
  title: string;
}>();
</script>

<template>
  <div class="card card--lg settings-section">
    <h2>{{ title }}</h2>
    <slot />
  </div>
</template>

<style scoped>
.settings-section {
  margin-bottom: var(--space-lg);
}

.settings-section h2 {
  font-size: 16px;
  font-weight: 600;
  margin-bottom: var(--space-lg);
}
</style>
