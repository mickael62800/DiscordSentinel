//! Handlers citoyens (profil).

use axum::extract::State;
use axum::Json;
use serde::Deserialize;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::extractors::ValidatedGuild;
use crate::adapters::inbound::http::handlers::influence::dto::ProfileViewDto;
use crate::adapters::inbound::http::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ViewProfileDto {
    /// Discord user id de celui qui consulte (decide chiffre vs palier).
    pub viewer_user_id: String,
    /// Discord user id du citoyen consulte.
    pub target_user_id: String,
    /// Pseudo du citoyen consulte (pour l'enregistrement auto).
    #[serde(default)]
    pub target_username: String,
}

/// POST /api/influence/{guild_id}/profile
pub async fn view_profile(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<ViewProfileDto>,
) -> Result<Json<ProfileViewDto>, ApiError> {
    let view = state
        .influence_view_profile_uc
        .view(
            &guild_id,
            &dto.viewer_user_id,
            &dto.target_user_id,
            &dto.target_username,
        )
        .await?;
    Ok(Json(view.into()))
}
