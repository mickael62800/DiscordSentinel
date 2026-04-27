//! Handler HTTP RollSteal (Phase 2 #4 audit).

use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::ports::inbound::roll_steal::{RollStealCommand, StealRoll};

#[derive(Debug, Deserialize)]
pub struct RollStealDto {
    pub afk: bool,
}

#[derive(Debug, Serialize)]
pub struct StealRollDto {
    pub thief_d20: i32,
    pub victim_d20: i32,
    pub steal_pct_bp: u32,
}

impl From<StealRoll> for StealRollDto {
    fn from(r: StealRoll) -> Self {
        Self {
            thief_d20: r.thief_d20,
            victim_d20: r.victim_d20,
            steal_pct_bp: r.steal_pct_bp,
        }
    }
}

/// POST /api/coude/{guild_id}/steal/roll
pub async fn roll_steal(
    State(state): State<AppState>,
    Path(_guild_id): Path<String>,
    Json(dto): Json<RollStealDto>,
) -> Result<Json<StealRollDto>, ApiError> {
    let r = state
        .roll_steal_uc
        .roll(RollStealCommand { afk: dto.afk })
        .await?;
    Ok(Json(r.into()))
}
