<script setup lang="ts">
import type { SelectOption } from "../../types";
import AppSelect from "../atoms/AppSelect.vue";

defineProps<{
  filters: Array<{
    modelValue: string;
    options: SelectOption[];
  }>;
}>();

defineEmits<{
  "update:filter": [index: number, value: string];
}>();
</script>

<template>
  <div class="filter-bar">
    <AppSelect
      v-for="(filter, index) in filters"
      :key="index"
      :model-value="filter.modelValue"
      :options="filter.options"
      @update:model-value="$emit('update:filter', index, $event)"
    />
  </div>
</template>

<style scoped>
.filter-bar {
  display: flex;
  gap: 12px;
  margin-bottom: 16px;
}
</style>
