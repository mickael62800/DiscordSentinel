//! Handlers HTTP pour les maledictions (cf. COUPE_AMELIORATIONS 5.1).
//!
//!   - POST /api/coude/{guild_id}/curses          — cast
//!   - GET  /api/coude/{guild_id}/curses/{target} — get active
//!   - POST /api/coude/{guild_id}/curses/{target}/lift — la cible leve
//!
//! Logique metier zero : delegation au use case.

use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use sentinel_core::domain::entities::coude::curse::ActiveCurse;
use sentinel_core::domain::entities::coude::curse::CurseKind;
use sentinel_core::domain::errors::DomainError;
use sentinel_core::domain::entities::system::discord_ids::GuildId;

#[derive(Debug, Deserialize)]
pub struct CastCurseDto {
    pub source_id: String,
    pub source_username: String,
    pub target_id: String,
    /// Si absent, tirage aleatoire parmi les 6 maledictions.
    #[serde(default)]
    pub kind: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CastedCurseDto {
    pub id: String,
    pub kind: String,
    pub kind_label: String,
    pub kind_emoji: String,
    pub cost_paid: i64,
}

#[derive(Debug, Serialize)]
pub struct ActiveCurseDto {
    pub id: String,
    pub guild_id: GuildId,
    pub target_id: String,
    pub source_id: String,
    pub kind: String,
    pub kind_label: String,
    pub kind_emoji: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub lifted_at: Option<DateTime<Utc>>,
    pub lifted_by: Option<String>,
}

impl From<ActiveCurse> for ActiveCurseDto {
    fn from(c: ActiveCurse) -> Self {
        Self {
            id: c.id.to_string(),
            guild_id: c.guild_id,
            target_id: c.target_id,
            source_id: c.source_id,
            kind: c.kind.as_db_str().into(),
            kind_label: c.kind.label().into(),
            kind_emoji: c.kind.emoji().into(),
            created_at: c.created_at,
            expires_at: c.expires_at,
            lifted_at: c.lifted_at,
            lifted_by: c.lifted_by,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct LiftCurseDto {
    pub target_username: String,
}

fn parse_kind(s: &str) -> Result<CurseKind, ApiError> {
    CurseKind::from_db_str(s).ok_or_else(|| {
        ApiError::from(DomainError::ValidationError(format!(
            "Type de malediction inconnu : {s}"
        )))
    })
}

/// POST /api/coude/{guild_id}/curses
pub async fn cast_curse(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<CastCurseDto>,
) -> Result<Json<CastedCurseDto>, ApiError> {
    let kind = match dto.kind.as_deref() {
        Some(s) => Some(parse_kind(s)?),
        None => None,
    };
    let out = state
        .coude_curses_uc
        .cast(
            &guild_id,
            &dto.source_id,
            &dto.source_username,
            &dto.target_id,
            kind,
        )
        .await?;
    Ok(Json(CastedCurseDto {
        id: out.id.to_string(),
        kind: out.kind.as_db_str().into(),
        kind_label: out.kind.label().into(),
        kind_emoji: out.kind.emoji().into(),
        cost_paid: out.cost_paid,
    }))
}

/// GET /api/coude/{guild_id}/curses/{target_id}
pub async fn get_active_curse(
    State(state): State<AppState>,
    Path((guild_id, target_id)): Path<(String, String)>,
) -> Result<Json<Option<ActiveCurseDto>>, ApiError> {
    let curse = state
        .coude_curses_uc
        .get_active(&guild_id, &target_id)
        .await?;
    Ok(Json(curse.map(Into::into)))
}

/// POST /api/coude/{guild_id}/curses/{target_id}/lift
pub async fn lift_curse(
    State(state): State<AppState>,
    Path((guild_id, target_id)): Path<(String, String)>,
    Json(dto): Json<LiftCurseDto>,
) -> Result<(StatusCode, Json<ActiveCurseDto>), ApiError> {
    let updated = state
        .coude_curses_uc
        .lift_own(&guild_id, &target_id, &dto.target_username)
        .await?;
    Ok((StatusCode::OK, Json(updated.into())))
}
