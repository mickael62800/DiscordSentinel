<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { useAiTraining, type ModelType, type TrainingConfig } from "../../composables/useAiTraining";
import { Line, Doughnut } from "vue-chartjs";
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  ArcElement,
  Title,
  Tooltip,
  Legend,
  Filler,
} from "chart.js";

ChartJS.register(
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  ArcElement,
  Title,
  Tooltip,
  Legend,
  Filler,
);

const {
  status,
  datasets,
  epochHistory,
  loading,
  error,
  stopping,
  exporting,
  exportResult,
  fetchDatasets,
  uploadDataset,
  startTraining,
  stopTraining,
  exportOnnx,
} = useAiTraining();

// ── Onglet actif ──
const activeTab = ref<ModelType>("text-sentiment");

// ── Config entrainement ──
const epochs = ref(10);
const batchSize = ref(32);
const learningRate = ref(0.001);
const validationSplit = ref(0.2);

const trainingConfig = computed<TrainingConfig>(() => ({
  model_type: activeTab.value,
  epochs: epochs.value,
  batch_size: batchSize.value,
  learning_rate: learningRate.value,
  validation_split: validationSplit.value,
}));

const activeDataset = computed(() =>
  datasets.value.find((d) => d.model_type === activeTab.value)
);

const progressPercent = computed(() => {
  if (!status.value.total_epochs) return 0;
  const epochProgress = status.value.total_batches
    ? status.value.current_batch / status.value.total_batches
    : 0;
  const completedEpochs = Math.max(status.value.current_epoch - 1, 0);
  return Math.round(((completedEpochs + epochProgress) / status.value.total_epochs) * 100);
});

const batchPercent = computed(() => {
  if (!status.value.total_batches) return 0;
  return Math.round((status.value.current_batch / status.value.total_batches) * 100);
});

const isTrainingThisModel = computed(
  () => status.value.running && status.value.model_type === activeTab.value
);

const canStartTraining = computed(
  () => !status.value.running && activeDataset.value && activeDataset.value.total_samples > 0
);

async function handleUpload() {
  const selected = await open({
    multiple: false,
    filters: [{ name: "Dataset", extensions: ["csv", "json", "jsonl", "tsv"] }],
  });
  if (selected) {
    await uploadDataset(activeTab.value, selected as string);
  }
}

async function handleStartTraining() {
  await startTraining(trainingConfig.value);
}

async function handleExport() {
  await exportOnnx(activeTab.value);
}

// ── Options graphiques ──
const lineChartOptions = {
  responsive: true,
  maintainAspectRatio: false,
  animation: { duration: 300 },
  plugins: {
    legend: { labels: { color: "#9495b0", font: { size: 11 } } },
  },
  scales: {
    x: {
      ticks: { color: "#9495b0", font: { size: 10 } },
      grid: { color: "rgba(58, 59, 92, 0.5)" },
    },
    y: {
      ticks: { color: "#9495b0", font: { size: 10 } },
      grid: { color: "rgba(58, 59, 92, 0.5)" },
      beginAtZero: true,
    },
  },
};

const accuracyChartOptions = {
  ...lineChartOptions,
  scales: {
    ...lineChartOptions.scales,
    y: {
      ...lineChartOptions.scales.y,
      max: 1,
      ticks: {
        ...lineChartOptions.scales.y.ticks,
        callback: (v: number | string) => `${(Number(v) * 100).toFixed(0)}%`,
      },
    },
  },
};

const doughnutOptions = {
  responsive: true,
  maintainAspectRatio: false,
  plugins: {
    legend: {
      position: "bottom" as const,
      labels: { color: "#9495b0", font: { size: 11 }, padding: 16 },
    },
  },
};

const epochLabels = computed(() =>
  epochHistory.value.map((e) => `Epoch ${e.epoch}`)
);

const lossChartData = computed(() => ({
  labels: epochLabels.value,
  datasets: [
    {
      label: "Loss (train)",
      data: epochHistory.value.map((e) => e.loss),
      borderColor: "#ef4444",
      backgroundColor: "rgba(239, 68, 68, 0.1)",
      fill: true,
      tension: 0.3,
      pointRadius: 3,
    },
    {
      label: "Loss (validation)",
      data: epochHistory.value.map((e) => e.val_loss),
      borderColor: "#f97316",
      backgroundColor: "rgba(249, 115, 22, 0.1)",
      fill: true,
      tension: 0.3,
      pointRadius: 3,
      borderDash: [5, 5],
    },
  ],
}));

