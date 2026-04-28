//! Mapping entite metier <-> message Discord (cf. migration 175 +
//! SYNC_DISCORD_WEB_DESIGN.md).

use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscordActionMessage {
    pub action_id: Uuid,
    pub kind: String,
    pub guild_id: String,
    pub channel_id: String,
    pub message_id: String,
    pub posted_at: DateTime<Utc>,
    pub last_edited_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct NewDiscordActionMessage {
    pub action_id: Uuid,
    pub kind: String,
    pub guild_id: String,
    pub channel_id: String,
    pub message_id: String,
}

/// Conventions de `kind` reconnues — non exhaustif, le champ reste libre
/// pour faciliter l'ajout de nouvelles features sans toucher au domaine.
pub mod kinds {
    pub const BAN_PROPOSAL: &str = "ban_proposal";
    pub const TICKET: &str = "ticket";
    pub const ROLES_PANEL: &str = "roles_panel";
    pub const COMBAT_CHALLENGE: &str = "combat_challenge";
    pub const REVIEW_REQUEST: &str = "review_request";
}
