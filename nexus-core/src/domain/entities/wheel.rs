//! Domaine pur de la Roue du Destin (repris de l'ancien module Sentinel).
//!
//! 10 cases ponderees, chaque case a un effet coins (positif, negatif ou
//! neutre). RNG injectee via `spin_with_rng(rng)` -> testable/seedable.
//! Probabilites et payouts REPRIS A L'IDENTIQUE de l'ancien
//! `sentinel-core/src/domain/entities/casino/wheel.rs` (commit ff6e8a46^).

use chrono::DateTime;
use chrono::Utc;
use rand::distributions::WeightedIndex;
use rand::prelude::Distribution;
use rand::RngCore;
use uuid::Uuid;

/// Une case de la roue : identifiant stable, libelle affiche, payout fixe
/// en coins, et poids RNG (plus eleve = sort plus souvent).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WheelCase {
    pub key: &'static str,
    pub label: &'static str,
    /// Payout en coins. Negatif = perte. 0 = neutre (la case "blanche").
    pub payout: i64,
    pub weight: u32,
}

/// Les 10 cases historiques — coins-only. Somme des poids = 100 :
/// - 60% : petits gains / pertes (ambiance, presque neutres)
/// - 30% : gains moyens
/// - 9%  : gros gains ou grosses pertes
/// - 1%  : LICORNE jackpot rare
pub const WHEEL_CASES: &[WheelCase] = &[
    WheelCase {
        key: "blanche",
        label: "🌀 Blanche — Rien. Du tout.",
        payout: 0,
        weight: 25,
    },
    WheelCase {
        key: "pq",
        label: "🧻 PQ — +50c (collection)",
        payout: 50,
        weight: 20,
    },
    WheelCase {
        key: "sieste",
        label: "💤 Sieste — +200c",
        payout: 200,
        weight: 15,
    },
    WheelCase {
        key: "colis",
        label: "📦 Colis — +500c",
        payout: 500,
        weight: 12,
    },
    WheelCase {
        key: "trefle",
        label: "🍀 Trefle — +1000c",
        payout: 1000,
        weight: 10,
    },
    WheelCase {
        key: "couronne",
        label: "👑 Couronne — +1500c (Roi du jour)",
        payout: 1500,
        weight: 7,
    },
    WheelCase {
        key: "ruine",
        label: "💀 Ruine — -500c",
        payout: -500,
        weight: 5,
    },
    WheelCase {
        key: "jackpot",
        label: "🎰 Jackpot — +5000c",
        payout: 5000,
        weight: 3,
    },
    WheelCase {
        key: "bombe",
        label: "💣 Bombe — -2000c (apocalypse)",
        payout: -2000,
        weight: 2,
    },
    WheelCase {
        key: "licorne",
        label: "🦄 LICORNE — +10000c",
        payout: 10000,
        weight: 1,
    },
];

/// Resultat d'un spin (pas encore persiste).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WheelOutcome {
    pub case_index: usize,
    pub case: WheelCase,
}

/// Spin de la roue. RNG injectee -> seedable pour les tests.
/// Ne panique jamais : les poids constants sont non-nuls.
pub fn spin_with_rng(rng: &mut impl RngCore) -> WheelOutcome {
    let weights: Vec<u32> = WHEEL_CASES.iter().map(|c| c.weight).collect();
    let dist = WeightedIndex::new(&weights).expect("poids constants valides");
    let idx = dist.sample(rng);
    WheelOutcome {
        case_index: idx,
        case: WHEEL_CASES[idx].clone(),
    }
}

/// True si la case est "memorable" (jackpot, licorne, bombe) -> mise en
/// avant dans l'embed de resultat.
pub fn is_memorable_case(key: &str) -> bool {
    matches!(key, "jackpot" | "licorne" | "bombe")
}

/// Entree persistee dans `nexus_wheel_spin_log`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WheelSpin {
    pub id: Uuid,
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub case_key: String,
    pub case_label: String,
    pub payout: i64,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
#[path = "tests/wheel.rs"]
mod tests;
