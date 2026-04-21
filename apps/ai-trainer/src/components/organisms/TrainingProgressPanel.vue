<script setup lang="ts">
import { computed } from "vue";
import type { TrainingStatus, ModelType } from "../../composables/useAiTraining";

const props = defineProps<{
  status: TrainingStatus;
  stopping: boolean;
  canStart: boolean;
  isTrainingThis: boolean;
  isTrainingOther: boolean;
  activeTab: ModelType;
  hasDataset: boolean;
}>();

const emit = defineEmits<{
  start: [];
  stop: [];
}>();

const pct = (v: number) => `${(v * 100).toFixed(1)}%`;

const CANONICAL_ORDER: Record<ModelType, string[]> = {
  "image-classification": ["safe", "nsfw", "illicit"],
  "text-sentiment": ["neutral", "toxic_light", "toxic_severe"],
};

const classNames = computed(() => {
  const fm = props.status.final_metrics;
  if (fm?.class_names && fm.class_names.length > 0) return fm.class_names;
  const mt = props.status.model_type;
  if (mt && CANONICAL_ORDER[mt]) {
    const canonical = CANONICAL_ORDER[mt];
    if (fm?.per_class && canonical.every((n) => n in fm.per_class)) {
      return canonical;
    }
  }
  if (fm?.per_class) return Object.keys(fm.per_class);
  return [];
});

const perClassRows = computed(() => {
  const fm = props.status.final_metrics;
  if (!fm?.per_class) return [];
  return classNames.value
    .map((name) => {
      const m = fm.per_class[name];
      if (!m) return null;
      return { name, precision: m.precision, recall: m.recall, f1: m.f1, support: m.support };
    })
    .filter((r): r is NonNullable<typeof r> => r !== null);
});

const hasConfusion = computed(() => {
  const cm = props.status.final_metrics?.confusion_matrix;
  return Array.isArray(cm) && cm.length > 0 && Array.isArray(cm[0]) && cm[0].length > 0;
});

const progressPercent = computed(() => {
  if (!props.status.total_epochs) return 0;
  const epochProgress = props.status.total_batches
    ? props.status.current_batch / props.status.total_batches
    : 0;
  const completedEpochs = Math.max(props.status.current_epoch - 1, 0);
  return Math.round(((completedEpochs + epochProgress) / props.status.total_epochs) * 100);
});

const batchPercent = computed(() => {
  if (!props.status.total_batches) return 0;
  return Math.round((props.status.current_batch / props.status.total_batches) * 100);
});
</script>

