<script setup lang="ts">
defineProps<{
  modelValue: string | number;
  type?: string;
  placeholder?: string;
  min?: number;
  max?: number;
}>();

defineEmits<{
  "update:modelValue": [value: string | number];
}>();
</script>

<template>
  <input
    :type="type ?? 'text'"
    :value="modelValue"
    :placeholder="placeholder"
    :min="min"
    :max="max"
    @input="$emit('update:modelValue', ($event.target as HTMLInputElement).value)"
  />
</template>
