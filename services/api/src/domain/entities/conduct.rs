use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConductConfig {
    pub guild_id: String,
    pub max_points: i32,
    pub regen_amount: i32,
    pub regen_interval: String,
    pub penalty_warn: i32,
    pub penalty_delete: i32,
    pub penalty_mute: i32,
    pub penalty_ban: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ConductConfig {
    pub fn default_for_guild(guild_id: &str) -> Self {
        let now = Utc::now();
        Self {
            guild_id: guild_id.to_string(),
            max_points: 12,
            regen_amount: 1,
            regen_interval: "weekly".to_string(),
            penalty_warn: 1,
            penalty_delete: 2,
            penalty_mute: 3,
            penalty_ban: 6,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn penalty_for_action(&self, action: &str) -> i32 {
        match action {
            "warn" => self.penalty_warn,
            "delete" => self.penalty_delete,
            "mute" => self.penalty_mute,
            "ban" => self.penalty_ban,
            _ => 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConductPoints {
    pub id: Uuid,
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub points: i32,
    pub last_regen_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConductPointsLog {
    pub id: Uuid,
    pub guild_id: String,
    pub user_id: String,
    pub delta: i32,
    pub reason: String,
    pub points_before: i32,
    pub points_after: i32,
    pub created_at: DateTime<Utc>,
}
