//! Machine à états de l'administrateur tournant. La DÉCISION (transitions,
//! seuils de timeout, rang round-robin, échéance) vit ici ; le bot ne garde
//! que la collecte des membres, les DM et les boutons.
//!
//! États : `idle` → `offering_candidate` → `awaiting_owner` → (`offering_stay`)
//! → `idle`. Les timestamps sont stockés en RFC3339 côté API.

use chrono::{DateTime, Utc};

/// Parse un timestamp RFC3339 optionnel (état persisté côté API).
pub fn parse_dt(s: Option<&str>) -> Option<DateTime<Utc>> {
    s.and_then(|v| DateTime::parse_from_rfc3339(v).ok())
        .map(|d| d.with_timezone(&Utc))
}

/// Heures écoulées depuis `since`. Sentinelle : un timestamp absent ou
/// invalide vaut `i64::MAX` (le timeout est considéré atteint — fail-open
/// pour ne pas bloquer la machine sur un état corrompu).
pub fn elapsed_hours(since: Option<&str>, now: DateTime<Utc>) -> i64 {
    match parse_dt(since) {
        Some(d) => (now - d).num_hours(),
        None => i64::MAX,
    }
}

/// Prochaine échéance de rotation : `now + period_days` (minimum 1 jour).
pub fn next_rotation_at(now: DateTime<Utc>, period_days: i64) -> DateTime<Utc> {
    now + chrono::Duration::days(period_days.max(1))
}

/// Rang round-robin d'un candidat : jamais servi d'abord (0), sinon trié par
/// date de dernier mandat ascendante (le plus ancien d'abord).
pub fn candidate_rank(served_at: Option<&str>) -> (u8, String) {
    match served_at {
        Some(d) => (1, d.to_string()),
        None => (0, String::new()),
    }
}

/// Décision du tick périodique.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationTick {
    /// `idle` et l'échéance est atteinte (ou jamais posée) : démarrer un cycle.
    StartRotation,
    /// Offre candidat / validation owner expirée : passer au candidat suivant
    /// ou terminer le tour.
    TimeoutAdvance,
    /// Offre « rester admin » expirée : on garde l'admin, retour à `idle`.
    TimeoutKeepAdmin,
    /// Rien à faire ce tick.
    Nothing,
}

/// Table de décision du tick : état courant + timestamps + timeout configuré.
pub fn decide_tick(
    state: &str,
    next_rotation: Option<&str>,
    candidate_offered_at: Option<&str>,
    timeout_hours: i64,
    now: DateTime<Utc>,
) -> RotationTick {
    match state {
        "idle" => {
            let due = match parse_dt(next_rotation) {
                None => true,
                Some(d) => now >= d,
            };
            if due {
                RotationTick::StartRotation
            } else {
                RotationTick::Nothing
            }
        }
        "offering_candidate" | "awaiting_owner"
            if elapsed_hours(candidate_offered_at, now) >= timeout_hours =>
        {
            RotationTick::TimeoutAdvance
        }
        "offering_stay" if elapsed_hours(candidate_offered_at, now) >= timeout_hours => {
            RotationTick::TimeoutKeepAdmin
        }
        _ => RotationTick::Nothing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-07-28T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn rfc(s: &str) -> String {
        s.to_string()
    }

    #[test]
    fn idle_without_deadline_starts() {
        assert_eq!(
            decide_tick("idle", None, None, 72, now()),
            RotationTick::StartRotation
        );
    }

    #[test]
    fn idle_before_deadline_waits() {
        let next = rfc("2026-08-01T00:00:00Z");
        assert_eq!(
            decide_tick("idle", Some(&next), None, 72, now()),
            RotationTick::Nothing
        );
    }

    #[test]
    fn idle_past_deadline_starts() {
        let next = rfc("2026-07-01T00:00:00Z");
        assert_eq!(
            decide_tick("idle", Some(&next), None, 72, now()),
            RotationTick::StartRotation
        );
    }

    #[test]
    fn offer_within_timeout_waits() {
        let offered = rfc("2026-07-28T00:00:00Z"); // il y a 12h
        assert_eq!(
            decide_tick("offering_candidate", None, Some(&offered), 72, now()),
            RotationTick::Nothing
        );
    }

    #[test]
    fn offer_expired_advances() {
        let offered = rfc("2026-07-20T00:00:00Z"); // > 72h
        for state in ["offering_candidate", "awaiting_owner"] {
            assert_eq!(
                decide_tick(state, None, Some(&offered), 72, now()),
                RotationTick::TimeoutAdvance
            );
        }
    }

    #[test]
    fn stay_expired_keeps_admin() {
        let offered = rfc("2026-07-20T00:00:00Z");
        assert_eq!(
            decide_tick("offering_stay", None, Some(&offered), 72, now()),
            RotationTick::TimeoutKeepAdmin
        );
    }

    #[test]
    fn missing_offered_at_is_treated_as_expired() {
        // Timestamp corrompu/absent -> i64::MAX -> timeout atteint (fail-open).
        assert_eq!(
            decide_tick("offering_candidate", None, None, 72, now()),
            RotationTick::TimeoutAdvance
        );
    }

    #[test]
    fn unknown_state_does_nothing() {
        assert_eq!(
            decide_tick("weird", None, None, 72, now()),
            RotationTick::Nothing
        );
    }

    #[test]
    fn rank_never_served_first_then_oldest() {
        let mut ids = vec![
            (1u64, candidate_rank(Some("2026-05-01T00:00:00Z"))),
            (2u64, candidate_rank(None)),
            (3u64, candidate_rank(Some("2026-01-01T00:00:00Z"))),
        ];
        ids.sort_by(|a, b| a.1.cmp(&b.1));
        let order: Vec<u64> = ids.into_iter().map(|(id, _)| id).collect();
        assert_eq!(order, vec![2, 3, 1]);
    }

    #[test]
    fn next_rotation_min_one_day() {
        let n = now();
        assert_eq!(next_rotation_at(n, 0), n + chrono::Duration::days(1));
        assert_eq!(next_rotation_at(n, 30), n + chrono::Duration::days(30));
    }

    #[test]
    fn elapsed_hours_sentinel_on_invalid() {
        assert_eq!(elapsed_hours(Some("not-a-date"), now()), i64::MAX);
        assert_eq!(elapsed_hours(None, now()), i64::MAX);
    }
}
