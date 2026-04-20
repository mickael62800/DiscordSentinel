<script setup lang="ts">
withDefaults(defineProps<{ message?: string }>(), {
  message: "Aucun resultat",
});
</script>

<template>
  <div class="card empty-state">{{ message }}</div>
</template>

<style scoped>
.empty-state {
  color: var(--text-secondary);
  padding: var(--space-3xl);
  text-align: center;
}
</style>
