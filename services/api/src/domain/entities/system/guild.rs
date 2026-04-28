use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guild {
    pub guild_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub member_count: i32,
    pub registered_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
