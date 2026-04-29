use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;

use crate::domain::enums::moderation::action::Action;
use crate::domain::entities::moderation::detection_flags::DetectionFlags;
use crate::domain::entities::system::discord_ids::MessageId;
use crate::domain::entities::system::discord_ids::ChannelId;
use crate::domain::entities::system::discord_ids::UserId;
use crate::domain::entities::system::discord_ids::GuildId;
#[derive(Debug, Clone)]
pub struct Infraction {
    pub id: Uuid,
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub user_id: UserId,
    pub username: String,
    pub message_id: MessageId,
    pub content: String,
    pub flags: DetectionFlags,
    pub score: f64,
    pub action: Action,
    pub reason: String,
    pub duration: Option<u64>,
    pub created_at: DateTime<Utc>,
}
