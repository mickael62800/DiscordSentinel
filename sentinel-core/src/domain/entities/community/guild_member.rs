use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use crate::domain::entities::system::discord_ids::UserId;
use crate::domain::entities::system::discord_ids::GuildId;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildMember {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
    pub roles: serde_json::Value,
    pub joined_at: Option<DateTime<Utc>>,
    pub account_created: Option<DateTime<Utc>>,
    pub is_bot: bool,
    pub last_seen_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberSummary {
    pub member: GuildMember,
    pub conduct: MemberConduct,
    pub infractions: MemberInfractions,
    pub moderation: MemberModeration,
    pub stats: MemberStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberConduct {
    pub points: i32,
    pub max_points: i32,
    pub log: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberInfractions {
    pub total: i64,
    pub recent: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberModeration {
    pub total_warns: i64,
    pub total_mutes: i64,
    pub total_bans: i64,
    pub actions: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberStats {
    pub message_count: i64,
    pub voice_seconds: i64,
    pub last_active: Option<DateTime<Utc>>,
}
