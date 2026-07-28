<script setup lang="ts">
defineProps<{
  modelValue: string | null | undefined;
  placeholder?: string;
  id?: string;
  name?: string;
  rows?: number;
  required?: boolean;
  disabled?: boolean;
  readonly?: boolean;
  maxlength?: number | string;
}>();

defineEmits<{
  "update:modelValue": [value: string];
}>();
</script>

<template>
  <textarea
    :id="id"
    :name="name"
    :value="modelValue ?? ''"
    :placeholder="placeholder"
    :rows="rows ?? 3"
    :required="required"
    :disabled="disabled"
    :readonly="readonly"
    :maxlength="maxlength"
    @input="$emit('update:modelValue', ($event.target as HTMLTextAreaElement).value)"
  />
</template>
