use crate::domain::entities::system::discord_ids::GuildId;
use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AuditLog {
    pub id: Uuid,
    pub guild_id: GuildId,
    pub event_type: String,
    pub actor_id: Option<String>,
    pub actor_name: Option<String>,
    pub target_id: Option<String>,
    pub target_name: Option<String>,
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    pub details: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Event type pour l'historique des changements de nickname d'un membre.
/// Regle metier : identifiant stable consomme par le desktop et les exports.
pub const AUDIT_EVENT_MEMBER_NICKNAME_HISTORY: &str = "member_nickname_history";

/// Prefixe commun aux events de securite (auto-detection, raid, alt, etc.).
/// Utilise par le handler security::purge_events pour cibler le DELETE.
pub const AUDIT_EVENT_SECURITY_PREFIX: &str = "security_";

/// Verifie qu'un event_type est un event de securite (commence par "security_").
pub fn is_security_audit_event(event_type: &str) -> bool {
    event_type.starts_with(AUDIT_EVENT_SECURITY_PREFIX)
}

#[cfg(test)]
#[path = "tests/audit_log.rs"]
mod tests;
