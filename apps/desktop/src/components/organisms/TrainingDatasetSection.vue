<script setup lang="ts">
import type { DatasetInfo, ModelType } from "../../composables/useAiTraining";

defineProps<{
  dataset: DatasetInfo | undefined;
  loading: boolean;
  modelType: ModelType;
  disabled: boolean;
}>();

const emit = defineEmits<{
  upload: [];
}>();

</script>

<template>
  <section class="section-card">
    <div class="section-header">
      <h3>Dataset</h3>
      <button class="btn btn-secondary" @click="emit('upload')" :disabled="disabled">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="btn-icon">
          <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4" />
          <polyline points="17 8 12 3 7 8" />
          <line x1="12" y1="3" x2="12" y2="15" />
        </svg>
        Importer un dataset
      </button>
    </div>

    <div v-if="loading" class="loading-text">Chargement...</div>

    <div v-else-if="!dataset" class="empty-state">
      <p>Aucun dataset importe pour ce modele.</p>
      <p class="hint">
        <template v-if="modelType === 'text-sentiment'">
          Format attendu : CSV ou JSON avec colonnes <code>text</code> et <code>label</code>
          (anger, threat, harassment, spam, safe)
        </template>
        <template v-else>
          Format attendu : JSON avec chemins d'images et labels
          (nsfw, illicit, safe)
        </template>
      </p>
    </div>

    <div v-else class="dataset-stats">
      <div class="stat-card">
        <span class="stat-value">{{ dataset.total_samples.toLocaleString() }}</span>
        <span class="stat-label">Echantillons</span>
      </div>
      <div
        v-for="(count, label) in dataset.label_distribution"
        :key="label"
        class="stat-card"
      >
        <span class="stat-value">{{ count.toLocaleString() }}</span>
        <span class="stat-label">{{ label }}</span>
      </div>
      <div v-if="dataset.last_updated" class="stat-card">
        <span class="stat-value stat-date">{{ dataset.last_updated }}</span>
        <span class="stat-label">Derniere mise a jour</span>
      </div>
    </div>
  </section>
</template>

<style scoped>
.section-card {
  background: var(--bg-secondary, #1e1e2e);
  border-radius: 12px;
  padding: 1.5rem;
  margin-bottom: 1.5rem;
}

.section-card h3 {
  margin: 0 0 1rem;
  font-size: 1rem;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1rem;
}

.section-header h3 {
  margin: 0;
}

.dataset-stats {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
}

.stat-card {
  background: var(--bg-primary, #161622);
  border-radius: 8px;
  padding: 12px 16px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 100px;
}

.stat-value {
  font-size: 1.2rem;
  font-weight: 700;
  color: var(--text-primary, #fff);
}

.stat-value.stat-date {
  font-size: 0.85rem;
}

.stat-label {
  font-size: 0.75rem;
  color: var(--text-secondary, #888);
  text-transform: capitalize;
}

.empty-state {
  text-align: center;
  padding: 1.5rem;
  color: var(--text-secondary, #888);
}

.empty-state .hint {
  font-size: 0.8rem;
  margin-top: 0.5rem;
}

.empty-state code {
  background: var(--bg-primary, #161622);
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 0.8rem;
}

.loading-text {
  text-align: center;
  padding: 1rem;
  color: var(--text-secondary, #888);
}

.btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  border: none;
  border-radius: 8px;
  font-size: 0.85rem;
  font-weight: 600;
  cursor: pointer;
  transition: opacity 0.15s;
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn:hover:not(:disabled) {
  opacity: 0.85;
}

.btn-secondary {
  background: var(--bg-hover, #2a2a3e);
  color: var(--text-primary, #fff);
}

.btn-icon {
  width: 16px;
  height: 16px;
}
</style>
