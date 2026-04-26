//! Handlers HTTP pour les primes collectives (cf. COUPE_AMELIORATIONS 5.3).

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::entities::{ActiveBounty, BOUNTY_MIN_CONTRIBUTION};
use crate::domain::errors::DomainError;

#[derive(Debug, Serialize)]
pub struct ActiveBountyDto {
    pub id: String,
    pub guild_id: String,
    pub target_id: String,
    pub total_amount: i64,
    pub status: String,
    pub opened_at: DateTime<Utc>,
    pub claimed_by: Option<String>,
    pub claimed_at: Option<DateTime<Utc>>,
}

impl From<ActiveBounty> for ActiveBountyDto {
    fn from(b: ActiveBounty) -> Self {
        Self {
            id: b.id.to_string(),
            guild_id: b.guild_id,
            target_id: b.target_id,
            total_amount: b.total_amount,
            status: b.status.as_db_str().into(),
            opened_at: b.opened_at,
            claimed_by: b.claimed_by,
            claimed_at: b.claimed_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ContributeBountyDto {
    pub contributor_id: String,
    pub contributor_name: String,
    pub amount: i64,
}

#[derive(Debug, Serialize)]
pub struct ContributedBountyDto {
    pub bounty_id: String,
    pub new_total: i64,
}

/// GET /api/coude/{guild_id}/bounties/by-target/{target_id}
pub async fn get_bounty_by_target(
    State(state): State<AppState>,
    Path((guild_id, target_id)): Path<(String, String)>,
) -> Result<Json<Option<ActiveBountyDto>>, ApiError> {
    let b = state
        .coude_bounty_repo
        .get_open(&guild_id, &target_id)
        .await?;
    Ok(Json(b.map(Into::into)))
}

/// POST /api/coude/{guild_id}/bounties/by-target/{target_id}/contribute
///
/// Endpoint d ergonomie : auto-resolve la prime ouverte sur la cible et
/// y ajoute le montant. Echoue si pas de prime ouverte.
pub async fn contribute_to_target(
    State(state): State<AppState>,
    Path((guild_id, target_id)): Path<(String, String)>,
    Json(dto): Json<ContributeBountyDto>,
) -> Result<Json<ContributedBountyDto>, ApiError> {
    if dto.amount < BOUNTY_MIN_CONTRIBUTION {
        return Err(ApiError::from(DomainError::ValidationError(format!(
            "Contribution minimum : {}c.",
            BOUNTY_MIN_CONTRIBUTION
        ))));
    }
    let bounty = state
        .coude_bounty_repo
        .get_open(&guild_id, &target_id)
        .await?
        .ok_or_else(|| {
            ApiError::from(DomainError::NotFound(
                "Aucune prime ouverte sur cette cible.".into(),
            ))
        })?;
    let new_total = state
        .coude_bounty_repo
        .contribute(
            bounty.id,
            &dto.contributor_id,
            &dto.contributor_name,
            dto.amount,
        )
        .await?;
    Ok(Json(ContributedBountyDto {
        bounty_id: bounty.id.to_string(),
        new_total,
    }))
}

/// GET /api/coude/{guild_id}/bounties/min-contribution
pub async fn get_bounty_config(
    State(_state): State<AppState>,
    Path(_guild_id): Path<String>,
) -> Result<Json<i64>, ApiError> {
    Ok(Json(BOUNTY_MIN_CONTRIBUTION))
}

#[allow(dead_code)]
pub async fn _placeholder(_state: State<AppState>) -> StatusCode {
    StatusCode::NO_CONTENT
}
