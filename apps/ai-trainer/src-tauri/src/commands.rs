//! Commandes Tauri — spawn les scripts Python et relaient stdout vers le frontend via events.
//!
//! Protocole: chaque ligne stdout du script est une JSON object {"event": "...", ...}.
//! Cette commande forward chaque ligne vers le frontend via l'event global "training://event".

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

use crate::TrainingState;

// ──────────────────────────────────────────────────────────────
// Resolution des chemins python/ et data/
// ──────────────────────────────────────────────────────────────

/// Resout le repertoire contenant les scripts Python.
/// - Dev: `<CARGO_MANIFEST_DIR>/../python`
/// - Prod (bundled): `resource_dir/python`
fn resolve_python_dir(app: &AppHandle) -> Result<PathBuf, String> {
    // Tente d'abord la ressource bundled
    if let Ok(resource_dir) = app.path().resource_dir() {
        let candidate = resource_dir.join("python");
        if candidate.join("train.py").exists() {
            return Ok(candidate);
        }
    }
    // Fallback dev: racine projet relative au manifest
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidate = manifest.parent().ok_or("chemin manifest invalide")?.join("python");
    if candidate.join("train.py").exists() {
        return Ok(candidate);
    }
    Err(format!("scripts Python introuvables (essaye: {})", candidate.display()))
}

/// Resout le repertoire data/ qui contient datasets, checkpoints, exports.
/// En dev: `<manifest>/../data`. Cree si absent.
fn resolve_data_dir(_app: &AppHandle) -> Result<PathBuf, String> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let data = manifest.parent().ok_or("chemin manifest invalide")?.join("data");
    std::fs::create_dir_all(&data).map_err(|e| format!("creation data/: {e}"))?;
    Ok(data)
}

fn python_executable() -> String {
    std::env::var("AI_TRAINER_PYTHON").unwrap_or_else(|_| "python".to_string())
}

// ──────────────────────────────────────────────────────────────
// Commande: datasets
// ──────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn ai_get_datasets(app: AppHandle) -> Result<serde_json::Value, String> {
    let python_dir = resolve_python_dir(&app)?;
    let data_dir = resolve_data_dir(&app)?;

    let output = Command::new(python_executable())
        .arg(python_dir.join("dataset_stats.py"))
        .arg("--data-root")
        .arg(&data_dir)
        .output()
        .await
        .map_err(|e| format!("echec execution python: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "dataset_stats.py a echoue: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(stdout.trim()).map_err(|e| format!("parse stats: {e}"))
}

// ──────────────────────────────────────────────────────────────
// Commande: upload dataset (copie locale)
// ──────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn ai_upload_dataset(
    app: AppHandle,
    model_type: String,
    file_path: String,
) -> Result<serde_json::Value, String> {
    let data_dir = resolve_data_dir(&app)?;
    let source = PathBuf::from(&file_path);
    if !source.exists() {
        return Err("fichier source introuvable".into());
    }
    let file_name = source
        .file_name()
        .ok_or("nom de fichier invalide")?
        .to_string_lossy()
        .to_string();

    let target_dir = match model_type.as_str() {
        "text-sentiment" => data_dir.join("text").join("datasets").join("toxic"),
        "image-classification" => data_dir.join("vision").join("datasets"),
        other => return Err(format!("model_type inconnu: {other}")),
    };
    std::fs::create_dir_all(&target_dir).map_err(|e| format!("creation dossier: {e}"))?;

    let target = target_dir.join(&file_name);
    std::fs::copy(&source, &target).map_err(|e| format!("copie fichier: {e}"))?;
    let size = std::fs::metadata(&target).map(|m| m.len()).unwrap_or(0);

    Ok(serde_json::json!({ "uploaded": file_name, "size": size }))
}

// ──────────────────────────────────────────────────────────────
// Commande: start training
// ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct StartTrainingArgs {
    pub modelType: String,
    pub epochs: u32,
    pub batchSize: u32,
    pub learningRate: f64,
    pub validationSplit: f64,
    pub earlyStoppingPatience: Option<u32>,
    pub useClassWeights: Option<bool>,
    pub useMixedPrecision: Option<bool>,
    pub labelSmoothing: Option<f64>,
    pub weightDecay: Option<f64>,
    pub warmupRatio: Option<f64>,
    pub maxLength: Option<u32>,
    pub neutralCap: Option<u32>,
    pub backbone: Option<String>,
}

