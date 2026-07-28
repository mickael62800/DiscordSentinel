<script setup lang="ts">
withDefaults(defineProps<{ message?: string }>(), {
  message: "Chargement...",
});
</script>

<template>
  <div class="loading-state">{{ message }}</div>
</template>

<style scoped>
.loading-state {
  color: var(--text-secondary);
  padding: 40px;
  text-align: center;
}
</style>
