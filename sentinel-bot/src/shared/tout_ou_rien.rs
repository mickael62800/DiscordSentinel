//! Constantes TOUT-OU-RIEN partagees (cf. COUPE_AMELIORATIONS 6.1).
//!
//! Mirror simplifie du domaine API (sentinel-api/src/domain/entities/
//! tout_ou_rien.rs). Sync manuel — toute valeur change ici doit etre
//! repercutee cote API.

pub const TOUT_OU_RIEN_COOLDOWN_KEY: &str = "tout_ou_rien";
pub const TOUT_OU_RIEN_COOLDOWN_SECS: i64 = 7 * 24 * 3600;
pub const TOUT_OU_RIEN_WIN_PROBABILITY: f64 = 0.5;
pub const TOUT_OU_RIEN_LOSS_KEEP_PCT: f64 = 0.20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToutOuRienOutcome {
    Win,
    Lose,
}

/// Calcule le delta de coins (positif = gain, negatif = perte).
pub fn coin_delta(balance: i64, outcome: ToutOuRienOutcome) -> i64 {
    if balance <= 0 {
        return 0;
    }
    match outcome {
        ToutOuRienOutcome::Win => balance,
        ToutOuRienOutcome::Lose => {
            -((balance as f64 * (1.0 - TOUT_OU_RIEN_LOSS_KEEP_PCT)) as i64)
        }
    }
}

pub fn resolve_outcome(roll: f64) -> ToutOuRienOutcome {
    if roll < TOUT_OU_RIEN_WIN_PROBABILITY {
        ToutOuRienOutcome::Win
    } else {
        ToutOuRienOutcome::Lose
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn win_doubles() {
        assert_eq!(coin_delta(1000, ToutOuRienOutcome::Win), 1000);
    }

    #[test]
    fn lose_keeps_20_percent() {
        assert_eq!(coin_delta(1000, ToutOuRienOutcome::Lose), -800);
    }

    #[test]
    fn resolve_outcome_threshold() {
        assert_eq!(resolve_outcome(0.0), ToutOuRienOutcome::Win);
        assert_eq!(resolve_outcome(0.5), ToutOuRienOutcome::Lose);
    }
}
