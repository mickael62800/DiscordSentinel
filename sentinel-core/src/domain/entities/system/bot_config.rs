use crate::domain::entities::system::discord_ids::GuildId;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotGuildConfig {
    pub id: Uuid,
    pub guild_id: GuildId,
    pub bot_name: String,
    pub config_key: String,
    pub config_value: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotDefinition {
    pub bot_name: String,
    pub display_name: String,
    pub description: String,
    pub config_schema: serde_json::Value,
}
