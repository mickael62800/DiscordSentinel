//! Compteur de refus par paire (cf. COUPE_AMELIORATIONS 5.3 — Dette d honneur).

use crate::domain::entities::system::discord_ids::GuildId;
use chrono::DateTime;
use chrono::Utc;
/// Seuil au-dessus duquel le requester peut invoquer la dette d honneur.
pub const HONOR_DEBT_THRESHOLD: i32 = 3;

#[derive(Debug, Clone)]
pub struct RefusalCount {
    pub guild_id: GuildId,
    pub requester_id: String,
    pub refuser_id: String,
    pub count: i32,
    pub last_refused_at: DateTime<Utc>,
}

impl RefusalCount {
    /// Dette d honneur due si le compteur atteint le `threshold` configure.
    /// Le seuil est passe en donnee (domaine pur) — cf.
    /// `CoudeEconomyConfig::honor_debt_threshold`. Le defaut historique
    /// reste `HONOR_DEBT_THRESHOLD`.
    pub fn honor_debt_owed(&self, threshold: i32) -> bool {
        self.count >= threshold
    }
}

#[cfg(test)]
#[path = "tests/refusal_count.rs"]
mod tests;
