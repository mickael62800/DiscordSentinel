<script setup lang="ts">
defineProps<{
  label: string;
  hint?: string;
  error?: string;
}>();
</script>

<template>
  <div class="form-field">
    <label>{{ label }}</label>
    <slot />
    <span v-if="hint" class="hint">{{ hint }}</span>
    <span v-if="error" class="field-error">{{ error }}</span>
  </div>
</template>

<style scoped>
.form-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.form-field label {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.hint {
  font-size: 11px;
  color: var(--text-secondary);
  opacity: 0.7;
}

.field-error {
  font-size: 11px;
  color: var(--danger);
}
</style>
