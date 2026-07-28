//! Prédicats purs de planification/rétention (consommés par sentinel-worker).

use chrono::{DateTime, Duration, Utc};

/// Garde ANTI-PURGE-TOTALE : une retention <= 0 rend `NOW() - interval '0 day'`
/// egal a `NOW()` -> `WHERE created_at < NOW()` supprimerait TOUTE la table (et
/// une valeur negative supprimerait meme les lignes futures). On refuse alors
/// d'executer le DELETE : mieux vaut conserver les donnees qu'une purge totale
/// declenchee par une simple case de config erronee.
pub fn valid_retention(days: i64) -> Option<i64> {
    if days >= 1 {
        Some(days)
    } else {
        None
    }
}

/// Vrai si une tache periodique est due : jamais executee, ou l'intervalle est
/// ecoule depuis la derniere execution. Un intervalle <= 0 est traite comme le
/// defaut `default_hours` (garde-fou contre une valeur absurde en config).
pub fn is_due(
    last: Option<DateTime<Utc>>,
    interval_hours: i64,
    default_hours: i64,
    now: DateTime<Utc>,
) -> bool {
    let hours = if interval_hours <= 0 {
        default_hours
    } else {
        interval_hours
    };
    match last {
        None => true,
        Some(last) => now - last >= Duration::hours(hours),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retention_positive_ok() {
        assert_eq!(valid_retention(30), Some(30));
        assert_eq!(valid_retention(1), Some(1));
    }

    #[test]
    fn retention_zero_or_negative_refused() {
        assert_eq!(valid_retention(0), None);
        assert_eq!(valid_retention(-7), None);
    }

    #[test]
    fn due_when_never_run() {
        assert!(is_due(None, 24, 24, Utc::now()));
    }

    #[test]
    fn due_when_interval_elapsed() {
        let now = Utc::now();
        assert!(is_due(Some(now - Duration::hours(25)), 24, 24, now));
        assert!(!is_due(Some(now - Duration::hours(23)), 24, 24, now));
    }

    #[test]
    fn invalid_interval_falls_back_to_default() {
        let now = Utc::now();
        // interval 0 -> défaut 24h : pas dû après 12h, dû après 25h.
        assert!(!is_due(Some(now - Duration::hours(12)), 0, 24, now));
        assert!(is_due(Some(now - Duration::hours(25)), -5, 24, now));
    }
}
