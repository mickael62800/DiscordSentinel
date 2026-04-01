import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export type ModelType = "text-sentiment" | "image-classification";

export interface TrainingConfig {
  model_type: ModelType;
  epochs: number;
  batch_size: number;
  learning_rate: number;
  validation_split: number;
}

export interface TrainingStatus {
  running: boolean;
  model_type: ModelType | null;
  current_epoch: number;
  total_epochs: number;
  loss: number;
  accuracy: number;
  val_loss: number;
  val_accuracy: number;
  phase: string;
  epoch_history: EpochRecord[];
  current_batch: number;
  total_batches: number;
  batch_loss: number;
  batch_accuracy: number;
  early_stopped: boolean;
  best_epoch: number;
}

export interface DatasetInfo {
  model_type: ModelType;
  total_samples: number;
  label_distribution: Record<string, number>;
  last_updated: string | null;
}

export interface EpochRecord {
  epoch: number;
  loss: number;
  accuracy: number;
  val_loss: number;
  val_accuracy: number;
}

export interface OnnxExportResult {
  model_type: ModelType;
  file_path: string;
  file_size_bytes: number;
}

// ── State global (singleton) — persiste entre les navigations de page ──

const status = ref<TrainingStatus>({
  running: false,
  model_type: null,
  current_epoch: 0,
  total_epochs: 0,
  loss: 0,
  accuracy: 0,
  val_loss: 0,
  val_accuracy: 0,
  phase: "idle",
  epoch_history: [],
  current_batch: 0,
  total_batches: 0,
  batch_loss: 0,
  batch_accuracy: 0,
  early_stopped: false,
  best_epoch: 0,
});
const datasets = ref<DatasetInfo[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
const epochHistory = ref<EpochRecord[]>([]);
const stopping = ref(false);
const exporting = ref(false);
const exportResult = ref<OnnxExportResult | null>(null);
let pollTimer: ReturnType<typeof setInterval> | null = null;
let pollCount = 0;
let hasSeenRunning = false;

// ── Fonctions ──

async function fetchDatasets() {
  loading.value = true;
  error.value = null;
  try {
    datasets.value = await invoke<DatasetInfo[]>("ai_get_datasets");
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function uploadDataset(modelType: ModelType, filePath: string) {
  error.value = null;
  try {
    await invoke("ai_upload_dataset", { model_type: modelType, file_path: filePath });
    await fetchDatasets();
  } catch (e) {
    error.value = String(e);
  }
}

async function startTraining(config: TrainingConfig) {
  error.value = null;
  exportResult.value = null;
  epochHistory.value = [];
  status.value = {
    running: true,
    model_type: config.model_type,
    current_epoch: 0,
    total_epochs: config.epochs,
    loss: 0,
    accuracy: 0,
    val_loss: 0,
    val_accuracy: 0,
    phase: "demarrage",
    epoch_history: [],
    current_batch: 0,
    total_batches: 0,
    batch_loss: 0,
    batch_accuracy: 0,
    early_stopped: false,
    best_epoch: 0,
  };
  try {
    await invoke("ai_start_training", {
      model_type: config.model_type,
      epochs: config.epochs,
      batch_size: config.batch_size,
      learning_rate: config.learning_rate,
      validation_split: config.validation_split,
    });
    startPolling();
  } catch (e) {
    error.value = String(e);
    status.value.running = false;
    status.value.phase = "idle";
  }
}

async function pollStatus() {
  try {
    pollCount++;
    const result = await invoke<TrainingStatus>("ai_training_status");

    if (!result.running && !hasSeenRunning && pollCount <= 3) {
      return;
    }

    status.value = result;

    if (result.epoch_history && result.epoch_history.length > 0) {
      epochHistory.value = result.epoch_history;
    }

    if (result.running) {
      hasSeenRunning = true;
    }

    if (!result.running && pollTimer && (hasSeenRunning || pollCount > 20)) {
      stopPolling();
      stopping.value = false;
    }
  } catch {
    // silently ignore poll errors
  }
}

function startPolling() {
  stopPolling();
  pollCount = 0;
  hasSeenRunning = false;
  pollTimer = setInterval(pollStatus, 1500);
}

function stopPolling() {
  if (pollTimer) {
    clearInterval(pollTimer);
    pollTimer = null;
  }
}

async function stopTraining() {
  stopping.value = true;
  try {
    await invoke("ai_stop_training");
    status.value.phase = "arret en cours...";
    if (!pollTimer) {
      startPolling();
    }
  } catch (e) {
    error.value = String(e);
    stopping.value = false;
  }
}

async function exportOnnx(modelType: ModelType) {
  exporting.value = true;
  error.value = null;
  exportResult.value = null;
  try {
    exportResult.value = await invoke<OnnxExportResult>("ai_export_onnx", { model_type: modelType });
  } catch (e) {
    error.value = String(e);
  } finally {
    exporting.value = false;
  }
}

/// Synchronise le state avec le backend.
/// Appele quand on arrive sur la page pour recuperer l'etat reel.
async function syncWithBackend() {
  try {
    const result = await invoke<TrainingStatus>("ai_training_status");
    status.value = result;
    if (result.epoch_history && result.epoch_history.length > 0) {
      epochHistory.value = result.epoch_history;
    }
    // Si un training est en cours cote backend, relancer le polling
    if (result.running && !pollTimer) {
      hasSeenRunning = true;
      startPolling();
    }
  } catch {
    // API ML pas disponible
  }
}

// ── Export singleton ──

export function useAiTraining() {
  return {
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
    stopPolling,
    exportOnnx,
    pollStatus,
    syncWithBackend,
  };
}
