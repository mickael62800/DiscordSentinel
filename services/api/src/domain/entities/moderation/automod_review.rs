//! Carte de review automod persistee (cf. migration 176).
//!
//! Le bot poste un embed avec boutons (Apply / Warn / Mute / Ban / Ignore)
//! dans le salon de logs ; en parallele il INSERT une `AutomodReview` dans
//! cette table et register l'`action_id` dans `discord_action_messages`.
//! Du coup la web peut lister les reviews pending et resoudre depuis l UI ;
//! le bot edite la carte Discord en reaction (sync bilateral).

use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuggestedAction {
    Warn,
    Delete,
    Mute,
    Ban,
}

impl SuggestedAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Warn => "warn",
            Self::Delete => "delete",
            Self::Mute => "mute",
            Self::Ban => "ban",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "warn" => Some(Self::Warn),
            "delete" => Some(Self::Delete),
            "mute" => Some(Self::Mute),
            "ban" => Some(Self::Ban),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppliedAction {
    Warn,
    Delete,
    Mute,
    Ban,
    Ignore,
}

impl AppliedAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Warn => "warn",
            Self::Delete => "delete",
            Self::Mute => "mute",
            Self::Ban => "ban",
            Self::Ignore => "ignore",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "warn" => Some(Self::Warn),
            "delete" => Some(Self::Delete),
            "mute" => Some(Self::Mute),
            "ban" => Some(Self::Ban),
            "ignore" => Some(Self::Ignore),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AutomodReview {
    pub id: Uuid,
    pub guild_id: String,
    pub channel_id: String,
    pub message_id: String,
    pub user_id: String,
    pub user_name: String,
    pub content_preview: String,
    pub suggested_action: String,
    pub score: f64,
    pub reason: String,
    pub flags: serde_json::Value,
    pub status: String,
    pub applied_action: Option<String>,
    pub resolved_by_id: Option<String>,
    pub resolved_by_name: Option<String>,
    pub resolved_source: Option<String>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct NewAutomodReview {
    pub guild_id: String,
    pub channel_id: String,
    pub message_id: String,
    pub user_id: String,
    pub user_name: String,
    pub content_preview: String,
    pub suggested_action: SuggestedAction,
    pub score: f64,
    pub reason: String,
    pub flags: serde_json::Value,
}
