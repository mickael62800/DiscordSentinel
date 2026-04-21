import { ref, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type ModelType = "text-sentiment" | "image-classification";

export interface TrainingConfig {
  model_type: ModelType;
  epochs: number;
  batch_size: number;
  learning_rate: number;
  validation_split: number;
  early_stopping_patience?: number;
  use_class_weights?: boolean;
  use_mixed_precision?: boolean;
  run_lr_finder?: boolean;
  label_smoothing?: number;
  weight_decay?: number;
  warmup_ratio?: number;
  max_length?: number;
  neutral_cap?: number;
  backbone?: string;
}

export interface PerClassMetrics {
  precision: number;
  recall: number;
  f1: number;
  support: number;
}

export interface FinalMetrics {
  accuracy: number;
  macro_precision: number;
  macro_recall: number;
  macro_f1: number;
  per_class: Record<string, PerClassMetrics>;
  confusion_matrix?: number[][];
  class_names?: string[];
}

export interface EpochRecord {
  epoch: number;
  loss: number;
  accuracy: number;
  val_loss: number;
  val_accuracy: number;
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
  final_metrics: FinalMetrics | null;
}

export interface DatasetInfo {
  model_type: ModelType;
  total_samples: number;
  label_distribution: Record<string, number>;
  last_updated: string | null;
}

export interface OnnxExportResult {
  model_type: ModelType;
  file_path: string;
  file_size_bytes: number;
}

// ── State singleton ──

function emptyStatus(): TrainingStatus {
  return {
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
    final_metrics: null,
  };
}

const status = ref<TrainingStatus>(emptyStatus());
const datasets = ref<DatasetInfo[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
const epochHistory = ref<EpochRecord[]>([]);
const stopping = ref(false);
const exporting = ref(false);
const exportResult = ref<OnnxExportResult | null>(null);
let unlisten: UnlistenFn | null = null;
let listenerAttached = false;

// ── Gestion des evenements streames depuis le process Python ──

function handleEvent(payload: Record<string, unknown>) {
  const event = payload.event as string;

  switch (event) {
    case "start": {
      status.value.running = true;
      status.value.total_epochs = (payload.total_epochs as number) ?? status.value.total_epochs;
      status.value.model_type = (payload.model_type as ModelType) ?? status.value.model_type;
      status.value.phase = "demarrage";
      break;
    }
    case "phase": {
      status.value.phase = (payload.phase as string) ?? status.value.phase;
      break;
    }
    case "batch": {
      status.value.current_epoch = (payload.epoch as number) ?? status.value.current_epoch;
      status.value.current_batch = (payload.current as number) ?? 0;
      status.value.total_batches = (payload.total as number) ?? 0;
      status.value.batch_loss = (payload.loss as number) ?? 0;
      status.value.batch_accuracy = (payload.accuracy as number) ?? 0;
      break;
    }
    case "epoch": {
      const rec: EpochRecord = {
        epoch: payload.epoch as number,
        loss: payload.loss as number,
        accuracy: payload.accuracy as number,
        val_loss: payload.val_loss as number,
        val_accuracy: payload.val_accuracy as number,
      };
      epochHistory.value = [...epochHistory.value, rec];
      status.value.epoch_history = epochHistory.value;
      status.value.current_epoch = rec.epoch;
      status.value.loss = rec.loss;
      status.value.accuracy = rec.accuracy;
      status.value.val_loss = rec.val_loss;
      status.value.val_accuracy = rec.val_accuracy;
      status.value.best_epoch = (payload.best_epoch as number) ?? status.value.best_epoch;
      // reset batch progress
      status.value.current_batch = 0;
      status.value.total_batches = 0;
      break;
    }
    case "metrics": {
      status.value.final_metrics = {
        accuracy: payload.accuracy as number,
        macro_precision: payload.macro_precision as number,
        macro_recall: payload.macro_recall as number,
        macro_f1: payload.macro_f1 as number,
        per_class: payload.per_class as Record<string, PerClassMetrics>,
        confusion_matrix: payload.confusion_matrix as number[][] | undefined,
        class_names: payload.class_names as string[] | undefined,
      };
      break;
    }
    case "done": {
      status.value.phase = (payload.phase as string) ?? "termine";
      status.value.early_stopped = Boolean(payload.early_stopped);
      status.value.best_epoch = (payload.best_epoch as number) ?? status.value.best_epoch;
      status.value.running = false;
      stopping.value = false;
      break;
    }
    case "process_exited": {
      status.value.running = false;
      stopping.value = false;
      break;
    }
    case "error": {
      error.value = (payload.message as string) ?? "erreur inconnue";
      status.value.phase = `erreur: ${error.value}`;
      status.value.running = false;
      stopping.value = false;
      break;
    }
  }
}

async function attachListener() {
  if (listenerAttached) return;
  unlisten = await listen<Record<string, unknown>>("training://event", (evt) => {
    handleEvent(evt.payload);
  });
  listenerAttached = true;
}

// ── Fonctions exposees ──

async function fetchDatasets() {
  loading.value = true;
  error.value = null;
  try {
    const result = await invoke<DatasetInfo[]>("ai_get_datasets");
    datasets.value = Array.isArray(result) ? result : [];
  } catch (e) {
    error.value = String(e);
    datasets.value = [];
  } finally {
    loading.value = false;
  }
}

async function uploadDataset(modelType: ModelType, filePath: string) {
  error.value = null;
  try {
    await invoke("ai_upload_dataset", { modelType, filePath });
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
    ...emptyStatus(),
    running: true,
    model_type: config.model_type,
    total_epochs: config.epochs,
    phase: "demarrage",
  };

  await attachListener();

  try {
    await invoke("ai_start_training", {
      modelType: config.model_type,
      epochs: config.epochs,
      batchSize: config.batch_size,
      learningRate: config.learning_rate,
      validationSplit: config.validation_split,
      earlyStoppingPatience: config.early_stopping_patience,
      useClassWeights: config.use_class_weights,
      useMixedPrecision: config.use_mixed_precision,
      labelSmoothing: config.label_smoothing,
      weightDecay: config.weight_decay,
      warmupRatio: config.warmup_ratio,
      maxLength: config.max_length,
      neutralCap: config.neutral_cap,
      backbone: config.backbone,
    });
  } catch (e) {
    error.value = String(e);
    status.value.running = false;
    status.value.phase = "idle";
  }
}

async function stopTraining() {
  stopping.value = true;
  try {
    await invoke("ai_stop_training");
    status.value.phase = "arret en cours...";
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
    exportResult.value = await invoke<OnnxExportResult>("ai_export_onnx", { modelType });
  } catch (e) {
    error.value = String(e);
  } finally {
    exporting.value = false;
  }
}

async function syncWithBackend() {
  await attachListener();
  try {
    const running = await invoke<boolean>("ai_is_training");
    if (!running) {
      status.value.running = false;
    }
  } catch {
    // ignorer
  }
}

function cleanup() {
  if (unlisten) {
    unlisten();
    unlisten = null;
    listenerAttached = false;
  }
}

// ── Export singleton ──

export function useAiTraining() {
  onUnmounted(() => {
    // on garde le listener actif au niveau module (singleton),
    // mais on offre cleanup explicite si besoin
  });

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
    exportOnnx,
    syncWithBackend,
    cleanup,
  };
}
