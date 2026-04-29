use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use crate::domain::enums::moderation::moderation_gravity::ModerationGravity;
use crate::domain::entities::system::discord_ids::ChannelId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationAction {
    pub id: Uuid,
    pub guild_id: String,
    pub channel_id: ChannelId,
    pub moderator_id: String,
    pub moderator_name: String,
    pub target_id: String,
    pub target_name: String,
    pub action_type: String,
    pub reason: String,
    pub gravity: Option<ModerationGravity>,
    pub duration: Option<u64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserModerationHistory {
    pub target_id: String,
    pub target_name: String,
    pub total_warns: u32,
    pub total_mutes: u32,
    pub total_bans: u32,
    pub actions: Vec<ModerationAction>,
}
