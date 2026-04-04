use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::adapters::inbound::http::state::AppState;

#[derive(Debug, Serialize)]
pub struct ModelInfo {
    pub name: String,
    pub model_type: String,
    pub loaded: bool,
}

#[derive(Debug, Serialize)]
pub struct ModelsStatusResponse {
    pub models: Vec<ModelInfo>,
}

/// GET /api/models/status — retourne l'etat des modeles IA charges
pub async fn get_models_status(
    State(state): State<AppState>,
) -> Json<ModelsStatusResponse> {
    let vision_path = std::env::var("VISION_MODEL_PATH").unwrap_or_default();
    let text_path = std::env::var("TEXT_MODEL_PATH").unwrap_or_default();

    let models = vec![
        ModelInfo {
            name: if vision_path.is_empty() {
                "Vision ONNX (non configure)".to_string()
            } else {
                format!("Vision ONNX ({})", vision_path.rsplit('/').next().unwrap_or(&vision_path))
            },
            model_type: "vision".to_string(),
            loaded: state.inference.vision_available(),
        },
        ModelInfo {
            name: if text_path.is_empty() {
                "Text ONNX (non configure)".to_string()
            } else {
                format!("Text ONNX ({})", text_path.rsplit('/').next().unwrap_or(&text_path))
            },
            model_type: "text".to_string(),
            loaded: state.inference.text_available(),
        },
    ];

    Json(ModelsStatusResponse { models })
}

#[derive(Debug, Deserialize)]
pub struct ReloadRequest {
    pub model_type: String,
}

#[derive(Debug, Serialize)]
pub struct ReloadResponse {
    pub success: bool,
    pub message: String,
}

/// POST /api/models/reload — recharge un modele ONNX a chaud
pub async fn reload_model(
    State(state): State<AppState>,
    Json(req): Json<ReloadRequest>,
) -> (StatusCode, Json<ReloadResponse>) {
    info!(model_type = %req.model_type, "Rechargement du modele demande");
    match state.inference.reload(&req.model_type) {
        Ok(msg) => {
            info!("{}", msg);
            (StatusCode::OK, Json(ReloadResponse { success: true, message: msg }))
        }
        Err(msg) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ReloadResponse { success: false, message: msg }))
        }
    }
}