<template>
  <section class="section-card">
    <div class="section-header">
      <h3>Entrainement</h3>
      <div class="actions">
        <button
          v-if="!status.running"
          class="btn btn-primary"
          :disabled="!canStart || stopping"
          @click="emit('start')"
        >
          {{ stopping ? "Arret en cours..." : "Lancer l'entrainement" }}
        </button>
        <button
          v-else
          class="btn btn-danger"
          :disabled="stopping"
          @click="emit('stop')"
        >
          {{ stopping ? "Arret en cours..." : "Arreter" }}
        </button>
      </div>
    </div>

    <div v-if="isTrainingThis" class="training-progress">
      <div class="progress-header">
        <span class="phase">{{ status.phase }}</span>
        <span class="epoch-count">
          Epoch {{ status.current_epoch }} / {{ status.total_epochs }}
        </span>
      </div>
      <div class="progress-bar-container">
        <div class="progress-bar" :style="{ width: progressPercent + '%' }"></div>
      </div>

      <div v-if="status.total_batches > 0" class="batch-progress">
        <div class="batch-header">
          <span class="batch-label">Batch {{ status.current_batch }} / {{ status.total_batches }}</span>
          <span class="batch-percent">{{ batchPercent }}%</span>
        </div>
        <div class="progress-bar-container progress-bar-sm">
          <div class="progress-bar progress-bar-batch" :style="{ width: batchPercent + '%' }"></div>
        </div>
      </div>

      <div class="metrics-grid">
        <div class="metric">
          <span class="metric-label">Loss (live)</span>
          <span class="metric-value">{{ status.batch_loss.toFixed(4) }}</span>
        </div>
        <div class="metric">
          <span class="metric-label">Accuracy (live)</span>
          <span class="metric-value">{{ (status.batch_accuracy * 100).toFixed(1) }}%</span>
        </div>
        <div class="metric">
          <span class="metric-label">Val Loss</span>
          <span class="metric-value">{{ status.val_loss.toFixed(4) }}</span>
        </div>
        <div class="metric">
          <span class="metric-label">Val Accuracy</span>
          <span class="metric-value">{{ (status.val_accuracy * 100).toFixed(1) }}%</span>
        </div>
      </div>

      <div v-if="stopping" class="stopping-indicator">
        Arret en cours — le GPU termine le batch actuel, veuillez patienter...
      </div>

      <div v-if="status.best_epoch > 0" class="best-epoch-indicator">
        Meilleur modele : epoch {{ status.best_epoch }}
        <span v-if="status.current_epoch - status.best_epoch >= 2" class="patience-warning">
          — pas d'amelioration depuis {{ status.current_epoch - status.best_epoch }} epochs
        </span>
      </div>
    </div>

    <div
      v-else-if="(status.phase === 'termine' || status.early_stopped || status.phase.startsWith('early stop')) && status.model_type === activeTab"
      class="training-complete"
      :class="{ 'early-stopped': status.early_stopped }"
    >
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="check-icon">
        <path d="M22 11.08V12a10 10 0 11-5.93-9.14" />
        <polyline points="22 4 12 14.01 9 11.01" />
      </svg>
      <div>
        <strong v-if="status.early_stopped">Optimisation atteinte — arret automatique</strong>
        <strong v-else>Entrainement termine</strong>
        <p v-if="status.early_stopped" class="early-stop-detail">
          Le modele a atteint son meilleur resultat a l'epoch {{ status.best_epoch }}
          sur {{ status.current_epoch }} epochs.
          La validation ne s'ameliorait plus — le meilleur modele a ete sauvegarde automatiquement.
        </p>
        <p>
          Accuracy finale : {{ (status.val_accuracy * 100).toFixed(1) }}% —
          Loss : {{ status.val_loss.toFixed(4) }}
        </p>
      </div>
    </div>

    <div v-else class="idle-state">
      <p v-if="!canStart && !hasDataset">
        Importez un dataset pour commencer l'entrainement.
      </p>
      <p v-else-if="status.running">
        Un entrainement est deja en cours sur un autre modele.
      </p>
      <p v-else>Pret a lancer l'entrainement.</p>
    </div>

    <div
      v-if="status.final_metrics && status.model_type === activeTab"
      class="final-metrics"
    >
      <h4>Metriques finales (test set)</h4>

      <div class="macro-grid">
        <div class="macro-cell">
          <div class="macro-label">Accuracy</div>
          <div class="macro-value">{{ pct(status.final_metrics.accuracy) }}</div>
        </div>
        <div class="macro-cell">
          <div class="macro-label">Macro F1</div>
          <div class="macro-value">{{ pct(status.final_metrics.macro_f1) }}</div>
        </div>
        <div class="macro-cell">
          <div class="macro-label">Macro Precision</div>
          <div class="macro-value">{{ pct(status.final_metrics.macro_precision) }}</div>
        </div>
        <div class="macro-cell">
          <div class="macro-label">Macro Recall</div>
          <div class="macro-value">{{ pct(status.final_metrics.macro_recall) }}</div>
        </div>
      </div>

      <table class="per-class-table">
        <thead>
          <tr>
            <th>Classe</th>
            <th>Precision</th>
            <th>Recall</th>
            <th>F1</th>
            <th>Support</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="row in perClassRows" :key="row.name">
            <td class="class-name">{{ row.name }}</td>
            <td :class="{ weak: row.precision < 0.7 }">{{ pct(row.precision) }}</td>
            <td :class="{ weak: row.recall < 0.7 }">{{ pct(row.recall) }}</td>
            <td :class="{ weak: row.f1 < 0.7 }">{{ pct(row.f1) }}</td>
            <td>{{ row.support }}</td>
          </tr>
        </tbody>
      </table>

      <details v-if="hasConfusion" class="confusion-details">
        <summary>Matrice de confusion</summary>
        <table class="confusion-table">
          <thead>
            <tr>
              <th></th>
              <th v-for="cn in classNames" :key="'h-' + cn">{{ cn }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(row, i) in status.final_metrics.confusion_matrix" :key="'r-' + i">
              <th>{{ classNames[i] || i }}</th>
              <td
                v-for="(cell, j) in row"
                :key="'c-' + i + '-' + j"
                :class="{ diagonal: i === j }"
              >
                {{ cell }}
              </td>
            </tr>
          </tbody>
        </table>
        <p class="confusion-hint">Lignes = vraie classe, colonnes = predite. Diagonale en vert = correct.</p>
      </details>
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
.section-card h3 { margin: 0 0 1rem; font-size: 1rem; }

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1rem;
}
.section-header h3 { margin: 0; }

.actions { display: flex; gap: 8px; }

.training-progress { display: flex; flex-direction: column; gap: 12px; }

.progress-header { display: flex; justify-content: space-between; align-items: center; }

.phase {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--accent, #7c3aed);
  text-transform: capitalize;
}

