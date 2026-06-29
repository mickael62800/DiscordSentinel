//! Handler HTTP pour le duel amical (cf. COUPE_AMELIORATIONS 4.5).

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::extractors::ValidatedGuild;
use crate::adapters::inbound::http::state::AppState;
use crate::ports::inbound::coude::resolve_friendly_duel::FriendlyDuelInput;
use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize)]
pub struct FriendlyDuelRequest {
    pub attacker_id: String,
    pub attacker_name: String,
    pub defender_id: String,
    pub defender_name: String,
}

#[derive(Debug, Serialize)]
pub struct FriendlyDuelResponse {
    pub winner_id: Option<String>,
    pub loser_id: Option<String>,
    pub draw: bool,
    pub total_rounds: i32,
    pub attacker_hp_final: i32,
    pub attacker_hp_max: i32,
    pub defender_hp_final: i32,
    pub defender_hp_max: i32,
    pub winner_xp: i64,
    pub loser_xp: i64,
}

/// POST /api/coude/{guild_id}/friendly-duels
pub async fn resolve_friendly_duel(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(req): Json<FriendlyDuelRequest>,
) -> Result<Json<FriendlyDuelResponse>, ApiError> {
    let out = state
        .resolve_friendly_duel_uc
        .resolve(FriendlyDuelInput {
            guild_id: guild_id.into(),
            attacker_id: req.attacker_id,
            attacker_name: req.attacker_name,
            defender_id: req.defender_id,
            defender_name: req.defender_name,
        })
        .await?;
    Ok(Json(FriendlyDuelResponse {
        winner_id: out.winner_id,
        loser_id: out.loser_id,
        draw: out.draw,
        total_rounds: out.total_rounds,
        attacker_hp_final: out.attacker_hp_final,
        attacker_hp_max: out.attacker_hp_max,
        defender_hp_final: out.defender_hp_final,
        defender_hp_max: out.defender_hp_max,
        winner_xp: out.winner_xp,
        loser_xp: out.loser_xp,
    }))
}
