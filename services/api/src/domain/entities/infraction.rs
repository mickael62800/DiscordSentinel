use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::value_objects::{Action, DetectionFlags};

#[derive(Debug, Clone)]
pub struct Infraction {
    pub id: Uuid,
    pub guild_id: String,
    pub channel_id: String,
    pub user_id: String,
    pub username: String,
    pub message_id: String,
    pub content: String,
    pub flags: DetectionFlags,
    pub score: f64,
    pub action: Action,
    pub reason: String,
    pub duration: Option<u64>,
    pub created_at: DateTime<Utc>,
}
