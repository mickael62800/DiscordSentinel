<script setup lang="ts">
defineProps<{
  label: string;
  value: string | number;
  color?: string;
}>();
</script>

<template>
  <div class="card card--lg stat-card">
    <span class="stat-value" :style="color ? { color } : {}">{{ value }}</span>
    <span class="stat-label">{{ label }}</span>
  </div>
</template>

<style scoped>
.stat-card {
  display: flex;
  flex-direction: column;
  gap: var(--space-xs);
}

.stat-value {
  font-size: 28px;
  font-weight: 700;
  color: var(--text-primary);
}

.stat-label {
  font-size: 13px;
  color: var(--text-secondary);
  font-weight: 500;
}
</style>
