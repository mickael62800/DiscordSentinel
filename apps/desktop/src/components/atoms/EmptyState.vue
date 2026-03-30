<script setup lang="ts">
withDefaults(defineProps<{ message?: string }>(), {
  message: "Aucun resultat",
});
</script>

<template>
  <div class="empty-state">{{ message }}</div>
</template>

<style scoped>
.empty-state {
  color: var(--text-secondary);
  padding: 40px;
  text-align: center;
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
}
</style>
