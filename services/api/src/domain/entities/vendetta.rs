//! Vendetta (cf. COUPE_AMELIORATIONS section 5.3).
//!
//! Premier sous-module : declaration + resolution. Apres avoir perdu un
//! combat contre X, le challenger peut declarer une vendetta. Dans les
//! 7 jours :
//!   - revanche gagnee -> +100% sur la mise standard
//!   - revanche reperdue -> X est marque "Bourreau" pour 7 jours
//!
//! La logique pure ici (constantes + helpers d ajustement payout). La
//! persistance vit dans `coude_vendettas` et le service correspondant.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Duree de la fenetre vendetta (heures, 7 jours).
pub const VENDETTA_WINDOW_HOURS: i64 = 7 * 24;

/// Multiplicateur applique au gain en cas de revanche victorieuse.
pub const VENDETTA_WIN_BONUS_MULTIPLIER: f64 = 2.0;

/// Etat d une vendetta enregistree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VendettaStatus {
    Active,
    Won,
    Lost,
    Expired,
}

impl VendettaStatus {
    pub fn as_db_str(self) -> &'static str {
        match self {
            VendettaStatus::Active => "active",
            VendettaStatus::Won => "won",
            VendettaStatus::Lost => "lost",
            VendettaStatus::Expired => "expired",
        }
    }
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "active" => Some(VendettaStatus::Active),
            "won" => Some(VendettaStatus::Won),
            "lost" => Some(VendettaStatus::Lost),
            "expired" => Some(VendettaStatus::Expired),
            _ => None,
        }
    }
}

/// Snapshot persistance.
#[derive(Debug, Clone)]
pub struct ActiveVendetta {
    pub id: Uuid,
    pub guild_id: String,
    pub challenger_id: String,
    pub target_id: String,
    pub declared_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub status: VendettaStatus,
    pub resolved_at: Option<DateTime<Utc>>,
}

impl ActiveVendetta {
    pub fn is_active_at(&self, now: DateTime<Utc>) -> bool {
        self.status == VendettaStatus::Active && self.expires_at > now
    }
}

/// Bonus de gain pour une revanche gagnee (multiplicateur applique au
/// nominal). Floor a la valeur nominale si pas de vendetta active.
pub fn apply_revenge_bonus(nominal_gain: i64, has_active_vendetta: bool) -> i64 {
    if !has_active_vendetta || nominal_gain <= 0 {
        return nominal_gain;
    }
    ((nominal_gain as f64) * VENDETTA_WIN_BONUS_MULTIPLIER) as i64
}

/// Suffixe applique au pseudo du gagnant initial s il bat a nouveau le
/// challenger (humiliation publique). Conforme au catalogue de suffix
/// utilise par taunts_dispatch.
pub const VENDETTA_BOURREAU_SUFFIX_PREFIX: &str = " le Bourreau de ";

#[cfg(test)]
#[path = "tests/vendetta.rs"]
mod tests;
