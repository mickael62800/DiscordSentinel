//! Bareme des succes Coup de Coude (cf. COUPE_AMELIORATIONS 3.4).
//!
//! Regle metier PURE : a partir de l'etat courant d'un joueur, on derive
//! la liste des succes debloques. Le bareme (seuils) vit ici, cote serveur,
//! et non plus dans le bot (qui ne fait que l'affichage des emojis/labels).
//!
//! Aucun avantage gameplay : ces succes sont purement cosmetiques.

use crate::domain::entities::coude::player::Player;

/// Clefs stables des 30 succes, dans l'ordre d'affichage suggere
/// (combats -> lachete/chaos -> vol -> economie -> casino -> niveau -> stats).
pub const ACHIEVEMENT_KEYS: &[&str] = &[
    // ── Combat ──
    "first_blood",
    "veteran",
    "butcher",
    "legend",
    "punching_ball",
    "tapis",
    "diplomat",
    "no_quarter",
    // ── Lachete / Chaos ──
    "coward_obvious",
    "coward_notorious",
    "chaos_king",
    "chaos_master",
    // ── Vol ──
    "first_heist",
    "pickpocket",
    "pro_thief",
    "bank_robber",
    // ── Economie ──
    "rich",
    "millionaire",
    "magnate",
    "investor",
    "bankrupt",
    // ── Casino ──
    "casino_addict",
    "lucky",
    "casino_cursed",
    // ── Niveau ──
    "apprentice",
    "veteran_play",
    "guardian",
    "ascetic",
    "master",
    // ── Stats ──
    "tank",
    "brute",
    "specialist",
];

/// Nombre total de succes disponibles.
pub fn total_achievements() -> usize {
    ACHIEVEMENT_KEYS.len()
}

/// Retourne `true` si le joueur a debloque le succes `key`.
///
/// Bareme reproduit a l'identique de l'ancien `is_unlocked` du bot.
pub fn is_unlocked(key: &str, p: &Player) -> bool {
    match key {
        // Combat
        "first_blood" => p.total_wins >= 1,
        "veteran" => p.total_wins >= 10,
        "butcher" => p.total_wins >= 50,
        "legend" => p.total_wins >= 100,
        "punching_ball" => p.total_losses >= 10,
        "tapis" => p.total_losses >= 50,
        "diplomat" => p.total_draws >= 20,
        "no_quarter" => p.total_wins >= 20 && p.total_draws == 0,
        // Lachete / Chaos
        "coward_obvious" => p.cowardice_count >= 5,
        "coward_notorious" => p.cowardice_count >= 20,
        "chaos_king" => p.chaos_events >= 10,
        "chaos_master" => p.chaos_events >= 50,
        // Vol
        "first_heist" => p.total_stolen >= 1,
        "pickpocket" => p.total_stolen >= 1_000,
        "pro_thief" => p.total_stolen >= 10_000,
        "bank_robber" => p.total_stolen >= 100_000,
        // Economie
        "rich" => p.coins >= 10_000,
        "millionaire" => p.coins >= 100_000,
        "magnate" => p.coins >= 1_000_000,
        "investor" => p.total_earned >= 200_000,
        "bankrupt" => p.total_lost >= 100_000,
        // Casino
        "casino_addict" => (p.casino_wins + p.casino_losses) >= 10,
        "lucky" => p.casino_wins >= 20,
        "casino_cursed" => p.casino_losses >= 20,
        // Niveau
        "apprentice" => p.level >= 5,
        "veteran_play" => p.level >= 10,
        "guardian" => p.level >= 15,
        "ascetic" => p.level >= 20,
        "master" => p.level >= 25,
        // Stats
        "tank" => p.def >= 50,
        "brute" => p.atk >= 50,
        "specialist" => p.class.is_some(),
        _ => false,
    }
}

/// Liste des clefs de succes debloquees par le joueur (ordre du catalogue).
pub fn unlocked_keys(p: &Player) -> Vec<&'static str> {
    ACHIEVEMENT_KEYS
        .iter()
        .filter(|k| is_unlocked(k, p))
        .copied()
        .collect()
}
