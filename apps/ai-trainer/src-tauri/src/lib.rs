mod commands;

use std::sync::Arc;
use tokio::sync::Mutex;

pub struct TrainingState {
    pub child: Mutex<Option<tokio::process::Child>>,
    pub stop_flag_path: Mutex<Option<std::path::PathBuf>>,
}

impl TrainingState {
    pub fn new() -> Self {
        Self {
            child: Mutex::new(None),
            stop_flag_path: Mutex::new(None),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Arc::new(TrainingState::new()))
        .invoke_handler(tauri::generate_handler![
            commands::ai_get_datasets,
            commands::ai_upload_dataset,
            commands::ai_start_training,
            commands::ai_stop_training,
            commands::ai_is_training,
            commands::ai_export_onnx,
        ])
        .run(tauri::generate_context!())
        .expect("erreur au demarrage de Sentinel AI Trainer");
}
