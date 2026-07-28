<script setup lang="ts">
defineProps<{
  modelValue: string | number | null | undefined;
  type?: string;
  placeholder?: string;
  min?: number;
  max?: number;
  id?: string;
  name?: string;
  required?: boolean;
  disabled?: boolean;
  readonly?: boolean;
  maxlength?: number | string;
  autocomplete?: string;
  pattern?: string;
}>();

defineEmits<{
  "update:modelValue": [value: string | number];
}>();
</script>

<template>
  <input
    :id="id"
    :name="name"
    :type="type ?? 'text'"
    :value="modelValue ?? ''"
    :placeholder="placeholder"
    :min="min"
    :max="max"
    :required="required"
    :disabled="disabled"
    :readonly="readonly"
    :maxlength="maxlength"
    :autocomplete="autocomplete"
    :pattern="pattern"
    @input="$emit('update:modelValue', ($event.target as HTMLInputElement).value)"
  />
</template>
