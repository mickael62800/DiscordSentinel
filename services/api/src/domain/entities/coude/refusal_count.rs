//! Compteur de refus par paire (cf. COUPE_AMELIORATIONS 5.3 — Dette d honneur).

use chrono::{DateTime, Utc};

/// Seuil au-dessus duquel le requester peut invoquer la dette d honneur.
pub const HONOR_DEBT_THRESHOLD: i32 = 3;

#[derive(Debug, Clone)]
pub struct RefusalCount {
    pub guild_id: String,
    pub requester_id: String,
    pub refuser_id: String,
    pub count: i32,
    pub last_refused_at: DateTime<Utc>,
}

impl RefusalCount {
    pub fn honor_debt_owed(&self) -> bool {
        self.count >= HONOR_DEBT_THRESHOLD
    }
}

#[cfg(test)]
#[path = "tests/refusal_count.rs"]
mod tests;
