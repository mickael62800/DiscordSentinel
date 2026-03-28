<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { useAiTraining, type ModelType, type TrainingConfig } from "../../composables/useAiTraining";

const {
  status,
  datasets,
  loading,
  error,
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
  return Math.round((status.value.current_epoch / status.value.total_epochs) * 100);
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
            :disabled="!canStartTraining"
            @click="handleStartTraining"
          >
            Lancer l'entrainement
          </button>
          <button v-else class="btn btn-danger" @click="stopTraining">
            Arreter
          </button>
        </div>
      </div>

      <!-- Progression -->
      <div v-if="isTrainingThisModel" class="training-progress">
        <div class="progress-header">
          <span class="phase">{{ status.phase }}</span>
          <span class="epoch-count">
            Epoch {{ status.current_epoch }} / {{ status.total_epochs }}
          </span>
        </div>
        <div class="progress-bar-container">
          <div class="progress-bar" :style="{ width: progressPercent + '%' }"></div>
        </div>
        <div class="metrics-grid">
          <div class="metric">
            <span class="metric-label">Loss</span>
            <span class="metric-value">{{ status.loss.toFixed(4) }}</span>
          </div>
          <div class="metric">
            <span class="metric-label">Accuracy</span>
            <span class="metric-value">{{ (status.accuracy * 100).toFixed(1) }}%</span>
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
        v-else-if="status.phase === 'completed' && status.model_type === activeTab"
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

    <div v-if="error" class="error-banner">{{ error }}</div>
  </div>
</template>

<style scoped>
.ai-training-page {
  padding: 2rem;
  max-width: 900px;
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
