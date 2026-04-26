//! Handlers HTTP pour les ultimates par classe (cf. COUPE_AMELIORATIONS 3.1).

use axum::extract::{Path, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::entities::{ultimate_ready, UltimateKind};
use crate::domain::errors::DomainError;

#[derive(Debug, Deserialize)]
pub struct ActivateUltimateDto {
    pub kind: String,
}

#[derive(Debug, Serialize)]
pub struct UltimateStateDto {
    pub pending_kind: Option<String>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub activated_at: Option<DateTime<Utc>>,
}

/// POST /api/coude/{guild_id}/ultimates/{user_id}/activate
///
/// Active une ultimate. Validations :
/// - kind valide
/// - level joueur >= 10
/// - cooldown ecoule
/// - classe joueur correspond au kind
pub async fn activate_ultimate(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<ActivateUltimateDto>,
) -> Result<Json<UltimateStateDto>, ApiError> {
    let kind = UltimateKind::from_db_str(&dto.kind).ok_or_else(|| {
        ApiError::from(DomainError::ValidationError(format!(
            "Kind d ultimate inconnu : {}",
            dto.kind
        )))
    })?;
    // Verifier classe + level via players_uc (lecture).
    let player = state.coude_players_uc.get(&guild_id, &user_id).await?;
    let class_key = player.class.as_ref().map(|c| c.as_str()).unwrap_or("");
    if class_key != kind.class_key() {
        return Err(ApiError::from(DomainError::ValidationError(format!(
            "Cet ultimate est reserve a la classe {} (la tienne : {}).",
            kind.class_key(),
            class_key
        ))));
    }
    // Cooldown.
    let current = state
        .coude_ultimate_repo
        .get(&guild_id, &user_id)
        .await?;
    if !ultimate_ready(player.level, kind, current.last_used_at) {
        return Err(ApiError::from(DomainError::Conflict(format!(
            "Ultimate non disponible (level requis 10, cooldown {} jours).",
            kind.cooldown_days()
        ))));
    }
    state
        .coude_ultimate_repo
        .activate(&guild_id, &user_id, kind)
        .await?;
    let state_after = state
        .coude_ultimate_repo
        .get(&guild_id, &user_id)
        .await?;
    Ok(Json(UltimateStateDto {
        pending_kind: state_after.pending_kind.map(|k| k.as_db_str().into()),
        last_used_at: state_after.last_used_at,
        activated_at: state_after.activated_at,
    }))
}

/// GET /api/coude/{guild_id}/ultimates/{user_id}
pub async fn get_ultimate_state(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<UltimateStateDto>, ApiError> {
    let s = state.coude_ultimate_repo.get(&guild_id, &user_id).await?;
    Ok(Json(UltimateStateDto {
        pending_kind: s.pending_kind.map(|k| k.as_db_str().into()),
        last_used_at: s.last_used_at,
        activated_at: s.activated_at,
    }))
}
