//! TOUT-OU-RIEN (cf. COUPE_AMELIORATIONS section 6.1).
//!
//! Mecanique simple : 1× par semaine, le joueur mise tout son wallet.
//! 50/50 — pile, il double ; face, il perd 80%.
//!
//! Logique purement domaine. Le draw RNG est fait par le service.

/// Probabilite de gagner (50/50 strict).
pub const TOUT_OU_RIEN_WIN_PROBABILITY: f64 = 0.5;

/// Multiplicateur applique au wallet en cas de victoire (x2 = double).
pub const TOUT_OU_RIEN_WIN_MULTIPLIER: f64 = 2.0;

/// Pourcentage du wallet conserve en cas de defaite (0.20 = 20% restent).
pub const TOUT_OU_RIEN_LOSS_KEEP_PCT: f64 = 0.20;

/// Cooldown entre deux tentatives (7 jours = 604_800 secondes).
pub const TOUT_OU_RIEN_COOLDOWN_SECS: i64 = 7 * 24 * 3600;

/// Cle de cooldown stockee dans `coude_cooldowns` cote API.
pub const TOUT_OU_RIEN_COOLDOWN_KEY: &str = "tout_ou_rien";

/// Resultat d une tentative.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToutOuRienOutcome {
    Win,
    Lose,
}

/// Calcule le delta de coins (positif = gain, negatif = perte) pour un
/// joueur qui mise `balance` et obtient `outcome`.
///
/// Win  : delta = +balance (le solde double)
/// Lose : delta = -(0.8 * balance) (le joueur garde 20%)
pub fn coin_delta(balance: i64, outcome: ToutOuRienOutcome) -> i64 {
    if balance <= 0 {
        return 0;
    }
    match outcome {
        ToutOuRienOutcome::Win => balance,
        ToutOuRienOutcome::Lose => -((balance as f64 * (1.0 - TOUT_OU_RIEN_LOSS_KEEP_PCT)) as i64),
    }
}

/// Resoud l outcome a partir d un tirage uniforme dans [0, 1).
pub fn resolve_outcome(roll: f64) -> ToutOuRienOutcome {
    if roll < TOUT_OU_RIEN_WIN_PROBABILITY {
        ToutOuRienOutcome::Win
    } else {
        ToutOuRienOutcome::Lose
    }
}

#[cfg(test)]
#[path = "tests/tout_ou_rien.rs"]
mod tests;
