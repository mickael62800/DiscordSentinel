//! Domaine pur de la Roue du Destin.
//!
//! 10 cases ponderees, chaque case a un effet coins (positif, negatif, ou
//! neutre). RNG seedable via `spin_with_rng(rng)` -> testable.

use chrono::DateTime;
use chrono::Utc;
use rand::distributions::WeightedIndex;
use rand::prelude::Distribution;
use rand::RngCore;
use uuid::Uuid;

/// Une case de la roue : identifiant stable, libelle affiche, payout fixe
/// en coins, et poids RNG (plus eleve = sort plus souvent).
#[derive(Debug, Clone, PartialEq)]
pub struct WheelCase {
    pub key: &'static str,
    pub label: &'static str,
    /// Payout en coins. Negatif = perte. 0 = neutre (la case "blanche").
    pub payout: i64,
    pub weight: u32,
}

/// Les 10 cases v1 — coins-only. Effets cosmetiques (titre, mode hardcore,
/// etc.) viendront en v2.
///
/// Equilibre approximatif (somme des poids = 100) :
/// - 60% : petits gains / pertes (ambiance, presque neutres)
/// - 30% : gains moyens
/// - 9% : gros gains ou grosses pertes (les moments memes)
/// - 1% : LICORNE jackpot rare
pub const WHEEL_CASES: &[WheelCase] = &[
    WheelCase { key: "blanche",   label: "🌀 Blanche — Rien. Du tout.",                payout: 0,      weight: 25 },
    WheelCase { key: "pq",        label: "🧻 PQ — +50c (collection)",                  payout: 50,     weight: 20 },
    WheelCase { key: "sieste",    label: "💤 Sieste — +200c",                          payout: 200,    weight: 15 },
    WheelCase { key: "colis",     label: "📦 Colis — +500c",                           payout: 500,    weight: 12 },
    WheelCase { key: "trefle",    label: "🍀 Trefle — +1000c",                         payout: 1000,   weight: 10 },
    WheelCase { key: "couronne",  label: "👑 Couronne — +1500c (Roi du jour)",         payout: 1500,   weight: 7 },
    WheelCase { key: "ruine",     label: "💀 Ruine — -500c",                           payout: -500,   weight: 5 },
    WheelCase { key: "jackpot",   label: "🎰 Jackpot — +5000c",                        payout: 5000,   weight: 3 },
    WheelCase { key: "bombe",     label: "💣 Bombe — -2000c (apocalypse)",             payout: -2000,  weight: 2 },
    WheelCase { key: "licorne",   label: "🦄 LICORNE — +10000c",                       payout: 10000,  weight: 1 },
];

/// Resultat d un spin (pas encore persiste).
#[derive(Debug, Clone, PartialEq)]
pub struct WheelOutcome {
    pub case_index: usize,
    pub case: WheelCase,
}

/// Spin de la roue. RNG injectee -> seedable pour les tests.
/// Ne peut pas paniquer car WHEEL_CASES n est pas vide et au moins un poids > 0.
pub fn spin_with_rng(rng: &mut impl RngCore) -> WheelOutcome {
    spin_with_rng_curses(rng, false)
}

/// Variante avec malediction "Heartbreak" (cf. COUPE_AMELIORATIONS 5.1) :
/// si `block_licorne` est `true`, la case licorne (poids 1) est exclue du
/// tirage. Utilise quand le spinner est sous l effet "Malchance amoureuse".
pub fn spin_with_rng_curses(rng: &mut impl RngCore, block_licorne: bool) -> WheelOutcome {
    let weights: Vec<u32> = WHEEL_CASES
        .iter()
        .map(|c| {
            if block_licorne && c.key == "licorne" {
                0
            } else {
                c.weight
            }
        })
        .collect();
    let dist = WeightedIndex::new(&weights).expect("WHEEL_CASES doit avoir des poids valides");
    let idx = dist.sample(rng);
    WheelOutcome {
        case_index: idx,
        case: WHEEL_CASES[idx].clone(),
    }
}

/// Retourne true si la case est "memorable" (jackpot, licorne, bombe) -> a
/// broadcaster aussi dans le log_channel_id si configure.
pub fn is_memorable_case(key: &str) -> bool {
    matches!(key, "jackpot" | "licorne" | "bombe")
}

/// Entree persistee dans `wheel_spin_log`.
#[derive(Debug, Clone)]
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

/// Top winner cumule sur N derniers jours pour le leaderboard.
#[derive(Debug, Clone)]
pub struct WheelTopWinner {
    pub user_id: String,
    pub username: String,
    pub total_payout: i64,
    pub spin_count: u32,
}

#[cfg(test)]
#[path = "tests/wheel.rs"]
mod tests;
