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
  border-radius: 4px;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  white-space: nowrap;
}

.badge--info {
  background-color: rgba(91, 192, 235, 0.15);
  color: var(--info);
}

.badge--warn,
.badge--warning {
  background-color: rgba(254, 231, 92, 0.15);
  color: var(--warning);
}

.badge--error,
.badge--danger {
  background-color: rgba(237, 66, 69, 0.15);
  color: var(--danger);
}

.badge--success {
  background-color: rgba(87, 242, 135, 0.15);
  color: var(--success);
}

.badge--default {
  background-color: var(--bg-hover);
  color: var(--text-secondary);
}
</style>
