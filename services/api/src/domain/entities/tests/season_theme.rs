use super::*;

#[test]
fn four_themes_defined() {
    assert_eq!(SEASON_THEMES.len(), 4);
}

#[test]
fn keys_are_distinct() {
    let mut keys: Vec<_> = SEASON_THEMES.iter().map(|t| t.key).collect();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), SEASON_THEMES.len());
}

#[test]
fn lookup_known_key() {
    let t = season_theme_by_key("chaos").unwrap();
    assert_eq!(t.label, "Saison du Chaos");
    assert_eq!(t.chaos_multiplier, 2.0);
}

#[test]
fn lookup_unknown_returns_none() {
    assert!(season_theme_by_key("foo").is_none());
    assert!(season_theme_by_key("").is_none());
}

#[test]
fn each_theme_has_emoji_and_tagline() {
    for t in SEASON_THEMES {
        assert!(!t.emoji.is_empty(), "{} sans emoji", t.key);
        assert!(!t.tagline.is_empty(), "{} sans tagline", t.key);
    }
}

#[test]
fn theme_specific_multipliers() {
    let chaos = season_theme_by_key("chaos").unwrap();
    assert_eq!(chaos.chaos_multiplier, 2.0);
    assert_eq!(chaos.tank_def_bonus_pct, 0.0);

    let tank = season_theme_by_key("tank").unwrap();
    assert_eq!(tank.tank_def_bonus_pct, 20.0);
    assert_eq!(tank.chaos_multiplier, 1.0);

    let vol = season_theme_by_key("vol").unwrap();
    assert_eq!(vol.steal_gain_multiplier, 1.5);
    assert_eq!(vol.steal_protection_efficiency, 0.75);

    let braquage = season_theme_by_key("braquage").unwrap();
    assert_eq!(braquage.braquage_cooldown_multiplier, 0.5);
}

#[test]
fn config_key_is_stable() {
    assert_eq!(CURRENT_SEASON_THEME_CONFIG_KEY, "current_season_theme");
}

#[test]
fn theme_for_season_rotates() {
    assert_eq!(theme_for_season(1).key, "chaos");
    assert_eq!(theme_for_season(2).key, "tank");
    assert_eq!(theme_for_season(3).key, "vol");
    assert_eq!(theme_for_season(4).key, "braquage");
    // Rotation circulaire.
    assert_eq!(theme_for_season(5).key, "chaos");
    assert_eq!(theme_for_season(8).key, "braquage");
    assert_eq!(theme_for_season(9).key, "chaos");
}

#[test]
fn theme_for_season_zero_or_negative_defaults_to_chaos() {
    assert_eq!(theme_for_season(0).key, "chaos");
    assert_eq!(theme_for_season(-1).key, "chaos");
}
