use async_trait::async_trait;

use crate::domain::errors::DomainError;

#[derive(Debug, Clone)]
pub struct Game {
    pub id: String,
    pub guild_id: String,
    pub game_name: String,
    pub created_by: String,
    pub created_at: String,
    pub emoji: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GamePanel {
    pub id: String,
    pub guild_id: String,
    pub channel_id: String,
    pub message_id: String,
    pub category: Option<String>,
}

#[async_trait]
pub trait GameRepository: Send + Sync {
    async fn list(&self, guild_id: &str) -> Result<Vec<Game>, DomainError>;
    async fn list_by_category(&self, guild_id: &str, category: Option<&str>) -> Result<Vec<Game>, DomainError>;
    async fn create(&self, guild_id: &str, game_name: &str, created_by: &str, emoji: Option<&str>, category: Option<&str>) -> Result<Game, DomainError>;
    async fn delete(&self, guild_id: &str, game_id: &str) -> Result<bool, DomainError>;
    async fn find_by_name(&self, guild_id: &str, game_name: &str) -> Result<Option<Game>, DomainError>;
    async fn subscribe(&self, guild_id: &str, game_id: &str, user_id: &str) -> Result<(), DomainError>;
    async fn unsubscribe(&self, guild_id: &str, game_id: &str, user_id: &str) -> Result<(), DomainError>;
    async fn get_subscribers(&self, game_id: &str) -> Result<Vec<String>, DomainError>;
    async fn get_user_games(&self, guild_id: &str, user_id: &str) -> Result<Vec<Game>, DomainError>;

    // Panels
    async fn save_panel(&self, guild_id: &str, channel_id: &str, message_id: &str, category: Option<&str>) -> Result<GamePanel, DomainError>;
    async fn find_panel_by_message(&self, guild_id: &str, message_id: &str) -> Result<Option<GamePanel>, DomainError>;
    async fn list_panels(&self, guild_id: &str) -> Result<Vec<GamePanel>, DomainError>;
}
