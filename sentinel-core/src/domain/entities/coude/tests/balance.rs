use super::*;
use std::collections::HashMap;

#[test]
fn defaults_match_expectations() {
    let d = BalanceParams::default();
    assert_eq!(d.surprise_min_hp_pct, 40);
    assert!(d.surprise_allow_defender_counter);
    assert_eq!(d.steal_max_active_boosts, 3);
    assert_eq!(d.braquage_tools_consumed_success_pct, 50);
    assert_eq!(d.braquage_tools_consumed_fail_pct, 25);
    assert_eq!(d.double_coup_mode, DoubleCoupMode::Median);
    assert_eq!(d.rage_atk_bonus_pct, 40);
    assert_eq!(d.rage_def_malus_pct, 15);
    assert_eq!(d.coup_traitre_def_malus_pct, 40);
    assert_eq!(d.bouclier_def_bonus_pct, 20);
    assert_eq!(d.poison_damage_per_round, 5);
}

#[test]
fn from_config_parses_overrides() {
    let mut cfg = HashMap::new();
    cfg.insert("rage_atk_bonus_pct".to_string(), "60".to_string());
    cfg.insert("double_coup_mode".to_string(), "max".to_string());
    cfg.insert(
        "surprise_allow_defender_counter".to_string(),
        "false".to_string(),
    );
    let p = BalanceParams::from_config(&cfg);
    assert_eq!(p.rage_atk_bonus_pct, 60);
    assert_eq!(p.double_coup_mode, DoubleCoupMode::Max);
    assert!(!p.surprise_allow_defender_counter);
    assert_eq!(p.poison_damage_per_round, 5);
}

#[test]
fn from_config_ignores_invalid_values() {
    let mut cfg = HashMap::new();
    cfg.insert("rage_atk_bonus_pct".to_string(), "not_a_number".to_string());
    let p = BalanceParams::from_config(&cfg);
    assert_eq!(p.rage_atk_bonus_pct, 40);
}

#[test]
fn double_coup_aggregate_modes() {
    assert_eq!(DoubleCoupMode::Max.aggregate(5, 15), 15);
    assert_eq!(DoubleCoupMode::Min.aggregate(5, 15), 5);
    assert_eq!(DoubleCoupMode::Median.aggregate(5, 15), 10);
}

#[test]
fn double_coup_parse_all_variants() {
    assert_eq!(DoubleCoupMode::parse("max"), DoubleCoupMode::Max);
    assert_eq!(DoubleCoupMode::parse("min"), DoubleCoupMode::Min);
    assert_eq!(DoubleCoupMode::parse("median"), DoubleCoupMode::Median);
    assert_eq!(DoubleCoupMode::parse("mediane"), DoubleCoupMode::Median);
    assert_eq!(DoubleCoupMode::parse(""), DoubleCoupMode::Median);
    // Unknown → Median (fallback)
    assert_eq!(DoubleCoupMode::parse("unknown"), DoubleCoupMode::Median);
    // Case-insensitive + trim
    assert_eq!(DoubleCoupMode::parse("  MAX  "), DoubleCoupMode::Max);
    assert_eq!(DoubleCoupMode::parse("Min"), DoubleCoupMode::Min);
}

#[test]
fn from_config_parse_bool_all_truthy_values() {
    let truthy = ["1", "true", "yes", "on", "TRUE", "YES", "On"];
    for v in truthy {
        let mut cfg = HashMap::new();
        cfg.insert("surprise_allow_defender_counter".into(), v.into());
        let p = BalanceParams::from_config(&cfg);
        assert!(
            p.surprise_allow_defender_counter,
            "'{}' doit etre truthy",
            v
        );
    }
}

#[test]
fn from_config_parse_bool_falsy_and_invalid() {
    for v in ["0", "false", "no", "off", "wibble", ""] {
        let mut cfg = HashMap::new();
        cfg.insert("surprise_allow_defender_counter".into(), v.into());
        let p = BalanceParams::from_config(&cfg);
        assert!(
            !p.surprise_allow_defender_counter,
            "'{}' doit etre falsy",
            v
        );
    }
}
