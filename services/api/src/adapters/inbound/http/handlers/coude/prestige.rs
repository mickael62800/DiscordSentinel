//! Handlers HTTP pour le systeme de Prestige (cf. COUPE_AMELIORATIONS 3.3).

use axum::extract::Path;
use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::entities::coude::prestige::PRESTIGE_MAX_COUNT;
use crate::domain::errors::DomainError;

#[derive(Debug, Serialize)]
pub struct PrestigeOutcomeDto {
    pub new_prestige_count: i32,
    pub stars: String,
}

/// POST /api/coude/{guild_id}/players/{user_id}/prestige
///
/// Effectue un prestige : valide level >= 25 ET prestige_count < MAX,
/// puis reset niveau/xp/stat_points/atk/def et incremente
/// prestige_count. Atomic via une seule UPDATE conditionnelle.
pub async fn prestige_player(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<PrestigeOutcomeDto>, ApiError> {
    // Cf. migration 170 : seuils configurables par-guild.
    let settings = crate::application::coude::guild_settings::CoudeGuildSettings::load(
        state.bot_config_repo.as_ref(),
        &guild_id,
    )
    .await;
    let unlock_level = settings.get_i32(
        "prestige_unlock_level",
        crate::domain::entities::coude::prestige::PRESTIGE_UNLOCK_LEVEL,
    );
    let max_count = settings.get_i32("prestige_max_count", PRESTIGE_MAX_COUNT);
    // Atomic : UPDATE conditionnel (WHERE level >= 25 AND prestige_count
    // < MAX) pour eviter une race + valider eligibilite en 1 query. Si
    // le RETURNING est vide, le joueur n est pas eligible.
    let row: Option<(i32,)> = sqlx::query_as(
        r#"UPDATE coude_players
           SET prestige_count = prestige_count + 1,
               level = 1,
               xp = 0,
               stat_points = 0,
               atk = 0,
               def = 0,
               updated_at = NOW()
           WHERE guild_id = $1 AND user_id = $2
             AND level >= $3
             AND prestige_count < $4
           RETURNING prestige_count"#,
    )
    .bind(&guild_id)
    .bind(&user_id)
    .bind(unlock_level)
    .bind(max_count)
    .fetch_optional(&state.pg_pool)
    .await
    .map_err(|e| {
        ApiError::from(DomainError::Internal(format!(
            "prestige UPDATE pg: {e}"
        )))
    })?;

    let new_count = row.map(|r| r.0).ok_or_else(|| {
        ApiError::from(DomainError::Conflict(format!(
            "Prestige indisponible : il faut etre niveau {}+ et avoir moins de {} prestiges.",
            unlock_level, max_count
        )))
    })?;
    Ok(Json(PrestigeOutcomeDto {
        new_prestige_count: new_count,
        stars: crate::domain::entities::coude::prestige::prestige_stars(new_count),
    }))
}
