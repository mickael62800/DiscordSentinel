<script setup lang="ts">
defineProps<{
  currentPage: number;
  totalPages: number;
  totalItems: number;
  perPage: number;
}>();

defineEmits<{
  "update:currentPage": [page: number];
  "update:perPage": [size: number];
}>();

const perPageOptions = [10, 25, 50, 100];
</script>

<template>
  <div class="card pagination-bar" v-if="totalItems > 0">
    <div class="pagination-info">
      {{ totalItems }} resultat(s) - Page {{ currentPage }} / {{ totalPages }}
    </div>

    <div class="pagination-controls">
      <button
        class="pagination-btn"
        :disabled="currentPage <= 1"
        @click="$emit('update:currentPage', 1)"
        title="Premiere page"
      >
        &laquo;
      </button>
      <button
        class="pagination-btn"
        :disabled="currentPage <= 1"
        @click="$emit('update:currentPage', currentPage - 1)"
        title="Page precedente"
      >
        &lsaquo;
      </button>

      <template v-for="page in totalPages" :key="page">
        <button
          v-if="page === 1 || page === totalPages || (page >= currentPage - 2 && page <= currentPage + 2)"
          class="pagination-btn"
          :class="{ active: page === currentPage }"
          @click="$emit('update:currentPage', page)"
        >
          {{ page }}
        </button>
        <span
          v-else-if="page === currentPage - 3 || page === currentPage + 3"
          class="pagination-dots"
        >...</span>
      </template>

      <button
        class="pagination-btn"
        :disabled="currentPage >= totalPages"
        @click="$emit('update:currentPage', currentPage + 1)"
        title="Page suivante"
      >
        &rsaquo;
      </button>
      <button
        class="pagination-btn"
        :disabled="currentPage >= totalPages"
        @click="$emit('update:currentPage', totalPages)"
        title="Derniere page"
      >
        &raquo;
      </button>
    </div>

    <div class="pagination-size">
      <select
        :value="perPage"
        @change="$emit('update:perPage', Number(($event.target as HTMLSelectElement).value))"
      >
        <option v-for="size in perPageOptions" :key="size" :value="size">
          {{ size }} par page
        </option>
      </select>
    </div>
  </div>
</template>

<style scoped>
.pagination-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-md) var(--space-lg);
  margin-top: var(--space-lg);
  border-radius: 10px; /* override .card pour un look plus compact */
  font-size: 13px;
  color: var(--text-secondary);
}

.pagination-info {
  white-space: nowrap;
}

.pagination-controls {
  display: flex;
  align-items: center;
  gap: var(--space-xs);
}

.pagination-btn {
  min-width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-primary);
  cursor: pointer;
  font-size: 13px;
  transition: all var(--transition-fast);
}

.pagination-btn:hover:not(:disabled) {
  background-color: var(--bg-hover);
}

.pagination-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.pagination-btn.active {
  background-color: var(--accent);
  color: white;
  border-color: var(--accent);
}

.pagination-dots {
  padding: 0 var(--space-xs);
  color: var(--text-secondary);
}

.pagination-size select {
  padding: 6px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background-color: var(--bg-card);
  color: var(--text-primary);
  font-size: 13px;
  cursor: pointer;
}

@media (max-width: 640px) {
  .pagination-bar {
    flex-wrap: wrap;
    gap: 8px;
    padding: var(--space-sm) var(--space-md);
  }
  .pagination-info { font-size: 12px; }
  .pagination-controls {
    flex-wrap: wrap;
    justify-content: center;
    flex: 1 1 100%;
  }
  .pagination-btn { min-width: 28px; height: 28px; font-size: 12px; }
}
</style>
