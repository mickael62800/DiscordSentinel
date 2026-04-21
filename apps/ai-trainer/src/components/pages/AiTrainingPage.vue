<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { useAiTraining, type ModelType, type TrainingConfig } from "../../composables/useAiTraining";
import TrainingDatasetSection from "../organisms/TrainingDatasetSection.vue";
import TrainingProgressPanel from "../organisms/TrainingProgressPanel.vue";
import TrainingChartsPanel from "../organisms/TrainingChartsPanel.vue";

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
  syncWithBackend,
} = useAiTraining();

const activeTab = ref<ModelType>("text-sentiment");

const epochs = ref(5);
const batchSize = ref(32);
const learningRate = ref(0.00002);
const validationSplit = ref(0.1);
const earlyStoppingPatience = ref(2);
const useClassWeights = ref(true);
const useMixedPrecision = ref(true);
const labelSmoothing = ref(0.05);
const weightDecay = ref(0.01);
const warmupRatio = ref(0.06);
const maxLength = ref(128);
const neutralCap = ref(0);

const trainingConfig = computed<TrainingConfig>(() => ({
  model_type: activeTab.value,
  epochs: epochs.value,
  batch_size: batchSize.value,
  learning_rate: learningRate.value,
  validation_split: validationSplit.value,
  early_stopping_patience: earlyStoppingPatience.value,
  use_class_weights: useClassWeights.value,
  use_mixed_precision: useMixedPrecision.value,
  label_smoothing: labelSmoothing.value,
  weight_decay: weightDecay.value,
  warmup_ratio: warmupRatio.value,
  max_length: maxLength.value,
  neutral_cap: neutralCap.value,
}));

const activeDataset = computed(() =>
  datasets.value.find((d) => d.model_type === activeTab.value)
);

const isTrainingThisModel = computed(
  () => status.value.running && status.value.model_type === activeTab.value
);

const canStartTraining = computed(
  () => !status.value.running && activeDataset.value && activeDataset.value.total_samples > 0
);

