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
  box-sizing: border-box;
  width: 100%;
  min-width: 0;
  word-wrap: break-word;
}

@media (max-width: 600px) {
  .empty-state {
    padding: var(--space-xl);
    font-size: 13px;
  }
}
</style>
