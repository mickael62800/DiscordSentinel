//! Handler HTTP du copilote de moderation (lecture seule, consultatif).
//!
//! `GET /api/moderation/{guild_id}/copilot/{user_id}?lookback_days=&min_precedents=`
//! Renvoie l'historique de moderation du membre + une suggestion de sanction
//! proportionnee et explicable. RBAC : Moderateur+ sur la guild (les appels
//! bot/internal `AuthKind::Internal` passent en pass-through via
//! `check_role_for_guild`). Aucune logique metier ici : mapping + authz.

use axum::extract::Query;
use axum::extract::State;
use axum::Json;
use serde::Deserialize;

use crate::adapters::inbound::http::dto::moderation::copilot::MemberModerationContextDto;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::extractors::ValidatedGuildUser;
use crate::bootstrap::state::ModerationState;

#[derive(Debug, Deserialize)]
pub struct CopilotQuery {
    /// Fenetre d'agregation en jours (defaut 90, borne 1..=365 cote service).
    #[serde(default)]
    pub lookback_days: Option<i64>,
    /// Nombre minimal de precedents pour suivre la jurisprudence (defaut 3).
    #[serde(default)]
    pub min_precedents: Option<u32>,
}

/// GET /api/moderation/{guild_id}/copilot/{user_id}
pub async fn get_member_context(
    State(state): State<ModerationState>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
    Query(params): Query<CopilotQuery>,
) -> Result<Json<MemberModerationContextDto>, ApiError> {

    let lookback_days = params.lookback_days.unwrap_or(90);
    let min_precedents = params.min_precedents.unwrap_or(3);

    let context = state
        .moderation_copilot_uc
        .get_member_context(&guild_id, &user_id, lookback_days, min_precedents)
        .await?;

    Ok(Json(MemberModerationContextDto::from(context)))
}
