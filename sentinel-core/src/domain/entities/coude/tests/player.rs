use super::*;

#[test]
fn xp_for_level_zero_is_zero() {
    assert_eq!(xp_for_level(0), 0);
}

#[test]
fn xp_for_level_is_monotonic() {
    let mut prev = 0;
    for lv in 1..=COUDE_MAX_LEVEL {
        let x = xp_for_level(lv);
        assert!(
            x > prev,
            "xp_for_level({}) = {} should be > {}",
            lv,
            x,
            prev
        );
        prev = x;
    }
}

#[test]
fn xp_for_level_known_values() {
    // 50 * n^2 + 50 * n
    assert_eq!(xp_for_level(1), 100); // 50 + 50
    assert_eq!(xp_for_level(2), 300); // 200 + 100
    assert_eq!(xp_for_level(5), 1500); // 1250 + 250
    assert_eq!(xp_for_level(10), 5500); // 5000 + 500
    assert_eq!(xp_for_level(25), 32500); // 31250 + 1250
}

#[test]
fn title_for_level_ranges() {
    assert_eq!(title_for_level(0), "Debutant");
    assert_eq!(title_for_level(1), "Debutant");
    assert_eq!(title_for_level(4), "Debutant");
    assert_eq!(title_for_level(5), "Bagarreur");
    assert_eq!(title_for_level(9), "Bagarreur");
    assert_eq!(title_for_level(10), "Guerrier");
    assert_eq!(title_for_level(14), "Guerrier");
    assert_eq!(title_for_level(15), "Veteran");
    assert_eq!(title_for_level(19), "Veteran");
    assert_eq!(title_for_level(20), "Champion");
    assert_eq!(title_for_level(24), "Champion");
    assert_eq!(title_for_level(25), "Inarretable");
}

#[test]
fn title_for_level_above_max_returns_debutant() {
    // Comportement défini : niveau > 25 ou < 0 → "Debutant" (fallback)
    assert_eq!(title_for_level(26), "Debutant");
    assert_eq!(title_for_level(100), "Debutant");
    assert_eq!(title_for_level(-1), "Debutant");
}

#[test]
fn coude_max_level_is_25() {
    assert_eq!(COUDE_MAX_LEVEL, 25);
}

#[test]
fn combat_stat_parse_valid() {
    assert_eq!(CombatStat::parse("atk"), Some(CombatStat::Atk));
    assert_eq!(CombatStat::parse("def"), Some(CombatStat::Def));
}

#[test]
fn combat_stat_parse_invalid() {
    assert_eq!(CombatStat::parse(""), None);
    assert_eq!(CombatStat::parse("ATK"), None); // case-sensitive
    assert_eq!(CombatStat::parse("attack"), None);
    assert_eq!(CombatStat::parse("hp"), None);
}

#[test]
fn combat_stat_column_matches_parse_roundtrip() {
    for stat in [CombatStat::Atk, CombatStat::Def] {
        let column = stat.column();
        let parsed = CombatStat::parse(column).unwrap();
        assert_eq!(parsed, stat);
    }
}

#[test]
fn combat_stat_column_names_stable() {
    // Invariant : les noms de colonnes correspondent à la DB.
    assert_eq!(CombatStat::Atk.column(), "atk");
    assert_eq!(CombatStat::Def.column(), "def");
}
