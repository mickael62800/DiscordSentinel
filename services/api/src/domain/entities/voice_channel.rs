use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceChannel {
    pub id: Uuid,
    pub guild_id: String,
    pub owner_id: String,
    pub owner_name: String,
    pub channel_id: String,
    pub text_channel_id: Option<String>,
    pub members_channel_id: Option<String>,
    pub queue_channel_id: Option<String>,
    pub category_id: Option<String>,
    pub channel_name: String,
    pub kind: String,
    pub visibility: String,
    pub queue_enabled: bool,
    pub locked: bool,
    pub stage_enabled: bool,
    pub member_limit: Option<i32>,
    pub status: Option<String>,
    pub channel_status: String,
    pub closed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceChannelCoAdmin {
    pub id: Uuid,
    pub voice_channel_id: Uuid,
    pub user_id: String,
    pub user_name: String,
    pub granted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceChannelWhitelistEntry {
    pub id: Uuid,
    pub guild_id: String,
    pub owner_id: String,
    pub target_id: String,
    pub target_name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceChannelBan {
    pub id: Uuid,
    pub voice_channel_id: Uuid,
    pub user_id: String,
    pub user_name: String,
    pub banned_by: String,
    pub reason: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceChannelInviteLink {
    pub id: Uuid,
    pub voice_channel_id: Uuid,
    pub guild_id: String,
    pub channel_id: String,
    pub created_by: String,
    pub created_by_name: String,
    pub code: String,
    pub max_uses: Option<i32>,
    pub current_uses: i32,
    pub expires_at: DateTime<Utc>,
    pub revoked: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceChannelTheme {
    pub id: Uuid,
    pub guild_id: String,
    pub name: String,
    pub emoji: Option<String>,
    pub channel_name_template: String,
    pub member_limit: Option<i32>,
    pub visibility: String,
    pub locked: bool,
    pub queue_enabled: bool,
    pub bitrate: Option<i32>,
    pub slowmode_secs: Option<i32>,
    pub stage_enabled: bool,
    pub is_default: bool,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceChannelDetail {
    pub channel: VoiceChannel,
    pub co_admins: Vec<VoiceChannelCoAdmin>,
    pub bans: Vec<VoiceChannelBan>,
    pub invite_links: Vec<VoiceChannelInviteLink>,
}