async function handleUpload() {
  const selected = await open({
    multiple: false,
    filters: [{ name: "Dataset", extensions: ["csv", "json", "jsonl", "tsv", "jpg", "jpeg", "png", "webp"] }],
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

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} o`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} Ko`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} Mo`;
}

onMounted(async () => {
  await fetchDatasets();
  await syncWithBackend();
});
</script>

<template>
  <div class="ai-training-page">
    <div class="page-header-row">
      <div>
        <h1>Sentinel AI Trainer</h1>
        <p class="subtitle">
          Entrainez des modeles de classification et exportez-les en ONNX pour l'inference Rust
        </p>
      </div>
    </div>

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

    <div class="page-layout">
      <div class="col-main">
        <section class="model-info">
          <template v-if="activeTab === 'text-sentiment'">
            <h2>CamemBERT — Sentiment / Toxicite</h2>
            <p>
              Fine-tuning d'un modele CamemBERT (138 Go de francais natif) pour classer
              les messages en 3 niveaux : neutre, toxicite legere, toxicite severe.
            </p>
            <div class="model-tags">
              <span class="tag">NLP</span>
              <span class="tag">CamemBERT</span>
              <span class="tag">Classification multi-classe</span>
              <span class="tag">ONNX</span>
            </div>
          </template>
          <template v-else>
            <h2>EfficientNetV2 — NSFW / Contenu illicite</h2>
            <p>
              Fine-tuning d'un modele EfficientNetV2 pour detecter les images NSFW et
              les contenus illicites.
            </p>
            <div class="model-tags">
              <span class="tag">Vision</span>
              <span class="tag">EfficientNetV2</span>
              <span class="tag">Classification binaire</span>
              <span class="tag">ONNX</span>
            </div>
          </template>
        </section>

        <TrainingDatasetSection
          :dataset="activeDataset"
          :loading="loading"
          :model-type="activeTab"
          :disabled="status.running"
          @upload="handleUpload"
        />

        <section class="section-card">
          <h3>Parametres d'entrainement</h3>
          <div class="params-grid">
            <div class="param">
              <label>Epochs</label>
              <input type="number" v-model.number="epochs" min="1" max="200" :disabled="status.running" />
            </div>
            <div class="param">
              <label>Batch size</label>
              <input type="number" v-model.number="batchSize" min="1" max="256" :disabled="status.running" />
            </div>
            <div class="param">
              <label>Learning rate</label>
              <input type="number" v-model.number="learningRate" min="0.00001" max="0.1" step="0.00001" :disabled="status.running" />
            </div>
            <div class="param">
              <label>Validation split</label>
              <input type="number" v-model.number="validationSplit" min="0.05" max="0.5" step="0.05" :disabled="status.running" />
            </div>
            <div class="param">
              <label>Early stopping patience</label>
              <input type="number" v-model.number="earlyStoppingPatience" min="0" max="50" :disabled="status.running" />
            </div>
            <div class="param">
              <label>Max length (tokens)</label>
              <input type="number" v-model.number="maxLength" min="16" max="512" step="16" :disabled="status.running" />
            </div>
            <div class="param">
              <label>Weight decay</label>
              <input type="number" v-model.number="weightDecay" min="0" max="0.5" step="0.005" :disabled="status.running" />
            </div>
            <div class="param">
              <label>Label smoothing</label>
              <input type="number" v-model.number="labelSmoothing" min="0" max="0.5" step="0.05" :disabled="status.running" />
            </div>
            <div class="param">
              <label>Warmup ratio</label>
              <input type="number" v-model.number="warmupRatio" min="0" max="0.5" step="0.05" :disabled="status.running" />
            </div>
            <div class="param">
              <label>Neutral cap (0 = off)</label>
              <input type="number" v-model.number="neutralCap" min="0" step="1000" :disabled="status.running" />
            </div>
          </div>

          <div class="toggle-row">
            <label class="toggle">
              <input type="checkbox" v-model="useClassWeights" :disabled="status.running" />
              <span>Class weights (inverse freq)</span>
              <small>Compense le desequilibre entre les 3 classes</small>
            </label>
            <label class="toggle">
              <input type="checkbox" v-model="useMixedPrecision" :disabled="status.running" />
              <span>Mixed precision (fp16)</span>
              <small>2x plus rapide sur GPU CUDA</small>
            </label>
          </div>
        </section>

        <TrainingProgressPanel
          :status="status"
          :stopping="stopping"
          :can-start="!!canStartTraining"
          :is-training-this="isTrainingThisModel"
          :is-training-other="status.running && !isTrainingThisModel"
          :active-tab="activeTab"
          :has-dataset="!!activeDataset"
          @start="handleStartTraining"
          @stop="stopTraining"
        />

        <section class="section-card">
          <div class="section-header">
            <h3>Export ONNX</h3>
            <button class="btn btn-primary" :disabled="exporting || status.running" @click="handleExport">
              {{ exporting ? "Export en cours..." : "Exporter en ONNX" }}
            </button>
          </div>
          <p class="description">
            Convertit le modele entraine au format ONNX pour une inference optimisee en Rust
            via <code>ort</code> (ONNX Runtime).
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

      <div class="col-charts">
        <TrainingChartsPanel :epoch-history="epochHistory" :dataset="activeDataset" />
      </div>
    </div>

    <div v-if="error" class="error-banner">{{ error }}</div>
  </div>
</template>

<style scoped>
.ai-training-page { padding: 2rem; }

.page-layout {
  display: grid;
  grid-template-columns: 1fr 380px;
  gap: 24px;
  align-items: start;
}
.col-main { min-width: 0; }
.col-charts {
  position: sticky;
  top: 2rem;
  display: flex;
  flex-direction: column;
  gap: 0;
}

.subtitle { color: var(--text-secondary, #888); margin-bottom: 1.5rem; }

.model-tabs { display: flex; gap: 8px; margin-bottom: 1.5rem; }

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
.tab:hover { color: var(--text-primary, #fff); background: var(--bg-hover, #2a2a3e); }
.tab.active {
  border-color: var(--accent, #7c3aed);
  color: var(--accent, #7c3aed);
  background: rgba(124, 58, 237, 0.08);
}
.tab-icon { width: 18px; height: 18px; }

.model-info {
  background: var(--bg-secondary, #1e1e2e);
  border-radius: 12px;
  padding: 1.5rem;
  margin-bottom: 1.5rem;
}
.model-info h2 { margin: 0 0 0.5rem; font-size: 1.1rem; }
.model-info p {
  color: var(--text-secondary, #888);
  font-size: 0.85rem;
  line-height: 1.5;
  margin-bottom: 0.75rem;
}

.model-tags { display: flex; flex-wrap: wrap; gap: 6px; }
.tag {
  padding: 3px 10px;
  background: rgba(124, 58, 237, 0.12);
  color: var(--accent, #7c3aed);
  border-radius: 6px;
  font-size: 0.75rem;
  font-weight: 600;
}

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

.params-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 16px;
}

.param { display: flex; flex-direction: column; gap: 6px; }
.param label { font-size: 0.8rem; font-weight: 600; color: var(--text-secondary, #888); }
.param input {
  padding: 8px 12px;
  background: var(--bg-primary, #161622);
  border: 1px solid var(--border, #333);
  border-radius: 8px;
  color: var(--text-primary, #fff);
  font-size: 0.9rem;
}
.param input:focus { outline: none; border-color: var(--accent, #7c3aed); }
.param input:disabled { opacity: 0.5; cursor: not-allowed; }

.toggle-row {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 12px;
  margin-top: 16px;
  padding-top: 16px;
  border-top: 1px solid var(--border, #333);
}
.toggle {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 10px 12px;
  background: var(--bg-primary, #161622);
  border: 1px solid var(--border, #333);
  border-radius: 8px;
  cursor: pointer;
  transition: border-color 0.15s;
}
.toggle:hover { border-color: var(--accent, #7c3aed); }
.toggle input[type="checkbox"] { margin-right: 6px; accent-color: var(--accent, #7c3aed); }
.toggle span { font-size: 0.85rem; font-weight: 600; color: var(--text-primary, #fff); }
.toggle small { font-size: 0.72rem; color: var(--text-secondary, #888); padding-left: 20px; }
.toggle input:disabled + span { opacity: 0.5; }

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
.export-result p { margin: 4px 0 0; font-size: 0.85rem; color: var(--text-secondary, #888); }
.check-icon { width: 32px; height: 32px; color: #22c55e; flex-shrink: 0; }
.file-size { color: var(--text-secondary, #888); margin-left: 4px; }

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

.page-header-row {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 1rem;
}

.error-banner {
  color: #ef4444;
  font-size: 0.85rem;
  padding: 0.75rem 1rem;
  background: rgba(239, 68, 68, 0.1);
  border-radius: 8px;
  margin-top: 0.5rem;
}
</style>
