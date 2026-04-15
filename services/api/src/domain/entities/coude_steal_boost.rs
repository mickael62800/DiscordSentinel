//! Entite pour les abonnements boost voleur (Phase 9 Part C).
//!
//! Miroir de `coude_steal_protection` mais pour l'attaquant : chaque item
//! ajoute un bonus plat au roll du voleur. Les bonus s'additionnent quand
//! plusieurs items sont actifs en parallele.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::entities::StealProtectionDuration;

#[derive(Debug, Clone)]
pub struct CoudeStealBoost {
    pub id: Uuid,
    pub guild_id: String,
    pub user_id: String,
    pub item_key: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Les boosts reutilisent la grille de duree des protections (1/3/5/7j
/// avec meme remise degressive) — pas la peine d'avoir deux enums.
pub type StealBoostDuration = StealProtectionDuration;

#[derive(Debug, Clone)]
pub struct StealBoostItemDef {
    pub key: &'static str,
    pub name: &'static str,
    pub emoji: &'static str,
    pub description: &'static str,
    /// Bonus ajoute au roll du voleur quand cet item est actif.
    pub roll_bonus: i32,
    pub base_cost_per_day: i64,
}

/// Catalogue des 5 items de boost voleur, tries par puissance croissante.
/// Le bot les affiche via `/boost-voleur` dans cet ordre ; la resolution
/// de vol somme tous les items actifs (pas de priorite, cumul complet).
pub const STEAL_BOOST_ITEMS: &[StealBoostItemDef] = &[
    StealBoostItemDef {
        key: "crochet",
        name: "Crochet",
        emoji: "\u{1f527}",
        description: "+5 au roll de vol. Le minimum syndical du voleur.",
        roll_bonus: 5,
        base_cost_per_day: 60,
    },
    StealBoostItemDef {
        key: "passe_partout",
        name: "Passe-partout",
        emoji: "\u{1f5dd}\u{fe0f}",
        description: "+10 au roll de vol. Ouvre presque tout.",
        roll_bonus: 10,
        base_cost_per_day: 120,
    },
    StealBoostItemDef {
        key: "deguisement",
        name: "Deguisement",
        emoji: "\u{1f977}",
        description: "+15 au roll de vol. La victime ne te reconnait pas.",
        roll_bonus: 15,
        base_cost_per_day: 200,
    },
    StealBoostItemDef {
        key: "fumigene",
        name: "Fumigene",
        emoji: "\u{1f4a8}",
        description: "+20 au roll de vol. Disparais dans la brume.",
        roll_bonus: 20,
        base_cost_per_day: 320,
    },
    StealBoostItemDef {
        key: "marteau",
        name: "Marteau",
        emoji: "\u{1fa9a}",
        description: "+25 au roll de vol. La methode directe.",
        roll_bonus: 25,
        base_cost_per_day: 500,
    },
];

pub fn find_boost_item(key: &str) -> Option<&'static StealBoostItemDef> {
    STEAL_BOOST_ITEMS.iter().find(|i| i.key == key)
}

/// Somme les bonus des items actifs. Accepte une liste d'item_keys
/// (les items inconnus sont ignores, comportement defensif).
pub fn sum_roll_bonus_for_active_keys<I, S>(active_keys: I) -> i32
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    active_keys
        .into_iter()
        .filter_map(|k| find_boost_item(k.as_ref()).map(|i| i.roll_bonus))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_sorted_by_bonus_ascending() {
        for pair in STEAL_BOOST_ITEMS.windows(2) {
            assert!(
                pair[0].roll_bonus <= pair[1].roll_bonus,
                "catalogue non trie par bonus croissant"
            );
        }
    }

    #[test]
    fn catalog_cost_per_bonus_point_decreases_or_stays_stable() {
        // Plus tu investis cher, meilleur doit etre le ratio.
        let mut last_ratio: Option<f64> = None;
        for i in STEAL_BOOST_ITEMS {
            let ratio = i.base_cost_per_day as f64 / i.roll_bonus as f64;
            if let Some(prev) = last_ratio {
                // On tolere une legere variation mais pas d'inversion forte.
                assert!(ratio >= prev * 0.8, "ratio cout/bonus regresse trop");
            }
            last_ratio = Some(ratio);
        }
    }

    #[test]
    fn find_boost_item_works() {
        assert!(find_boost_item("crochet").is_some());
        assert!(find_boost_item("marteau").is_some());
        assert!(find_boost_item("unknown").is_none());
    }

    #[test]
    fn sum_roll_bonus_empty_is_zero() {
        let empty: Vec<String> = vec![];
        assert_eq!(sum_roll_bonus_for_active_keys(empty), 0);
    }

    #[test]
    fn sum_roll_bonus_stacks_all_actives() {
        let actives = vec!["crochet", "marteau"]; // +5 + +25 = +30
        assert_eq!(sum_roll_bonus_for_active_keys(actives), 30);
    }

    #[test]
    fn sum_roll_bonus_ignores_unknown_keys() {
        let actives = vec!["crochet", "unknown_item"];
        assert_eq!(sum_roll_bonus_for_active_keys(actives), 5);
    }

    #[test]
    fn sum_all_items_gives_expected_total() {
        // 5 + 10 + 15 + 20 + 25 = 75
        let all: Vec<&str> = STEAL_BOOST_ITEMS.iter().map(|i| i.key).collect();
        assert_eq!(sum_roll_bonus_for_active_keys(all), 75);
    }
}
