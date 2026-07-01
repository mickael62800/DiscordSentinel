//! Domaine pur de la Roue du Destin.
//!
//! 10 cases ponderees, chaque case a un effet coins (positif, negatif, ou
//! neutre). RNG seedable via `spin_with_rng(rng)` -> testable.

use crate::domain::entities::system::discord_ids::GuildId;
use crate::domain::entities::system::discord_ids::UserId;
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

/// Borne de securite sur le payout d une case (±50000 coins). Une case mal
/// configuree ne doit pas pouvoir ruiner / faire exploser l economie.
pub const WHEEL_PAYOUT_CLAMP: i64 = 50_000;

/// Payout + poids d une case, editables par serveur. Les `key`/`label`
/// restent figes dans `WHEEL_CASES` (l ordre/index fait foi).
#[derive(Debug, Clone, PartialEq)]
pub struct WheelSegment {
    pub payout: i64,
    pub weight: u32,
}

/// Config par serveur de la Roue : payout + poids de chacune des 10 cases.
///
/// Le domaine reste PUR : cette structure est passee EN ENTREE (as data) ;
/// le domaine ne lit jamais la config du serveur. La couche application
/// construit un `WheelConfig` depuis la config `wheel-bot` puis le fournit.
///
/// `Default` reproduit EXACTEMENT `WHEEL_CASES` -> comportement inchange tant
/// qu aucune surcharge n est configuree.
#[derive(Debug, Clone, PartialEq)]
pub struct WheelConfig {
    /// Aligne par index avec `WHEEL_CASES` (10 elements).
    pub segments: Vec<WheelSegment>,
}

impl Default for WheelConfig {
    fn default() -> Self {
        Self {
            segments: WHEEL_CASES
                .iter()
                .map(|c| WheelSegment {
                    payout: c.payout,
                    weight: c.weight,
                })
                .collect(),
        }
    }
}

impl WheelConfig {
    /// Applique les garde-fous metier :
    /// - si la longueur ne correspond pas a `WHEEL_CASES`, on repart des defauts ;
    /// - payouts clampes a ±`WHEEL_PAYOUT_CLAMP` ;
    /// - si la somme des poids est nulle (roue injouable / panic WeightedIndex),
    ///   on restaure les poids par defaut.
    #[must_use]
    pub fn normalized(mut self) -> Self {
        if self.segments.len() != WHEEL_CASES.len() {
            return Self::default();
        }
        for seg in &mut self.segments {
            seg.payout = seg.payout.clamp(-WHEEL_PAYOUT_CLAMP, WHEEL_PAYOUT_CLAMP);
        }
        if self.segments.iter().map(|s| s.weight).sum::<u32>() == 0 {
            for (seg, case) in self.segments.iter_mut().zip(WHEEL_CASES) {
                seg.weight = case.weight;
            }
        }
        self
    }
}

/// Resultat d un spin (pas encore persiste).
#[derive(Debug, Clone, PartialEq)]
pub struct WheelOutcome {
    pub case_index: usize,
    pub case: WheelCase,
}

/// Spin de la roue avec les cases par defaut (payouts/poids historiques).
/// RNG injectee -> seedable pour les tests.
pub fn spin_with_rng(rng: &mut impl RngCore) -> WheelOutcome {
    spin_with_rng_curses(rng, false)
}

/// Variante avec malediction "Heartbreak" (cf. COUPE_AMELIORATIONS 5.1) :
/// si `block_licorne` est `true`, la case licorne est exclue du tirage.
/// Utilise les cases par defaut.
pub fn spin_with_rng_curses(rng: &mut impl RngCore, block_licorne: bool) -> WheelOutcome {
    spin_with_rng_curses_cfg(rng, block_licorne, &WheelConfig::default())
}

/// Spin parametrique : `config` fournit payout + poids de chaque case
/// (editables par serveur). Sans malediction.
pub fn spin_with_rng_cfg(rng: &mut impl RngCore, config: &WheelConfig) -> WheelOutcome {
    spin_with_rng_curses_cfg(rng, false, config)
}

/// Spin parametrique complet : `config` (payout/poids par case) + malediction
/// "Heartbreak". Ne peut pas paniquer : `config` est normalise (somme des
/// poids > 0) et si le blocage licorne annule tous les poids, on ignore le
/// blocage plutot que de paniquer.
pub fn spin_with_rng_curses_cfg(
    rng: &mut impl RngCore,
    block_licorne: bool,
    config: &WheelConfig,
) -> WheelOutcome {
    let config = config.clone().normalized();
    let mut weights: Vec<u32> = WHEEL_CASES
        .iter()
        .zip(&config.segments)
        .map(|(c, seg)| {
            if block_licorne && c.key == "licorne" {
                0
            } else {
                seg.weight
            }
        })
        .collect();
    if weights.iter().sum::<u32>() == 0 {
        // Le blocage licorne a annule tous les poids (config degeneree) :
        // on ignore le blocage pour ne pas paniquer.
        weights = config.segments.iter().map(|s| s.weight).collect();
    }
    let dist =
        WeightedIndex::new(&weights).expect("WheelConfig normalise doit avoir des poids > 0");
    let idx = dist.sample(rng);
    let case = WheelCase {
        key: WHEEL_CASES[idx].key,
        label: WHEEL_CASES[idx].label,
        payout: config.segments[idx].payout,
        weight: config.segments[idx].weight,
    };
    WheelOutcome {
        case_index: idx,
        case,
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
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub username: String,
    pub case_key: String,
    pub case_label: String,
    pub payout: i64,
    pub created_at: DateTime<Utc>,
}

/// Top winner cumule sur N derniers jours pour le leaderboard.
#[derive(Debug, Clone)]
pub struct WheelTopWinner {
    pub user_id: UserId,
    pub username: String,
    pub total_payout: i64,
    pub spin_count: u32,
}

#[cfg(test)]
#[path = "tests/wheel.rs"]
mod tests;
