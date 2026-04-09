use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sentinel_shared::api_client::BaseApiClient;

// ── Response DTOs ──

#[derive(Debug, Clone, Deserialize)]
pub struct CardDto {
    pub rank: String,
    pub suit: String,
    pub filename: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlackjackGameDto {
    pub id: String,
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub bet: i64,
    pub player_hand: Vec<CardDto>,
    pub dealer_hand: Vec<CardDto>,
    pub status: String,
    pub player_score: i32,
    pub dealer_score: i32,
    pub doubled: bool,
    pub payout: i64,
    pub created_at: String,
    pub finished_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WalletDto {
    pub id: String,
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub coins: i64,
    pub total_earned: i64,
    pub total_spent: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct TableDto {
    pub id: String,
    pub guild_id: String,
    pub channel_id: String,
    pub owner_id: String,
    pub owner_name: String,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct TablePlayerDto {
    pub user_id: String,
    pub user_name: String,
}

// ── Request DTOs ──

#[derive(Serialize)]
struct StartGamePayload {
    guild_id: String,
    user_id: String,
    username: String,
    bet: i64,
}

// ── API Client ──

pub struct ApiClient {
    pub base: Arc<BaseApiClient>,
}

impl ApiClient {
    pub fn new(base: Arc<BaseApiClient>) -> Self {
        Self { base }
    }

    // ── Blackjack ──

    pub async fn start_game(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        bet: i64,
    ) -> Result<BlackjackGameDto, String> {
        self.base
            .post_json(
                "/api/blackjack/start",
                &StartGamePayload {
                    guild_id: guild_id.to_string(),
                    user_id: user_id.to_string(),
                    username: username.to_string(),
                    bet,
                },
            )
            .await
    }

    pub async fn hit(&self, game_id: &str) -> Result<BlackjackGameDto, String> {
        self.base
            .post_json(&format!("/api/blackjack/{game_id}/hit"), &())
            .await
    }

    pub async fn stand(&self, game_id: &str) -> Result<BlackjackGameDto, String> {
        self.base
            .post_json(&format!("/api/blackjack/{game_id}/stand"), &())
            .await
    }

    pub async fn double_down(&self, game_id: &str) -> Result<BlackjackGameDto, String> {
        self.base
            .post_json(&format!("/api/blackjack/{game_id}/double"), &())
            .await
    }

    pub async fn get_active(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<BlackjackGameDto>, String> {
        self.base
            .get_json(&format!("/api/blackjack/{guild_id}/{user_id}/active"))
            .await
    }

    // ── Tables (multijoueur) ──

    pub async fn create_table(&self, guild_id: &str, channel_id: &str, owner_id: &str, owner_name: &str) -> Result<TableDto, String> {
        self.base.post_json("/api/blackjack/tables", &serde_json::json!({
            "guild_id": guild_id, "channel_id": channel_id, "owner_id": owner_id, "owner_name": owner_name,
        })).await
    }

    pub async fn join_table(&self, table_id: &str, user_id: &str, user_name: &str) -> Result<(), String> {
        self.base.post_fire_and_forget(
            &format!("/api/blackjack/tables/{table_id}/join"),
            &serde_json::json!({ "user_id": user_id, "user_name": user_name }),
        ).await;
        Ok(())
    }

    pub async fn get_table_by_channel(&self, channel_id: &str) -> Result<Option<TableDto>, String> {
        self.base.get_json(&format!("/api/blackjack/tables/by-channel/{channel_id}")).await
    }

    pub async fn list_table_players(&self, table_id: &str) -> Result<Vec<TablePlayerDto>, String> {
        self.base.get_json(&format!("/api/blackjack/tables/{table_id}/players")).await
    }

    pub async fn close_table(&self, table_id: &str) -> Result<(), String> {
        let req = self.base.client().delete(format!("{}/api/blackjack/tables/{}", self.base.base_url(), table_id));
        let _ = self.base.auth(req).send().await;
        Ok(())
    }

    // ── Wallet ──

    pub async fn get_wallet(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<WalletDto, String> {
        self.base
            .get_json(&format!("/api/wallet/{guild_id}/{user_id}"))
            .await
    }
}
