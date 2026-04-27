//! Handlers HTTP /prank (Phase 3 finalisation audit).
//!
//! Le prank "fausse alerte braquage" affiche un montant aleatoire entre
//! 5_000c et 50_000c. C'est purement cosmetique (pas persiste) mais le
//! RNG vit cote API pour eviter qu'il reste de la "decision" cote bot.

use axum::extract::{Path, State};
use axum::Json;
use rand::Rng;
use serde::Serialize;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;

#[derive(Debug, Serialize)]
pub struct PrankBraquageRollDto {
    pub amount: i64,
}

/// POST /api/coude/{guild_id}/prank/braquage/roll
///
/// Stateless. Retourne un montant random `5_000..=50_000` par pas de 1 000c
/// pour le faux braquage. `guild_id` est ignore mais conserve dans le path
/// pour homogeneite avec les autres endpoints coude.
pub async fn roll_prank_braquage_amount(
    State(_state): State<AppState>,
    Path(_guild_id): Path<String>,
) -> Result<Json<PrankBraquageRollDto>, ApiError> {
    let amount = {
        let mut rng = rand::thread_rng();
        rng.gen_range(5..=50) * 1000
    };
    Ok(Json(PrankBraquageRollDto { amount }))
}
