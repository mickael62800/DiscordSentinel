use super::*;
use std::collections::HashMap;

fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[test]
fn empty_config_equals_default() {
    let cfg = CoudeEconomyConfig::from_config(&HashMap::new());
    assert_eq!(cfg, CoudeEconomyConfig::default());
}

#[test]
fn parses_all_valid_keys() {
    let cfg = CoudeEconomyConfig::from_config(&map(&[
        ("combat_xp_winner_base", "20"),
        ("combat_xp_winner_underdog", "40"),
        ("combat_xp_loser", "8"),
        ("steal_afk_min_pct", "5"),
        ("steal_afk_max_pct", "12"),
        ("steal_active_min_pct", "12"),
        ("steal_active_max_pct", "30"),
        ("tout_ou_rien_win_probability", "0.6"),
        ("tout_ou_rien_win_multiplier", "2.5"),
        ("tout_ou_rien_loss_keep_pct", "0.3"),
        ("heist_base_success_pct", "8"),
        ("heist_max_success_pct", "60"),
        ("heist_gain_min_pct", "20"),
        ("heist_gain_max_pct", "80"),
        ("curse_cost_coins", "500"),
        ("curse_lift_multiplier", "3"),
        ("leaky_wallet_fee_coins", "25"),
        ("fausse_assurance_fee_coins", "300"),
        ("tournament_prize_pool_pct", "15"),
    ]));
    assert_eq!(cfg.combat_xp_winner_base, 20);
    assert_eq!(cfg.combat_xp_winner_underdog, 40);
    assert_eq!(cfg.combat_xp_loser, 8);
    assert_eq!(cfg.steal_afk_min_pct, 5);
    assert_eq!(cfg.steal_active_max_pct, 30);
    assert_eq!(cfg.tout_ou_rien_win_probability, 0.6);
    assert_eq!(cfg.tout_ou_rien_win_multiplier, 2.5);
    assert_eq!(cfg.tout_ou_rien_loss_keep_pct, 0.3);
    assert_eq!(cfg.heist_max_success_pct, 60);
    assert_eq!(cfg.curse_cost_coins, 500);
    assert_eq!(cfg.curse_lift_multiplier, 3);
    assert_eq!(cfg.leaky_wallet_fee_coins, 25);
    assert_eq!(cfg.fausse_assurance_fee_coins, 300);
    assert_eq!(cfg.tournament_prize_pool_pct, 15);
}

#[test]
fn malformed_values_fall_back_to_default() {
    let d = CoudeEconomyConfig::default();
    let cfg = CoudeEconomyConfig::from_config(&map(&[
        ("combat_xp_winner_base", "abc"),
        ("tout_ou_rien_win_probability", ""),
        ("heist_gain_min_pct", "not_a_number"),
    ]));
    assert_eq!(cfg.combat_xp_winner_base, d.combat_xp_winner_base);
    assert_eq!(
        cfg.tout_ou_rien_win_probability,
        d.tout_ou_rien_win_probability
    );
    assert_eq!(cfg.heist_gain_min_pct, d.heist_gain_min_pct);
}

#[test]
fn f64_accepts_french_comma() {
    let cfg = CoudeEconomyConfig::from_config(&map(&[("tout_ou_rien_loss_keep_pct", "0,25")]));
    assert_eq!(cfg.tout_ou_rien_loss_keep_pct, 0.25);
}

#[test]
fn percentages_clamped_to_0_100() {
    let cfg = CoudeEconomyConfig::from_config(&map(&[
        ("steal_afk_min_pct", "200"),
        ("steal_afk_max_pct", "250"),
        ("tournament_prize_pool_pct", "150"),
    ]));
    // steal afk both clamped to 100 -> min<=max still holds.
    assert_eq!(cfg.steal_afk_min_pct, 100);
    assert_eq!(cfg.steal_afk_max_pct, 100);
    assert_eq!(cfg.tournament_prize_pool_pct, 100);
}

#[test]
fn probability_clamped_to_unit() {
    let cfg = CoudeEconomyConfig::from_config(&map(&[
        ("tout_ou_rien_win_probability", "5.0"),
        ("tout_ou_rien_loss_keep_pct", "-1.0"),
    ]));
    assert_eq!(cfg.tout_ou_rien_win_probability, 1.0);
    assert_eq!(cfg.tout_ou_rien_loss_keep_pct, 0.0);
}

