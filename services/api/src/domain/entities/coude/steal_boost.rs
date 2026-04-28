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
///
/// **Choix d'architecture** : cf note sur `STEAL_PROTECTION_ITEMS` —
/// prix hardcodes, grille validee a la conception. Modifier ici puis
/// redeployer.
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
#[path = "tests/steal_boost.rs"]
mod tests;
