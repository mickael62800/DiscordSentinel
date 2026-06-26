//! Infos minimales necessaires pour annuler (reverser) une action de
//! moderation : derivees d'une ligne `audit_logs` (event_type `mod_*`).

/// `action_type` est deja stripe du prefixe `mod_` (ex: `ban_permanent`).
#[derive(Debug, Clone)]
pub struct ActionReversalInfo {
    pub guild_id: String,
    pub target_id: String,
    pub target_name: String,
    pub action_type: String,
}