const accuracyChartData = computed(() => ({
  labels: epochLabels.value,
  datasets: [
    {
      label: "Accuracy (train)",
      data: epochHistory.value.map((e) => e.accuracy),
      borderColor: "#22c55e",
      backgroundColor: "rgba(34, 197, 94, 0.1)",
      fill: true,
      tension: 0.3,
      pointRadius: 3,
    },
    {
      label: "Accuracy (validation)",
      data: epochHistory.value.map((e) => e.val_accuracy),
      borderColor: "#3b82f6",
      backgroundColor: "rgba(59, 130, 246, 0.1)",
      fill: true,
      tension: 0.3,
      pointRadius: 3,
      borderDash: [5, 5],
    },
  ],
}));

const LABEL_COLORS = [
  "#7c3aed", "#5865f2", "#22c55e", "#f97316", "#ef4444",
  "#06b6d4", "#ec4899", "#eab308", "#8b5cf6", "#14b8a6",
];

const datasetChartData = computed(() => {
  const ds = activeDataset.value;
  if (!ds) return { labels: [], datasets: [] };
  const labels = Object.keys(ds.label_distribution);
  const data = Object.values(ds.label_distribution);
  return {
    labels,
    datasets: [
      {
        data,
        backgroundColor: labels.map((_, i) => LABEL_COLORS[i % LABEL_COLORS.length]),
        borderWidth: 0,
      },
    ],
  };
});

