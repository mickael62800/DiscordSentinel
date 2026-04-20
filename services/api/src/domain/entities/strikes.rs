use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrikeThreshold {
    pub strikes: u32,
    pub action: String,
    pub duration: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrikeConfig {
    pub guild_id: String,
    pub window_secs: i64,
    pub thresholds: Vec<StrikeThreshold>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl StrikeConfig {
    pub fn default_for_guild(guild_id: &str) -> Self {
        let now = Utc::now();
        Self {
            guild_id: guild_id.to_string(),
            window_secs: 3600,
            thresholds: vec![],
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStrike {
    pub id: Uuid,
    pub guild_id: String,
    pub user_id: String,
    pub reason: String,
    pub source: String,
    pub infraction_id: Option<Uuid>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrikeResult {
    pub strike: UserStrike,
    pub active_count: u32,
    pub escalation_action: Option<String>,
    pub escalation_duration: Option<u64>,
}

#[cfg(test)]
#[path = "tests/strikes.rs"]
mod tests;
