use async_trait::async_trait;

use crate::domain::errors::DomainError;

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct BlackjackTable {
    pub id: String,
    pub guild_id: String,
    pub channel_id: String,
    pub owner_id: String,
    pub owner_name: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct BlackjackTablePlayer {
    pub user_id: String,
    pub user_name: String,
    pub joined_at: String,
}

#[async_trait]
pub trait BlackjackTableRepository: Send + Sync {
    async fn create(&self, guild_id: &str, channel_id: &str, owner_id: &str, owner_name: &str, deck_json: &serde_json::Value) -> Result<BlackjackTable, DomainError>;
    async fn get_status_and_guild(&self, table_id: &str) -> Result<Option<(String, String)>, DomainError>;
    async fn count_players(&self, table_id: &str) -> Result<i64, DomainError>;
    async fn add_player(&self, table_id: &str, user_id: &str, user_name: &str) -> Result<(), DomainError>;
    async fn touch_activity(&self, table_id: &str) -> Result<(), DomainError>;
    async fn list_players(&self, table_id: &str) -> Result<Vec<BlackjackTablePlayer>, DomainError>;
    async fn find_open_by_channel(&self, channel_id: &str) -> Result<Option<BlackjackTable>, DomainError>;
    async fn get_guild_id(&self, table_id: &str) -> Result<Option<String>, DomainError>;
    async fn close(&self, table_id: &str) -> Result<(), DomainError>;
    async fn list_games(&self, table_id: &str) -> Result<Vec<serde_json::Value>, DomainError>;
}
