//! Handler POST /api/wheel/{guild_id}/{user_id}/spin

use axum::extract::Path;
use axum::extract::State;
use axum::Json;
use nexus_core::ports::inbound::play_wheel::PlayWheelCommand;
use nexus_core::ports::inbound::play_wheel::PlayWheelResult;
use serde::Deserialize;
use serde::Serialize;

use super::ApiError;
use crate::bootstrap::AppState;

#[derive(Debug, Deserialize)]
pub struct WheelSpinDto {
    pub username: String,
}

#[derive(Debug, Serialize)]
pub struct WheelSpinResponseDto {
    pub spin_id: String,
    pub case_key: String,
    pub case_label: String,
    pub payout: i64,
    pub balance_after: i64,
    pub is_memorable: bool,
}

impl From<PlayWheelResult> for WheelSpinResponseDto {
    fn from(r: PlayWheelResult) -> Self {
        Self {
            spin_id: r.spin.id.to_string(),
            case_key: r.spin.case_key,
            case_label: r.spin.case_label,
            payout: r.spin.payout,
            balance_after: r.balance_after,
            is_memorable: r.is_memorable,
        }
    }
}

pub async fn spin(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<WheelSpinDto>,
) -> Result<Json<WheelSpinResponseDto>, ApiError> {
    let result = state
        .play_wheel
        .spin(PlayWheelCommand {
            guild_id,
            user_id,
            username: dto.username,
        })
        .await?;
    Ok(Json(WheelSpinResponseDto::from(result)))
}

#[derive(Debug, Serialize)]
pub struct WheelStatusDto {
    /// Le joueur peut-il encore tirer aujourd'hui ?
    pub can_spin: bool,
}

/// GET /api/wheel/{guild_id}/{user_id}/status
///
/// Lecture seule : permet a une interface de fermer son bouton avant tout
/// clic. La regle reste arbitree par `spin` — deux clics simultanes passent
/// tous deux ce controle, seul le claim atomique tranche.
pub async fn status(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<WheelStatusDto>, ApiError> {
    let can_spin = state.play_wheel.can_spin(&guild_id, &user_id).await?;
    Ok(Json(WheelStatusDto { can_spin }))
}
