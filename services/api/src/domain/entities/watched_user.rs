use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct WatchedUser {
    pub user_id: String,
    pub username: String,
    pub guild_id: String,
    pub guild_name: String,
    pub risk_level: String,
    pub total_warns: i64,
    pub total_mutes: i64,
    pub total_bans: i64,
    pub conduct_points: Option<i32>,
    pub max_conduct_points: Option<i32>,
    pub last_incident_at: Option<DateTime<Utc>>,
    pub security_events_count: i64,
    pub first_seen_at: DateTime<Utc>,
}
