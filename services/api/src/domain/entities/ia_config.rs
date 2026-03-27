use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct IaConfig {
    pub guild_id: String,
    pub text_enabled: bool,
    pub text_threshold: f64,
    pub vision_enabled: bool,
    pub vision_threshold: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl IaConfig {
    pub fn default_for_guild(guild_id: &str) -> Self {
        let now = Utc::now();
        Self {
            guild_id: guild_id.to_string(),
            text_enabled: true,
            text_threshold: 0.5,
            vision_enabled: true,
            vision_threshold: 0.5,
            created_at: now,
            updated_at: now,
        }
    }
}
