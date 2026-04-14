<script setup lang="ts">
defineProps<{
  variant?: "primary" | "secondary" | "success" | "warning" | "danger";
  size?: "sm" | "md" | "small";
}>();
</script>

<template>
  <button :class="[variant ?? 'primary', size ?? 'md']">
    <slot />
  </button>
</template>

<style scoped>
button {
  cursor: pointer;
  border: none;
  border-radius: 6px;
  font-weight: 500;
  transition: opacity 0.15s;
}

button.md {
  padding: 8px 16px;
  font-size: 0.875rem;
}

button.sm,
button.small {
  padding: 4px 10px;
  font-size: 0.8rem;
}

button.primary {
  background-color: var(--accent);
  color: white;
}

button.secondary {
  background-color: var(--surface);
  color: var(--text);
  border: 1px solid var(--border);
}

button.success {
  background-color: var(--success);
  color: white;
}

button.warning {
  background-color: var(--warning);
  color: white;
}

button.danger {
  background-color: var(--danger);
  color: white;
}

button:hover:not(:disabled) {
  opacity: 0.9;
}

button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
