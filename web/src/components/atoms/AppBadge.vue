<script setup lang="ts">
defineProps<{
  label: string;
  variant?: "info" | "warn" | "error" | "danger" | "warning" | "success" | "default";
}>();
</script>

<template>
  <span :class="['badge', `badge--${variant ?? 'default'}`]">{{ label }}</span>
</template>

<style scoped>
.badge {
  display: inline-block;
  padding: 2px 8px;
  border-radius: var(--radius-sm);
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  white-space: nowrap;
}

.badge--info {
  background-color: var(--info-bg);
  color: var(--info);
}

.badge--warn,
.badge--warning {
  background-color: var(--warning-bg);
  color: var(--warning);
}

.badge--error,
.badge--danger {
  background-color: var(--danger-bg);
  color: var(--danger);
}

.badge--success {
  background-color: var(--success-bg);
  color: var(--success);
}

.badge--default {
  background-color: var(--bg-hover);
  color: var(--text-secondary);
}
</style>
