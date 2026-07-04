//! Handler : consultation de la memoire du serveur (archives / actu).

use axum::extract::State;
use axum::Json;
use serde::Deserialize;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::extractors::ValidatedGuild;
use crate::adapters::inbound::http::handlers::influence::dto::ArchiveEntryDto;
use crate::adapters::inbound::http::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ArchivesQueryDto {
    /// Nombre d'evenements (None = taille configuree du fil).
    #[serde(default)]
    pub limit: Option<i64>,
}

/// POST /api/influence/{guild_id}/archives
pub async fn list_archives(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<ArchivesQueryDto>,
) -> Result<Json<Vec<ArchiveEntryDto>>, ApiError> {
    let entries = state
        .influence_archives_uc
        .list(&guild_id, dto.limit)
        .await?;
    Ok(Json(entries.into_iter().map(Into::into).collect()))
}
