//! Handlers HTTP pour les vendettas (cf. COUPE_AMELIORATIONS 5.3).

use axum::extract::Path;
use axum::extract::State;
use axum::Json;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::entities::coude::vendetta::ActiveVendetta;
use crate::domain::entities::system::discord_ids::GuildId;

#[derive(Debug, Deserialize)]
pub struct DeclareVendettaDto {
    pub challenger_id: String,
    pub target_id: String,
}

#[derive(Debug, Serialize)]
pub struct DeclaredVendettaDto {
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct ActiveVendettaDto {
    pub id: String,
    pub guild_id: GuildId,
    pub challenger_id: String,
    pub target_id: String,
    pub declared_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub status: String,
    pub resolved_at: Option<DateTime<Utc>>,
}

impl From<ActiveVendetta> for ActiveVendettaDto {
    fn from(v: ActiveVendetta) -> Self {
        Self {
            id: v.id.to_string(),
            guild_id: v.guild_id,
            challenger_id: v.challenger_id,
            target_id: v.target_id,
            declared_at: v.declared_at,
            expires_at: v.expires_at,
            status: v.status.as_db_str().into(),
            resolved_at: v.resolved_at,
        }
    }
}

/// POST /api/coude/{guild_id}/vendettas
pub async fn declare_vendetta(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<DeclareVendettaDto>,
) -> Result<Json<DeclaredVendettaDto>, ApiError> {
    let id = state
        .coude_vendetta_uc
        .declare(&guild_id, &dto.challenger_id, &dto.target_id)
        .await?;
    Ok(Json(DeclaredVendettaDto { id: id.to_string() }))
}

/// GET /api/coude/{guild_id}/vendettas/by-challenger/{challenger_id}
pub async fn list_vendettas_by_challenger(
    State(state): State<AppState>,
    Path((guild_id, challenger_id)): Path<(String, String)>,
) -> Result<Json<Vec<ActiveVendettaDto>>, ApiError> {
    let list = state
        .coude_vendetta_uc
        .list_by_challenger(&guild_id, &challenger_id)
        .await?;
    Ok(Json(list.into_iter().map(Into::into).collect()))
}
