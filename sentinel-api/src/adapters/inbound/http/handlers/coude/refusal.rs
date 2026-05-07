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
    use sentinel_core::domain::entities::coude::refusal_count::HONOR_DEBT_THRESHOLD;
    Ok(Json(RefusalCountDto {
        count,
        last_refused_at: Some(Utc::now()),
        honor_debt_owed: count >= HONOR_DEBT_THRESHOLD,
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
    use sentinel_core::domain::entities::coude::refusal_count::HONOR_DEBT_THRESHOLD;
    let (count, ts) = match r {
        Some(rc) => (rc.count, Some(rc.last_refused_at)),
        None => (0, None),
    };
    Ok(Json(RefusalCountDto {
        count,
        last_refused_at: ts,
        honor_debt_owed: count >= HONOR_DEBT_THRESHOLD,
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
