//! Handlers HTTP pour les refusals/dette d honneur (cf. COUPE_AMELIORATIONS 5.3).

use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::DateTime;
use chrono::Utc;
use serde::Serialize;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;

#[derive(Debug, Serialize)]
pub struct RefusalCountDto {
    pub count: i32,
    pub last_refused_at: Option<DateTime<Utc>>,
    pub honor_debt_owed: bool,
}

/// POST /api/coude/{g}/refusals/{requester}/{refuser}/increment
pub async fn increment_refusal(
    State(state): State<AppState>,
    Path((guild_id, requester_id, refuser_id)): Path<(String, String, String)>,
) -> Result<Json<RefusalCountDto>, ApiError> {
    let count = state
        .coude_refusal_count_repo
        .increment(&guild_id, &requester_id, &refuser_id)
        .await?;
    // Seuil de dette d honneur reglable par serveur (coude-bot).
    let econ = sentinel_core::application::coude::guild_settings::load_economy_config(
        state.bot_config_repo.as_ref(),
        &guild_id,
    )
    .await;
    Ok(Json(RefusalCountDto {
        count,
        last_refused_at: Some(Utc::now()),
        honor_debt_owed: count >= econ.honor_debt_threshold,
    }))
}

/// GET /api/coude/{g}/refusals/{requester}/{refuser}
pub async fn get_refusal(
    State(state): State<AppState>,
    Path((guild_id, requester_id, refuser_id)): Path<(String, String, String)>,
) -> Result<Json<RefusalCountDto>, ApiError> {
    let r = state
        .coude_refusal_count_repo
        .get(&guild_id, &requester_id, &refuser_id)
        .await?;
    let (count, ts) = match r {
        Some(rc) => (rc.count, Some(rc.last_refused_at)),
        None => (0, None),
    };
    // Seuil de dette d honneur reglable par serveur (coude-bot).
    let econ = sentinel_core::application::coude::guild_settings::load_economy_config(
        state.bot_config_repo.as_ref(),
        &guild_id,
    )
    .await;
    Ok(Json(RefusalCountDto {
        count,
        last_refused_at: ts,
        honor_debt_owed: count >= econ.honor_debt_threshold,
    }))
}

/// POST /api/coude/{g}/refusals/{requester}/{refuser}/reset
pub async fn reset_refusal(
    State(state): State<AppState>,
    Path((guild_id, requester_id, refuser_id)): Path<(String, String, String)>,
) -> Result<StatusCode, ApiError> {
    state
        .coude_refusal_count_repo
        .reset(&guild_id, &requester_id, &refuser_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
