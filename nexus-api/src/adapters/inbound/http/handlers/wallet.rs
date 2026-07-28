//! Handler GET /api/wallet/{guild_id}/{user_id}

use axum::extract::Path;
use axum::extract::State;
use axum::Json;
use serde::Serialize;

use super::ApiError;
use crate::bootstrap::AppState;

#[derive(Debug, Serialize)]
pub struct WalletDto {
    pub guild_id: String,
    pub user_id: String,
    pub coins: i64,
    pub total_earned: i64,
    pub total_spent: i64,
}

pub async fn get(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<WalletDto>, ApiError> {
    let w = state.get_wallet.get(&guild_id, &user_id).await?;
    Ok(Json(WalletDto {
        guild_id: w.guild_id,
        user_id: w.user_id,
        coins: w.coins,
        total_earned: w.total_earned,
        total_spent: w.total_spent,
    }))
}
