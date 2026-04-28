use super::*;
use chrono::Duration;

#[test]
fn db_str_round_trips_all_kinds() {
    for k in [
        UltimateKind::Bourrin,
        UltimateKind::Agile,
        UltimateKind::Fourbe,
        UltimateKind::Tank,
    ] {
        assert_eq!(UltimateKind::from_db_str(k.as_db_str()), Some(k));
    }
}

#[test]
fn unknown_kind_returns_none() {
    assert_eq!(UltimateKind::from_db_str("foo"), None);
    assert_eq!(UltimateKind::from_db_str(""), None);
}

#[test]
fn cooldown_days_fourbe_is_14_others_7() {
    assert_eq!(UltimateKind::Bourrin.cooldown_days(), 7);
    assert_eq!(UltimateKind::Agile.cooldown_days(), 7);
    assert_eq!(UltimateKind::Tank.cooldown_days(), 7);
    assert_eq!(UltimateKind::Fourbe.cooldown_days(), 14);
}

#[test]
fn class_key_matches_db_str() {
    for k in [
        UltimateKind::Bourrin,
        UltimateKind::Agile,
        UltimateKind::Fourbe,
        UltimateKind::Tank,
    ] {
        assert_eq!(k.class_key(), k.as_db_str());
    }
}

#[test]
fn unlock_level_is_10() {
    assert_eq!(ULTIMATE_UNLOCK_LEVEL, 10);
}

#[test]
fn ready_below_unlock_level() {
    assert!(!ultimate_ready(1, UltimateKind::Bourrin, None));
    assert!(!ultimate_ready(9, UltimateKind::Bourrin, None));
}

#[test]
fn ready_at_unlock_level_no_history() {
    assert!(ultimate_ready(10, UltimateKind::Bourrin, None));
    assert!(ultimate_ready(50, UltimateKind::Bourrin, None));
}

#[test]
fn not_ready_within_cooldown() {
    let recent = Utc::now() - Duration::days(3);
    assert!(!ultimate_ready(10, UltimateKind::Bourrin, Some(recent)));
}

#[test]
fn ready_after_cooldown_elapsed() {
    let old = Utc::now() - Duration::days(8);
    assert!(ultimate_ready(10, UltimateKind::Bourrin, Some(old)));
}

#[test]
fn fourbe_double_cooldown() {
    let day_8 = Utc::now() - Duration::days(8);
    assert!(ultimate_ready(10, UltimateKind::Bourrin, Some(day_8)));
    // Pas pret pour Fourbe : 8 jours < 14 jours.
    assert!(!ultimate_ready(10, UltimateKind::Fourbe, Some(day_8)));
    let day_15 = Utc::now() - Duration::days(15);
    assert!(ultimate_ready(10, UltimateKind::Fourbe, Some(day_15)));
}
