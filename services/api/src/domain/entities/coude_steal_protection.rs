//! Entite pour les abonnements anti-vol (Phase 9 Part B).
//!
//! Remplace le modele inventory-based (items consommes) par des
//! abonnements sur duree. Chaque item anti-vol a sa propre proba de
//! blocage, et une tentative de vol fait rouler tous les items actifs
//! de la cible dans l'ordre d'efficacite.

use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CoudeStealProtection {
    pub id: Uuid,
    pub guild_id: String,
    pub user_id: String,
    pub item_key: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Duree d'abonnement d'une protection. Le prix effectif est calcule a
/// partir du `base_cost_per_day` de l'item multiplie par le facteur
/// (avec remise degressive pour inciter aux durees longues).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StealProtectionDuration {
    OneDay,
    ThreeDays,
    FiveDays,
    SevenDays,
}

impl StealProtectionDuration {
    pub fn days(self) -> i64 {
        match self {
            Self::OneDay => 1,
            Self::ThreeDays => 3,
            Self::FiveDays => 5,
            Self::SevenDays => 7,
        }
    }

    /// Facteur multiplicatif par rapport au base_cost_per_day.
    /// - 1j : 1x (pas de remise)
    /// - 3j : 2.7x (10% de remise sur 3j)
    /// - 5j : 4.25x (15% de remise sur 5j)
    /// - 7j : 5.6x (20% de remise sur 7j)
    pub fn cost_multiplier(self) -> f64 {
        match self {
            Self::OneDay => 1.0,
            Self::ThreeDays => 2.7,
            Self::FiveDays => 4.25,
            Self::SevenDays => 5.6,
        }
    }

    /// Calcule le prix total de l'abonnement pour un item donne.
    pub fn total_cost(self, base_cost_per_day: i64) -> i64 {
        ((base_cost_per_day as f64) * self.cost_multiplier()).round() as i64
    }

    pub fn as_key(self) -> &'static str {
        match self {
            Self::OneDay => "1d",
            Self::ThreeDays => "3d",
            Self::FiveDays => "5d",
            Self::SevenDays => "7d",
        }
    }

    pub fn from_key(s: &str) -> Option<Self> {
        match s {
            "1d" => Some(Self::OneDay),
            "3d" => Some(Self::ThreeDays),
            "5d" => Some(Self::FiveDays),
            "7d" => Some(Self::SevenDays),
            _ => None,
        }
    }
}

/// Definition statique d'un item de protection vol. Les prix sont
/// configurables par guild (Phase 9 Part E) mais les caracteristiques
/// (key, name, block_chance) vivent dans le catalog domain.
#[derive(Debug, Clone)]
pub struct StealProtectionItemDef {
    pub key: &'static str,
    pub name: &'static str,
    pub emoji: &'static str,
    pub description: &'static str,
    pub block_chance_percent: u32,
    pub base_cost_per_day: i64,
}

/// Catalogue des 8 items de protection disponibles, tries par efficacite
/// croissante. Le shop bot affiche cette liste ; le moteur de vol iter
/// en ordre decroissant pour que les meilleurs items rollent en premier.
pub const STEAL_PROTECTION_ITEMS: &[StealProtectionItemDef] = &[
    StealProtectionItemDef {
        key: "chien_garde",
        name: "Chien de garde",
        emoji: "\u{1f415}",
        description: "25% de chance de bloquer un vol. Le classique fidele.",
        block_chance_percent: 25,
        base_cost_per_day: 50,
    },
    StealProtectionItemDef {
        key: "alarme_sonore",
        name: "Alarme sonore",
        emoji: "\u{1f514}",
        description: "30% de chance de bloquer un vol. Reveille tout le voisinage.",
        block_chance_percent: 30,
        base_cost_per_day: 80,
    },
    StealProtectionItemDef {
        key: "piege_a_loup",
        name: "Piege a loup",
        emoji: "\u{1faa4}",
        description: "35% de chance de bloquer un vol. Ca fait mal.",
        block_chance_percent: 35,
        base_cost_per_day: 120,
    },
    StealProtectionItemDef {
        key: "camera_surveillance",
        name: "Camera de surveillance",
        emoji: "\u{1f4f9}",
        description: "40% de chance de bloquer un vol. Il sera demasque.",
        block_chance_percent: 40,
        base_cost_per_day: 160,
    },
    StealProtectionItemDef {
        key: "leurre_dore",
        name: "Leurre dore",
        emoji: "\u{1f36f}",
        description: "45% de chance de bloquer un vol. Le voleur prend le faux.",
        block_chance_percent: 45,
        base_cost_per_day: 220,
    },
    StealProtectionItemDef {
        key: "garde_du_corps",
        name: "Garde du corps",
        emoji: "\u{1f46e}",
        description: "50% de chance de bloquer un vol. Professionnel.",
        block_chance_percent: 50,
        base_cost_per_day: 300,
    },
    StealProtectionItemDef {
        key: "coffre_fort",
        name: "Coffre-fort",
        emoji: "\u{1f512}",
        description: "60% de chance de bloquer un vol. Inviolable ou presque.",
        block_chance_percent: 60,
        base_cost_per_day: 450,
    },
    StealProtectionItemDef {
        key: "forteresse",
        name: "Forteresse privee",
        emoji: "\u{1f3f0}",
        description: "70% de chance de bloquer un vol. Le luxe ultime.",
        block_chance_percent: 70,
        base_cost_per_day: 700,
    },
];

pub fn find_protection_item(key: &str) -> Option<&'static StealProtectionItemDef> {
    STEAL_PROTECTION_ITEMS.iter().find(|i| i.key == key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_cost_multipliers_decrease_per_day() {
        let base = 100_i64;
        let one = StealProtectionDuration::OneDay.total_cost(base);
        let three = StealProtectionDuration::ThreeDays.total_cost(base);
        let five = StealProtectionDuration::FiveDays.total_cost(base);
        let seven = StealProtectionDuration::SevenDays.total_cost(base);

        assert_eq!(one, 100);
        assert_eq!(three, 270);
        assert_eq!(five, 425);
        assert_eq!(seven, 560);

        // Cost per day must strictly decrease
        let per_day_one = one as f64 / 1.0;
        let per_day_three = three as f64 / 3.0;
        let per_day_five = five as f64 / 5.0;
        let per_day_seven = seven as f64 / 7.0;
        assert!(per_day_one > per_day_three);
        assert!(per_day_three > per_day_five);
        assert!(per_day_five > per_day_seven);
    }

    #[test]
    fn duration_round_trip() {
        for d in [
            StealProtectionDuration::OneDay,
            StealProtectionDuration::ThreeDays,
            StealProtectionDuration::FiveDays,
            StealProtectionDuration::SevenDays,
        ] {
            assert_eq!(StealProtectionDuration::from_key(d.as_key()), Some(d));
        }
    }

    #[test]
    fn catalog_sorted_by_block_chance_ascending() {
        // Le shop affiche du moins cher au plus cher, donc block_chance ascending.
        for pair in STEAL_PROTECTION_ITEMS.windows(2) {
            assert!(
                pair[0].block_chance_percent <= pair[1].block_chance_percent,
                "catalogue non trie par block_chance croissant"
            );
        }
    }

    #[test]
    fn find_protection_item_works() {
        assert!(find_protection_item("chien_garde").is_some());
        assert!(find_protection_item("coffre_fort").is_some());
        assert!(find_protection_item("forteresse").is_some());
        assert!(find_protection_item("unknown").is_none());
    }
}
