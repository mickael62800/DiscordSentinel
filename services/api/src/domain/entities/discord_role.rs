use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DiscordRole {
    pub id: String,
    pub guild_id: String,
    pub name: String,
    pub color: i32,
    pub position: i32,
    pub permissions: i64,
    pub mentionable: bool,
    pub managed: bool,
    pub icon: Option<String>,
    pub member_count: i32,
    pub synced_at: DateTime<Utc>,
}
