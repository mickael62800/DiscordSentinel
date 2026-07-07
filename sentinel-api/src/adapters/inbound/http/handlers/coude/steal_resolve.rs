//! Handler HTTP ResolveSteal : resolution serveur-side complete de `/voler`.
//!
//! Remplace le couple `roll_steal` (des bruts) + decision client-side du
//! bot. Le serveur decide l'issue, calcule butin/penalite (clamp serveur),
//! mute les wallets atomiquement et renvoie l'embed pret a poster.

use super::taunts::TauntEventDto;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::ports::inbound::coude::resolve_steal::ResolveStealCommand;
use crate::ports::inbound::coude::resolve_steal::ResolveStealOutput;
use crate::ports::inbound::coude::resolve_steal::StealResolutionOutcome;
use axum::extract::Path;
use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize)]
pub struct ResolveStealDto {
    pub thief_id: String,
    pub target_id: String,
    pub afk: bool,
}

#[derive(Debug, Serialize)]
pub struct ResolveStealResponse {
    /// "success" | "failed" | "blocked".
    pub outcome: String,
    pub title: String,
    pub description: String,
    pub color: u32,
    pub stolen: i64,
    pub lost: i64,
    pub thief_roll: i32,
    pub victim_roll: i32,
    pub taunt_events: Vec<TauntEventDto>,
}

fn outcome_str(o: StealResolutionOutcome) -> &'static str {
    match o {
        StealResolutionOutcome::Success => "success",
        StealResolutionOutcome::Failed => "failed",
        StealResolutionOutcome::Blocked => "blocked",
    }
}

impl From<ResolveStealOutput> for ResolveStealResponse {
    fn from(o: ResolveStealOutput) -> Self {
        Self {
            outcome: outcome_str(o.outcome).to_string(),
            title: o.title,
            description: o.description,
            color: o.color,
            stolen: o.stolen,
            lost: o.lost,
            thief_roll: o.thief_roll,
            victim_roll: o.victim_roll,
            taunt_events: o.taunt_events.into_iter().map(Into::into).collect(),
        }
    }
}

/// POST /api/coude/{guild_id}/steal/resolve
pub async fn resolve_steal(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<ResolveStealDto>,
) -> Result<Json<ResolveStealResponse>, ApiError> {
    let out = state
        .resolve_steal_uc
        .resolve_steal(ResolveStealCommand {
            guild_id,
            thief_id: dto.thief_id,
            target_id: dto.target_id,
            afk: dto.afk,
        })
        .await?;
    Ok(Json(out.into()))
}
