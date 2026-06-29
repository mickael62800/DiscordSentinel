use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;

use crate::domain::entities::moderation::detection_flags::DetectionFlags;
use crate::domain::entities::system::discord_ids::ChannelId;
use crate::domain::entities::system::discord_ids::GuildId;
use crate::domain::entities::system::discord_ids::MessageId;
use crate::domain::entities::system::discord_ids::UserId;
use crate::domain::enums::moderation::action::Action;
#[derive(Debug, Clone)]
pub struct Infraction {
    pub id: Uuid,
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub user_id: UserId,
    pub username: String,
    /// Pseudo serveur (nickname) si l user en a un. Lu via LEFT JOIN
    /// `guild_members.display_name`. Optionnel : null si l user n'est plus
    /// dans la guild ou n'a pas de nickname.
    pub display_name: Option<String>,
    pub message_id: MessageId,
    pub content: String,
    pub flags: DetectionFlags,
    pub score: f64,
    pub action: Action,
    pub reason: String,
    pub duration: Option<u64>,
    pub created_at: DateTime<Utc>,
}
