<script setup lang="ts">
import type { SelectOption } from "../../types";

defineProps<{
  modelValue: string | number | null | undefined;
  options?: SelectOption[];
  id?: string;
  name?: string;
  required?: boolean;
  disabled?: boolean;
}>();

defineEmits<{
  "update:modelValue": [value: string];
  change: [event: Event];
}>();
</script>

<template>
  <select
    :id="id"
    :name="name"
    :value="modelValue ?? ''"
    :required="required"
    :disabled="disabled"
    @change="(e) => {
      $emit('update:modelValue', (e.target as HTMLSelectElement).value);
      $emit('change', e);
    }"
  >
    <template v-if="options">
      <option v-for="opt in options" :key="opt.value" :value="opt.value">
        {{ opt.label }}
      </option>
    </template>
    <slot v-else />
  </select>
</template>