const hasEpochData = computed(() => epochHistory.value.length > 0);

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} o`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} Ko`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} Mo`;
}

onMounted(fetchDatasets);
</script>

<template>
  <div class="ai-training-page">
    <h1>Entrainement IA</h1>
    <p class="subtitle">
      Entrainez des modeles de classification et exportez-les en ONNX pour l'inference Rust
    </p>

    <!-- Onglets modele -->
    <div class="model-tabs">
      <button
        :class="['tab', { active: activeTab === 'text-sentiment' }]"
        @click="activeTab = 'text-sentiment'"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="tab-icon">
          <path d="M21 15a2 2 0 01-2 2H7l-4 4V5a2 2 0 012-2h14a2 2 0 012 2z" />
        </svg>
        Analyse de texte
      </button>
      <button
        :class="['tab', { active: activeTab === 'image-classification' }]"
        @click="activeTab = 'image-classification'"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="tab-icon">
          <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
          <circle cx="8.5" cy="8.5" r="1.5" />
          <polyline points="21 15 16 10 5 21" />
        </svg>
        Classification d'images
      </button>
    </div>

    <!-- Layout 2 colonnes -->
    <div class="page-layout">
      <!-- Colonne gauche : controles -->
      <div class="col-main">
        <!-- Description du modele -->
        <section class="model-info">
          <template v-if="activeTab === 'text-sentiment'">
            <h2>DistilBERT — Sentiment / Toxicite</h2>
            <p>
              Fine-tuning d'un modele DistilBERT pour detecter la colere, les menaces,
              le harcelement et le spam dans les messages Discord. Utilise par
              <strong>automod-bot</strong> pour la moderation automatique du texte.
            </p>
            <div class="model-tags">
              <span class="tag">NLP</span>
              <span class="tag">DistilBERT</span>
              <span class="tag">Classification multi-label</span>
              <span class="tag">ONNX</span>
            </div>
          </template>
          <template v-else>
            <h2>EfficientNetV2 — NSFW / Contenu illicite</h2>
            <p>
              Fine-tuning d'un modele EfficientNetV2 pour detecter les images NSFW et
              les contenus illicites postes sur Discord. Utilise par
              <strong>image-bot</strong> pour la moderation automatique des images.
            </p>
            <div class="model-tags">
              <span class="tag">Vision</span>
              <span class="tag">EfficientNetV2</span>
              <span class="tag">Classification binaire</span>
              <span class="tag">ONNX</span>
            </div>
          </template>
        </section>

        <!-- Dataset -->
        <section class="section-card">
          <div class="section-header">
            <h3>Dataset</h3>
            <button class="btn btn-secondary" @click="handleUpload" :disabled="status.running">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="btn-icon">
                <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4" />
                <polyline points="17 8 12 3 7 8" />
                <line x1="12" y1="3" x2="12" y2="15" />
              </svg>
              Importer un dataset
            </button>
          </div>

          <div v-if="loading" class="loading-text">Chargement...</div>

          <div v-else-if="!activeDataset" class="empty-state">
            <p>Aucun dataset importe pour ce modele.</p>
            <p class="hint">
              <template v-if="activeTab === 'text-sentiment'">
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
              <span class="stat-value">{{ activeDataset.total_samples.toLocaleString() }}</span>
              <span class="stat-label">Echantillons</span>
            </div>
            <div
              v-for="(count, label) in activeDataset.label_distribution"
              :key="label"
              class="stat-card"
            >
              <span class="stat-value">{{ count.toLocaleString() }}</span>
              <span class="stat-label">{{ label }}</span>
            </div>
            <div v-if="activeDataset.last_updated" class="stat-card">
              <span class="stat-value stat-date">{{ activeDataset.last_updated }}</span>
              <span class="stat-label">Derniere mise a jour</span>
            </div>
          </div>
        </section>

        <!-- Parametres d'entrainement -->
        <section class="section-card">
          <h3>Parametres d'entrainement</h3>
          <div class="params-grid">
            <div class="param">
              <label>Epochs</label>
              <input type="number" v-model.number="epochs" min="1" max="100" :disabled="status.running" />
            </div>
            <div class="param">
              <label>Batch size</label>
              <input type="number" v-model.number="batchSize" min="1" max="256" :disabled="status.running" />
            </div>
            <div class="param">
              <label>Learning rate</label>
              <input
                type="number"
                v-model.number="learningRate"
                min="0.00001"
                max="0.1"
                step="0.0001"
                :disabled="status.running"
              />
            </div>
            <div class="param">
              <label>Validation split</label>
              <input
                type="number"
                v-model.number="validationSplit"
                min="0.05"
                max="0.5"
                step="0.05"
                :disabled="status.running"
              />
            </div>
          </div>
        </section>

        <!-- Entrainement -->
        <section class="section-card">
          <div class="section-header">
            <h3>Entrainement</h3>
            <div class="actions">
              <button
                v-if="!status.running"
                class="btn btn-primary"
                :disabled="!canStartTraining || stopping"
                @click="handleStartTraining"
              >
                {{ stopping ? "Arret en cours..." : "Lancer l'entrainement" }}
              </button>
              <button
                v-else
                class="btn btn-danger"
                :disabled="stopping"
                @click="stopTraining"
              >
                {{ stopping ? "Arret en cours..." : "Arreter" }}
              </button>
            </div>
          </div>

          <!-- Progression -->
          <div v-if="isTrainingThisModel" class="training-progress">
            <!-- Progression globale -->
            <div class="progress-header">
              <span class="phase">{{ status.phase }}</span>
              <span class="epoch-count">
                Epoch {{ status.current_epoch }} / {{ status.total_epochs }}
              </span>
            </div>
            <div class="progress-bar-container">
              <div class="progress-bar" :style="{ width: progressPercent + '%' }"></div>
            </div>

            <!-- Progression batch intra-epoch -->
            <div v-if="status.total_batches > 0" class="batch-progress">
              <div class="batch-header">
                <span class="batch-label">Batch {{ status.current_batch }} / {{ status.total_batches }}</span>
                <span class="batch-percent">{{ batchPercent }}%</span>
              </div>
              <div class="progress-bar-container progress-bar-sm">
                <div class="progress-bar progress-bar-batch" :style="{ width: batchPercent + '%' }"></div>
              </div>
            </div>

            <!-- Metriques live (batch en cours) -->
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
          </div>

          <div
            v-else-if="status.phase === 'termine' && status.model_type === activeTab"
            class="training-complete"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="check-icon">
              <path d="M22 11.08V12a10 10 0 11-5.93-9.14" />
              <polyline points="22 4 12 14.01 9 11.01" />
            </svg>
            <div>
              <strong>Entrainement termine</strong>
              <p>
                Accuracy finale : {{ (status.val_accuracy * 100).toFixed(1) }}% —
                Loss : {{ status.val_loss.toFixed(4) }}
              </p>
            </div>
          </div>

          <div v-else class="idle-state">
            <p v-if="!canStartTraining && !activeDataset">
              Importez un dataset pour commencer l'entrainement.
            </p>
            <p v-else-if="status.running">
              Un entrainement est deja en cours sur un autre modele.
            </p>
            <p v-else>Pret a lancer l'entrainement.</p>
          </div>
        </section>

        <!-- Export ONNX -->
        <section class="section-card">
          <div class="section-header">
            <h3>Export ONNX</h3>
            <button
              class="btn btn-primary"
              :disabled="exporting || status.running"
              @click="handleExport"
            >
              {{ exporting ? "Export en cours..." : "Exporter en ONNX" }}
            </button>
          </div>
          <p class="description">
            Convertit le modele entraine au format ONNX pour une inference optimisee en Rust
            via <code>ort</code> (ONNX Runtime). Le fichier sera place dans le repertoire
            de modeles de l'API.
          </p>

          <div v-if="exportResult" class="export-result">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="check-icon">
              <path d="M22 11.08V12a10 10 0 11-5.93-9.14" />
              <polyline points="22 4 12 14.01 9 11.01" />
            </svg>
            <div>
              <strong>Export reussi</strong>
              <p class="export-details">
                {{ exportResult.file_path }}
                <span class="file-size">({{ formatSize(exportResult.file_size_bytes) }})</span>
              </p>
            </div>
          </div>
        </section>
      </div>

      <!-- Colonne droite : graphiques (sticky) -->
      <div class="col-charts">
        <!-- Distribution du dataset -->
        <section v-if="activeDataset && Object.keys(activeDataset.label_distribution).length > 0" class="section-card">
          <h3>Distribution des labels</h3>
          <div class="chart-center">
            <div class="doughnut-wrapper">
              <Doughnut :data="datasetChartData" :options="doughnutOptions" />
            </div>
          </div>
        </section>

        <!-- Courbe Loss -->
        <section v-if="hasEpochData" class="section-card">
          <h3>Loss</h3>
          <div class="chart-wrapper">
            <Line :data="lossChartData" :options="lineChartOptions" />
          </div>
        </section>

        <!-- Courbe Accuracy -->
        <section v-if="hasEpochData" class="section-card">
          <h3>Accuracy</h3>
          <div class="chart-wrapper">
            <Line :data="accuracyChartData" :options="accuracyChartOptions" />
          </div>
        </section>

        <!-- Etat vide quand pas de graphiques -->
        <section v-if="!hasEpochData && !(activeDataset && Object.keys(activeDataset.label_distribution).length > 0)" class="section-card charts-empty">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="empty-icon">
            <polyline points="22 12 18 12 15 21 9 3 6 12 2 12" />
          </svg>
          <p>Les graphiques apparaitront ici une fois un dataset charge ou un entrainement lance.</p>
        </section>
      </div>
    </div>

    <div v-if="error" class="error-banner">{{ error }}</div>
  </div>
</template>

<style scoped>
.ai-training-page {
  padding: 2rem;
}

.page-layout {
  display: grid;
  grid-template-columns: 1fr 380px;
  gap: 24px;
  align-items: start;
}

.col-main {
  min-width: 0;
}

.col-charts {
  position: sticky;
  top: 2rem;
  display: flex;
  flex-direction: column;
  gap: 0;
}

.subtitle {
  color: var(--text-secondary, #888);
  margin-bottom: 1.5rem;
}

/* Onglets */
.model-tabs {
  display: flex;
  gap: 8px;
  margin-bottom: 1.5rem;
}

.tab {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 20px;
  background: var(--bg-secondary, #1e1e2e);
  border: 2px solid transparent;
  border-radius: 10px;
  color: var(--text-secondary, #888);
  font-weight: 600;
  font-size: 0.9rem;
  cursor: pointer;
  transition: all 0.15s;
}

.tab:hover {
  color: var(--text-primary, #fff);
  background: var(--bg-hover, #2a2a3e);
}

.tab.active {
  border-color: var(--accent, #7c3aed);
  color: var(--accent, #7c3aed);
  background: rgba(124, 58, 237, 0.08);
}

.tab-icon {
  width: 18px;
  height: 18px;
}

/* Info modele */
.model-info {
  background: var(--bg-secondary, #1e1e2e);
  border-radius: 12px;
  padding: 1.5rem;
  margin-bottom: 1.5rem;
}

.model-info h2 {
  margin: 0 0 0.5rem;
  font-size: 1.1rem;
}

.model-info p {
  color: var(--text-secondary, #888);
  font-size: 0.85rem;
  line-height: 1.5;
  margin-bottom: 0.75rem;
}

.model-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.tag {
  padding: 3px 10px;
  background: rgba(124, 58, 237, 0.12);
  color: var(--accent, #7c3aed);
  border-radius: 6px;
  font-size: 0.75rem;
  font-weight: 600;
}

/* Sections */
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

/* Dataset stats */
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

/* Parametres */
.params-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 16px;
}

.param {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.param label {
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--text-secondary, #888);
}

.param input {
  padding: 8px 12px;
  background: var(--bg-primary, #161622);
  border: 1px solid var(--border, #333);
  border-radius: 8px;
  color: var(--text-primary, #fff);
  font-size: 0.9rem;
}

.param input:focus {
  outline: none;
  border-color: var(--accent, #7c3aed);
}

.param input:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* Progression */
.training-progress {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.progress-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.phase {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--accent, #7c3aed);
  text-transform: capitalize;
}

.epoch-count {
  font-size: 0.85rem;
  color: var(--text-secondary, #888);
}

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

.progress-bar-sm {
  height: 6px;
}

.progress-bar-batch {
  background: linear-gradient(90deg, #3b82f6, #60a5fa);
}

.batch-progress {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.batch-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.batch-label {
  font-size: 0.8rem;
  color: var(--text-secondary, #888);
}

.batch-percent {
  font-size: 0.8rem;
  font-weight: 600;
  color: #3b82f6;
}

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

/* Complete */
.training-complete {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 1rem;
  background: rgba(34, 197, 94, 0.08);
  border-radius: 8px;
}

.check-icon {
  width: 32px;
  height: 32px;
  color: #22c55e;
  flex-shrink: 0;
}

.training-complete p {
  margin: 4px 0 0;
  font-size: 0.85rem;
  color: var(--text-secondary, #888);
}

.idle-state {
  text-align: center;
  padding: 1rem;
  color: var(--text-secondary, #888);
  font-size: 0.9rem;
}

/* Export */
.description {
  color: var(--text-secondary, #888);
  font-size: 0.85rem;
  line-height: 1.5;
  margin-bottom: 1rem;
}

.description code {
  background: var(--bg-primary, #161622);
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 0.8rem;
}

.export-result {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 1rem;
  background: rgba(34, 197, 94, 0.08);
  border-radius: 8px;
  margin-top: 1rem;
}

.export-result p {
  margin: 4px 0 0;
  font-size: 0.85rem;
  color: var(--text-secondary, #888);
}

.file-size {
  color: var(--text-secondary, #888);
  margin-left: 4px;
}

/* Graphiques colonne droite */
.chart-wrapper {
  position: relative;
  height: 200px;
}

.chart-center {
  display: flex;
  justify-content: center;
}

.doughnut-wrapper {
  position: relative;
  width: 100%;
  height: 260px;
}

.charts-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 3rem 1.5rem;
  text-align: center;
  color: var(--text-secondary, #888);
}

.charts-empty p {
  font-size: 0.85rem;
  line-height: 1.5;
}

.empty-icon {
  width: 40px;
  height: 40px;
  opacity: 0.3;
}

/* Boutons */
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

.btn-primary {
  background: var(--accent, #7c3aed);
  color: white;
}

.btn-secondary {
  background: var(--bg-hover, #2a2a3e);
  color: var(--text-primary, #fff);
}

.btn-danger {
  background: #ef4444;
  color: white;
}

.btn-icon {
  width: 16px;
  height: 16px;
}

.actions {
  display: flex;
  gap: 8px;
}

/* Erreur */
.error-banner {
  color: #ef4444;
  font-size: 0.85rem;
  padding: 0.75rem 1rem;
  background: rgba(239, 68, 68, 0.1);
  border-radius: 8px;
  margin-top: 0.5rem;
}
</style>