.epoch-count { font-size: 0.85rem; color: var(--text-secondary, #888); }

.progress-bar-container {
  width: 100%;
  height: 8px;
  background: var(--bg-primary, #161622);
  border-radius: 4px;
  overflow: hidden;
}
.progress-bar {
  height: 100%;
  background: linear-gradient(90deg, var(--accent, #7c3aed), #a78bfa);
  border-radius: 4px;
  transition: width 0.3s ease;
}
.progress-bar-sm { height: 6px; }
.progress-bar-batch { background: linear-gradient(90deg, #3b82f6, #60a5fa); }

.batch-progress { display: flex; flex-direction: column; gap: 4px; }
.batch-header { display: flex; justify-content: space-between; align-items: center; }
.batch-label { font-size: 0.8rem; color: var(--text-secondary, #888); }
.batch-percent { font-size: 0.8rem; font-weight: 600; color: #3b82f6; }

.metrics-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 12px;
}
.metric {
  background: var(--bg-primary, #161622);
  border-radius: 8px;
  padding: 10px 12px;
  text-align: center;
}
.metric-label {
  display: block;
  font-size: 0.7rem;
  color: var(--text-secondary, #888);
  margin-bottom: 4px;
}
.metric-value {
  font-size: 1rem;
  font-weight: 700;
  color: var(--text-primary, #fff);
}

.training-complete {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 1rem;
  background: rgba(34, 197, 94, 0.08);
  border-radius: 8px;
}
.check-icon { width: 32px; height: 32px; color: #22c55e; flex-shrink: 0; }
.training-complete p { margin: 4px 0 0; font-size: 0.85rem; color: var(--text-secondary, #888); }

.training-complete.early-stopped {
  background: rgba(234, 179, 8, 0.08);
  border: 1px solid rgba(234, 179, 8, 0.2);
}
.training-complete.early-stopped .check-icon { color: #eab308; }

.early-stop-detail {
  font-size: 0.8rem;
  color: var(--text-secondary, #999);
  margin: 4px 0 8px;
  line-height: 1.4;
}

.stopping-indicator {
  font-size: 0.85rem;
  color: #f59e0b;
  padding: 8px 12px;
  background: rgba(245, 158, 11, 0.1);
  border: 1px solid rgba(245, 158, 11, 0.25);
  border-radius: 6px;
  text-align: center;
  animation: pulse 1.5s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.6; }
}

.best-epoch-indicator {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 0.8rem;
  color: #22c55e;
  padding: 6px 10px;
  background: rgba(34, 197, 94, 0.08);
  border-radius: 6px;
}
.patience-warning { color: #eab308; font-weight: 600; }

.idle-state {
  text-align: center;
  padding: 1rem;
  color: var(--text-secondary, #888);
  font-size: 0.9rem;
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
.btn:disabled { opacity: 0.5; cursor: not-allowed; }
.btn:hover:not(:disabled) { opacity: 0.85; }
.btn-primary { background: var(--accent, #7c3aed); color: white; }
.btn-danger { background: #ef4444; color: white; }

.final-metrics {
  margin-top: 1.5rem;
  padding: 1.25rem;
  background: rgba(124, 58, 237, 0.05);
  border: 1px solid rgba(124, 58, 237, 0.25);
  border-radius: 10px;
}
.final-metrics h4 {
  margin: 0 0 1rem;
  font-size: 0.95rem;
  color: var(--accent, #7c3aed);
}

.macro-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 0.75rem;
  margin-bottom: 1.25rem;
}
.macro-cell {
  background: rgba(255, 255, 255, 0.03);
  padding: 0.75rem;
  border-radius: 8px;
  text-align: center;
}
.macro-label { font-size: 0.75rem; opacity: 0.7; margin-bottom: 0.25rem; }
.macro-value { font-size: 1.25rem; font-weight: 600; }

.per-class-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.85rem;
  margin-bottom: 1rem;
}
.per-class-table th,
.per-class-table td {
  padding: 0.5rem 0.75rem;
  text-align: right;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}
.per-class-table th { font-weight: 500; opacity: 0.7; text-align: right; }
.per-class-table th:first-child,
.per-class-table td:first-child { text-align: left; }
.per-class-table .class-name { font-weight: 500; }
.per-class-table td.weak { color: #f59e0b; }

.confusion-details { margin-top: 0.5rem; }
.confusion-details summary {
  cursor: pointer;
  font-size: 0.85rem;
  opacity: 0.8;
  padding: 0.25rem 0;
}
.confusion-table {
  border-collapse: collapse;
  font-size: 0.8rem;
  margin-top: 0.75rem;
}
.confusion-table th,
.confusion-table td {
  padding: 0.35rem 0.6rem;
  text-align: center;
  border: 1px solid rgba(255, 255, 255, 0.08);
}
.confusion-table th { font-weight: 500; opacity: 0.7; background: rgba(255, 255, 255, 0.03); }
.confusion-table td.diagonal {
  background: rgba(34, 197, 94, 0.15);
  color: #4ade80;
  font-weight: 600;
}
.confusion-hint { margin: 0.5rem 0 0; font-size: 0.75rem; opacity: 0.6; }
</style>
