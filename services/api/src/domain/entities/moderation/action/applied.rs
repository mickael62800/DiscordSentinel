use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use crate::domain::enums::moderation::moderation_gravity::ModerationGravity;
use crate::domain::entities::system::discord_ids::ChannelId;
use crate::domain::entities::system::discord_ids::GuildId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationAction {
    pub id: Uuid,
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub moderator_id: String,
    pub moderator_name: String,
    pub target_id: String,
    pub target_name: String,
    /// Pseudo serveur (nickname) actuel de la cible si elle est encore dans
    /// la guild. Lu via LEFT JOIN guild_members.display_name. Optionnel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_display_name: Option<String>,
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