#[test]
fn win_multiplier_floored_at_one() {
    let cfg = CoudeEconomyConfig::from_config(&map(&[("tout_ou_rien_win_multiplier", "0.5")]));
    assert_eq!(cfg.tout_ou_rien_win_multiplier, 2.0); // fallback to default (>=1)
}

#[test]
fn lift_multiplier_floored_at_one() {
    let cfg = CoudeEconomyConfig::from_config(&map(&[("curse_lift_multiplier", "0")]));
    assert_eq!(cfg.curse_lift_multiplier, 1);
}

#[test]
fn coin_amounts_forced_non_negative() {
    let cfg = CoudeEconomyConfig::from_config(&map(&[
        ("curse_cost_coins", "-100"),
        ("leaky_wallet_fee_coins", "-5"),
        ("combat_xp_loser", "-3"),
    ]));
    assert_eq!(cfg.curse_cost_coins, 0);
    assert_eq!(cfg.leaky_wallet_fee_coins, 0);
    assert_eq!(cfg.combat_xp_loser, 0);
}

#[test]
fn inverted_steal_pair_falls_back_to_defaults() {
    let d = CoudeEconomyConfig::default();
    let cfg = CoudeEconomyConfig::from_config(&map(&[
        ("steal_afk_min_pct", "40"),
        ("steal_afk_max_pct", "10"),
    ]));
    assert_eq!(cfg.steal_afk_min_pct, d.steal_afk_min_pct);
    assert_eq!(cfg.steal_afk_max_pct, d.steal_afk_max_pct);
}

#[test]
fn inverted_heist_gain_pair_falls_back_to_defaults() {
    let d = CoudeEconomyConfig::default();
    let cfg = CoudeEconomyConfig::from_config(&map(&[
        ("heist_gain_min_pct", "90"),
        ("heist_gain_max_pct", "20"),
    ]));
    assert_eq!(cfg.heist_gain_min_pct, d.heist_gain_min_pct);
    assert_eq!(cfg.heist_gain_max_pct, d.heist_gain_max_pct);
}

#[test]
fn gameplay_low_keys_parse_and_apply() {
    let cfg = CoudeEconomyConfig::from_config(&map(&[
        ("daily_chaos_max_events", "8"),
        ("min_coins_eligible", "25"),
        ("flavor_line_probability", "0.5"),
        ("honor_debt_threshold", "5"),
        ("underdog_level_gap", "4"),
    ]));
    assert_eq!(cfg.daily_chaos_max_events, 8);
    assert_eq!(cfg.min_coins_eligible, 25);
    assert_eq!(cfg.flavor_line_probability, 0.5);
    assert_eq!(cfg.honor_debt_threshold, 5);
    assert_eq!(cfg.underdog_level_gap, 4);
}

#[test]
fn gameplay_low_guards_apply() {
    let d = CoudeEconomyConfig::default();
    let cfg = CoudeEconomyConfig::from_config(&map(&[
        ("daily_chaos_max_events", "-3"),
        ("min_coins_eligible", "-10"),
        ("flavor_line_probability", "5.0"),
        ("honor_debt_threshold", "-1"),
        ("underdog_level_gap", "-2"),
    ]));
    // Compteurs / seuils >= 0.
    assert_eq!(cfg.daily_chaos_max_events, 0);
    assert_eq!(cfg.min_coins_eligible, 0);
    assert_eq!(cfg.honor_debt_threshold, 0);
    assert_eq!(cfg.underdog_level_gap, 0);
    // Probabilite bornee [0, 1].
    assert_eq!(cfg.flavor_line_probability, 1.0);
    // NaN / malforme -> defaut.
    let cfg2 = CoudeEconomyConfig::from_config(&map(&[("flavor_line_probability", "abc")]));
    assert_eq!(cfg2.flavor_line_probability, d.flavor_line_probability);
}

#[test]
fn inverted_heist_base_over_max_falls_back() {
    let d = CoudeEconomyConfig::default();
    let cfg = CoudeEconomyConfig::from_config(&map(&[
        ("heist_base_success_pct", "80"),
        ("heist_max_success_pct", "50"),
    ]));
    assert_eq!(cfg.heist_base_success_pct, d.heist_base_success_pct);
    assert_eq!(cfg.heist_max_success_pct, d.heist_max_success_pct);
}