#[tauri::command]
pub async fn ai_start_training(
    app: AppHandle,
    state: State<'_, Arc<TrainingState>>,
    model_type: String,
    epochs: u32,
    batch_size: u32,
    learning_rate: f64,
    validation_split: f64,
    early_stopping_patience: Option<u32>,
    use_class_weights: Option<bool>,
    use_mixed_precision: Option<bool>,
    label_smoothing: Option<f64>,
    weight_decay: Option<f64>,
    warmup_ratio: Option<f64>,
    max_length: Option<u32>,
    neutral_cap: Option<u32>,
) -> Result<(), String> {
    // Refuse si deja en cours
    {
        let guard = state.child.lock().await;
        if guard.is_some() {
            return Err("un entrainement est deja en cours".into());
        }
    }

    let python_dir = resolve_python_dir(&app)?;
    let data_dir = resolve_data_dir(&app)?;

    // Flag de stop dans le temp dir
    let stop_flag = std::env::temp_dir().join(format!("sentinel-trainer-stop-{}.flag", std::process::id()));
    let _ = std::fs::remove_file(&stop_flag);

    let mut cmd = Command::new(python_executable());
    cmd.arg(python_dir.join("train.py"))
        .arg("--model-type").arg(&model_type)
        .arg("--data-root").arg(&data_dir)
        .arg("--epochs").arg(epochs.to_string())
        .arg("--batch-size").arg(batch_size.to_string())
        .arg("--learning-rate").arg(learning_rate.to_string())
        .arg("--validation-split").arg(validation_split.to_string())
        .arg("--stop-flag").arg(&stop_flag)
        .arg("--use-class-weights").arg(use_class_weights.unwrap_or(true).to_string())
        .arg("--use-mixed-precision").arg(use_mixed_precision.unwrap_or(true).to_string())
        .arg("--weight-decay").arg(weight_decay.unwrap_or(0.01).to_string())
        .arg("--warmup-ratio").arg(warmup_ratio.unwrap_or(0.1).to_string());

    if let Some(p) = early_stopping_patience {
        cmd.arg("--early-stopping-patience").arg(p.to_string());
    }
    if let Some(v) = label_smoothing {
        cmd.arg("--label-smoothing").arg(v.to_string());
    }
    if let Some(v) = max_length {
        cmd.arg("--max-length").arg(v.to_string());
    }
    if let Some(v) = neutral_cap {
        cmd.arg("--neutral-cap").arg(v.to_string());
    }

    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

    let mut child = cmd.spawn().map_err(|e| format!("spawn python: {e}"))?;

    // Stream stdout -> events "training://event"
    if let Some(stdout) = child.stdout.take() {
        let app_clone = app.clone();
        tauri::async_runtime::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                match serde_json::from_str::<serde_json::Value>(trimmed) {
                    Ok(value) => {
                        let _ = app_clone.emit("training://event", value);
                    }
                    Err(_) => {
                        // Ligne non-JSON (log): ignorer ou emit raw
                        let _ = app_clone.emit(
                            "training://log",
                            serde_json::json!({ "line": trimmed }),
                        );
                    }
                }
            }
        });
    }

    // Stream stderr pour les logs/erreurs Python (utile au debug)
    if let Some(stderr) = child.stderr.take() {
        let app_clone = app.clone();
        tauri::async_runtime::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = app_clone.emit(
                    "training://log",
                    serde_json::json!({ "line": line, "stream": "stderr" }),
                );
            }
        });
    }

    // Stocke l'enfant et le flag de stop
    {
        let mut guard_child = state.child.lock().await;
        let mut guard_flag = state.stop_flag_path.lock().await;
        *guard_child = Some(child);
        *guard_flag = Some(stop_flag.clone());
    }

    // Attend la fin en tache detachee pour nettoyer le state quand termine
    let state_clone: Arc<TrainingState> = state.inner().clone();
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut guard_child = state_clone.child.lock().await;
        if let Some(mut child) = guard_child.take() {
            let _ = child.wait().await;
            // nettoyer le stop flag
            let mut guard_flag = state_clone.stop_flag_path.lock().await;
            if let Some(ref path) = *guard_flag {
                let _ = std::fs::remove_file(path);
            }
            *guard_flag = None;
            // prevenir le front
            let _ = app_clone.emit(
                "training://event",
                serde_json::json!({ "event": "process_exited" }),
            );
        }
    });

    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Commande: stop training (ecrit le stop flag, Python s'arrete proprement)
// ──────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn ai_stop_training(state: State<'_, Arc<TrainingState>>) -> Result<(), String> {
    let guard = state.stop_flag_path.lock().await;
    if let Some(ref path) = *guard {
        std::fs::write(path, b"stop").map_err(|e| format!("ecriture stop flag: {e}"))?;
        Ok(())
    } else {
        Err("aucun entrainement en cours".into())
    }
}

#[tauri::command]
pub async fn ai_is_training(state: State<'_, Arc<TrainingState>>) -> Result<bool, String> {
    let guard = state.child.lock().await;
    Ok(guard.is_some())
}

// ──────────────────────────────────────────────────────────────
// Commande: export ONNX
// ──────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn ai_export_onnx(
    app: AppHandle,
    model_type: String,
) -> Result<serde_json::Value, String> {
    let python_dir = resolve_python_dir(&app)?;
    let data_dir = resolve_data_dir(&app)?;

    let mut child = Command::new(python_executable())
        .arg(python_dir.join("export_onnx.py"))
        .arg("--model-type").arg(&model_type)
        .arg("--data-root").arg(&data_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn python: {e}"))?;

    let stdout = child.stdout.take().ok_or("pas de stdout")?;
    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    let mut last_done: Option<serde_json::Value> = None;
    let app_clone = app.clone();
    while let Ok(Some(line)) = lines.next_line().await {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if value.get("event").and_then(|v| v.as_str()) == Some("done") {
                last_done = Some(value);
            } else if value.get("event").and_then(|v| v.as_str()) == Some("error") {
                let msg = value.get("message").and_then(|v| v.as_str()).unwrap_or("erreur inconnue");
                return Err(msg.to_string());
            } else {
                let _ = app_clone.emit("training://export", value);
            }
        }
    }

    let _ = child.wait().await;

    last_done.ok_or_else(|| "export_onnx.py n'a pas emis d'evenement done".into())
}
