<script setup lang="ts">
defineProps<{
  text?: string;
}>();
</script>

<template>
  <p v-if="text" class="field-description">{{ text }}</p>
</template>

<style scoped>
.field-description {
  margin: 0;
  font-size: 12px;
  line-height: 1.5;
  color: var(--text-secondary);
  max-width: 100%;
}
</style>
