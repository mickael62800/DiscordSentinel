//! Codes d'invitation a usage unique (onboarding de nouveaux users).

use chrono::DateTime;
use chrono::Utc;

/// Roles valides pour une invitation.
pub const VALID_INVITATION_ROLES: &[&str] = &["viewer", "moderator", "admin", "owner"];

/// Une invitation telle que persistee (table `invitation_codes`).
#[derive(Debug, Clone)]
pub struct Invitation {
    pub code: String,
    pub guild_id: String,
    pub role: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub used_at: Option<DateTime<Utc>>,
    pub used_by_discord_id: Option<String>,
    pub notes: Option<String>,
}

impl Invitation {
    /// Statut derive a l'instant `now` : "used" | "expired" | "active".
    pub fn status(&self, now: DateTime<Utc>) -> &'static str {
        if self.used_at.is_some() {
            "used"
        } else if self.expires_at.map(|e| e < now).unwrap_or(false) {
            "expired"
        } else {
            "active"
        }
    }
}

/// Resultat de `check_access` : autorisation d'acces au dashboard.
#[derive(Debug, Clone)]
pub struct AccessStatus {
    pub is_authorized: bool,
    pub is_superadmin: bool,
    /// Nombre de guilds pour lesquelles l'utilisateur a un role.
    pub guild_count: i64,
    pub message: String,
}

/// Resultat d'un redeem reussi : la guild et le role octroyes.
#[derive(Debug, Clone)]
pub struct RedeemedInvitation {
    pub guild_id: String,
    pub role: String,
}
