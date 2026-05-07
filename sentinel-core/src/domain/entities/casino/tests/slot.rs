//! Tests unitaires du domaine slot machine.
//! Coverage : RNG seedable, evaluate, payouts, parsers, validation.

use super::*;
use rand::rngs::StdRng;
use rand::SeedableRng;

// ── Helpers ──

fn default_config() -> SlotConfig {
    SlotConfig::default()
}

fn config_with_only_one_symbol_high_weight() -> SlotConfig {
    // Permet de forcer un resultat deterministique via les poids :
    // si seul un symbole a un poids non-nul, il sera toujours tire.
    let mut c = SlotConfig::default();
    c.weights = vec![0, 0, 0, 0, 0, 0, 1]; // seul le jackpot
    c
}

// ══════════════════════════════════════════════════════════
// validate_slot_config
// ══════════════════════════════════════════════════════════

#[test]
fn validate_default_config_ok() {
    assert!(validate_slot_config(&default_config()).is_ok());
}

#[test]
fn validate_rejects_lengths_mismatch() {
    let mut c = default_config();
    c.weights.pop();
    assert_eq!(validate_slot_config(&c), Err(SlotConfigError::LengthsMismatch));
}

#[test]
fn validate_rejects_multipliers_mismatch() {
    let mut c = default_config();
    c.multipliers_3x.pop();
    assert_eq!(validate_slot_config(&c), Err(SlotConfigError::LengthsMismatch));
}

#[test]
fn validate_rejects_too_few_symbols() {
    let mut c = default_config();
    c.symbols = vec!["🍒".into()];
    c.weights = vec![1];
    c.multipliers_3x = vec![1.0];
    assert_eq!(validate_slot_config(&c), Err(SlotConfigError::EmptySymbols));
}

#[test]
fn validate_rejects_empty_symbols() {
    let mut c = default_config();
    c.symbols = vec![];
    c.weights = vec![];
    c.multipliers_3x = vec![];
    assert_eq!(validate_slot_config(&c), Err(SlotConfigError::EmptySymbols));
}

#[test]
fn validate_rejects_all_weights_zero() {
    let mut c = default_config();
    c.weights = vec![0; c.weights.len()];
    assert_eq!(validate_slot_config(&c), Err(SlotConfigError::AllWeightsZero));
}

#[test]
fn validate_rejects_min_bet_zero() {
    let mut c = default_config();
    c.min_bet = 0;
    assert_eq!(validate_slot_config(&c), Err(SlotConfigError::BetRangeInvalid));
}

#[test]
fn validate_rejects_min_bet_negative() {
    let mut c = default_config();
    c.min_bet = -10;
    assert_eq!(validate_slot_config(&c), Err(SlotConfigError::BetRangeInvalid));
}

#[test]
fn validate_rejects_min_bet_above_max_bet() {
    let mut c = default_config();
    c.min_bet = 100;
    c.max_bet = 50;
    assert_eq!(validate_slot_config(&c), Err(SlotConfigError::BetRangeInvalid));
}

#[test]
fn validate_accepts_min_equal_to_max() {
    let mut c = default_config();
    c.min_bet = 50;
    c.max_bet = 50;
    assert!(validate_slot_config(&c).is_ok());
}

#[test]
fn validate_rejects_share_pct_above_100() {
    let mut c = default_config();
    c.jackpot_pool_share_pct = 101.0;
    assert_eq!(validate_slot_config(&c), Err(SlotConfigError::SharePctOutOfRange));
}

#[test]
fn validate_rejects_share_pct_negative() {
    let mut c = default_config();
    c.jackpot_pool_share_pct = -1.0;
    assert_eq!(validate_slot_config(&c), Err(SlotConfigError::SharePctOutOfRange));
}

#[test]
fn validate_accepts_share_pct_zero() {
    let mut c = default_config();
    c.jackpot_pool_share_pct = 0.0;
    assert!(validate_slot_config(&c).is_ok());
}

#[test]
fn config_error_strings_are_descriptive() {
    assert!(SlotConfigError::LengthsMismatch.as_str().contains("longueur"));
    assert!(SlotConfigError::AllWeightsZero.as_str().contains("poids"));
}

// ══════════════════════════════════════════════════════════
// spin_with_rng — determinisme avec seed
// ══════════════════════════════════════════════════════════

#[test]
fn spin_with_seed_42_is_deterministic() {
    let cfg = default_config();
    let mut rng1 = StdRng::seed_from_u64(42);
    let mut rng2 = StdRng::seed_from_u64(42);
    assert_eq!(spin_with_rng(&mut rng1, &cfg), spin_with_rng(&mut rng2, &cfg));
}

#[test]
fn spin_with_different_seeds_can_differ() {
    let cfg = default_config();
    let mut rng1 = StdRng::seed_from_u64(1);
    let mut rng2 = StdRng::seed_from_u64(2);
    let mut diff_count = 0;
    for _ in 0..50 {
        if spin_with_rng(&mut rng1, &cfg) != spin_with_rng(&mut rng2, &cfg) {
            diff_count += 1;
        }
    }
    assert!(diff_count > 0, "deux seeds differents devraient produire des spins differents");
}

#[test]
fn spin_returns_indices_in_range() {
    let cfg = default_config();
    let mut rng = StdRng::seed_from_u64(7);
    for _ in 0..1000 {
        let s = spin_with_rng(&mut rng, &cfg);
        for i in s.iter() {
            assert!(*i < cfg.symbols.len(), "index hors range");
        }
    }
}

#[test]
fn spin_with_only_jackpot_weight_always_returns_jackpot_indices() {
    let cfg = config_with_only_one_symbol_high_weight();
    let jackpot_idx = cfg.symbols.len() - 1;
    let mut rng = StdRng::seed_from_u64(1);
    for _ in 0..50 {
        let s = spin_with_rng(&mut rng, &cfg);
        assert_eq!(s, [jackpot_idx, jackpot_idx, jackpot_idx]);
    }
}

#[test]
fn spin_weighted_distribution_respects_proportions() {
    // Sur 10000 spins individuels (1 tirage), le ratio cerise (poids 30) /
    // jackpot (poids 1) doit etre proche de 30:1 (tolerance large : 15-60).
    let cfg = default_config();
    let mut rng = StdRng::seed_from_u64(123);
    let dist = rand::distributions::WeightedIndex::new(&cfg.weights).unwrap();
    let mut counts = vec![0u32; cfg.symbols.len()];
    for _ in 0..10_000 {
        counts[<rand::distributions::WeightedIndex<u32> as rand::prelude::Distribution<usize>>::sample(&dist, &mut rng)] += 1;
    }
    let cherry = counts[0] as f64;
    let jackpot = (counts[cfg.symbols.len() - 1] as f64).max(1.0);
    let ratio = cherry / jackpot;
    assert!(ratio > 15.0 && ratio < 60.0, "ratio cerise/jackpot = {ratio}, attendu ~30");
}

// ══════════════════════════════════════════════════════════
// evaluate_spin
// ══════════════════════════════════════════════════════════

#[test]
fn evaluate_three_cherries_is_three_of_a_kind() {
    let cfg = default_config();
    let r = evaluate_spin(&[0, 0, 0], &cfg);
    assert_eq!(r, SpinOutcome::ThreeOfAKind { symbol_index: 0, multiplier: 2.0 });
}

#[test]
fn evaluate_three_jackpot_symbols_is_jackpot() {
    let cfg = default_config();
    let last = cfg.symbols.len() - 1;
    let r = evaluate_spin(&[last, last, last], &cfg);
    assert_eq!(r, SpinOutcome::Jackpot { multiplier: 100.0 });
}

#[test]
fn evaluate_two_of_a_kind_first_two_when_enabled() {
    let cfg = default_config();
    assert_eq!(evaluate_spin(&[2, 2, 5], &cfg), SpinOutcome::RefundTwoOfAKind);
}

#[test]
fn evaluate_two_of_a_kind_last_two_when_enabled() {
    let cfg = default_config();
    assert_eq!(evaluate_spin(&[5, 2, 2], &cfg), SpinOutcome::RefundTwoOfAKind);
}

#[test]
fn evaluate_two_of_a_kind_first_and_last_when_enabled() {
    // 1ere et 3eme position identiques mais pas la 2eme : compte aussi.
    let cfg = default_config();
    assert_eq!(evaluate_spin(&[3, 1, 3], &cfg), SpinOutcome::RefundTwoOfAKind);
}

#[test]
fn evaluate_two_of_a_kind_returns_loss_when_disabled() {
    let mut cfg = default_config();
    cfg.payout_2x_enabled = false;
    assert_eq!(evaluate_spin(&[2, 2, 5], &cfg), SpinOutcome::Loss);
}

#[test]
fn evaluate_three_distinct_is_loss() {
    let cfg = default_config();
    assert_eq!(evaluate_spin(&[0, 1, 2], &cfg), SpinOutcome::Loss);
}

#[test]
fn evaluate_three_of_a_kind_picks_correct_multiplier() {
    let cfg = default_config();
    // index 4 = bell, multiplier 12.0 par defaut
    assert_eq!(evaluate_spin(&[4, 4, 4], &cfg),
               SpinOutcome::ThreeOfAKind { symbol_index: 4, multiplier: 12.0 });
}

// ══════════════════════════════════════════════════════════
// compute_payout
// ══════════════════════════════════════════════════════════

#[test]
fn payout_loss_is_zero() {
    assert_eq!(compute_payout(100, &SpinOutcome::Loss, 5000), 0);
}

#[test]
fn payout_refund_returns_full_mise() {
    assert_eq!(compute_payout(150, &SpinOutcome::RefundTwoOfAKind, 5000), 150);
}

#[test]
fn payout_three_of_a_kind_multiplies_mise() {
    let outcome = SpinOutcome::ThreeOfAKind { symbol_index: 2, multiplier: 5.0 };
    assert_eq!(compute_payout(100, &outcome, 0), 500);
}

#[test]
fn payout_three_of_a_kind_rounds_to_int() {
    // 17 * 1.5 = 25.5 -> arrondi a 26
    let outcome = SpinOutcome::ThreeOfAKind { symbol_index: 0, multiplier: 1.5 };
    assert_eq!(compute_payout(17, &outcome, 0), 26);
}

#[test]
fn payout_jackpot_includes_pool() {
    let outcome = SpinOutcome::Jackpot { multiplier: 100.0 };
    assert_eq!(compute_payout(50, &outcome, 12345), 50 * 100 + 12345);
}

#[test]
fn payout_jackpot_zero_pool_still_pays_multiplier() {
    let outcome = SpinOutcome::Jackpot { multiplier: 100.0 };
    assert_eq!(compute_payout(10, &outcome, 0), 1000);
}

#[test]
fn payout_zero_mise_returns_zero_for_loss_and_refund() {
    assert_eq!(compute_payout(0, &SpinOutcome::Loss, 0), 0);
    assert_eq!(compute_payout(0, &SpinOutcome::RefundTwoOfAKind, 0), 0);
}

// ══════════════════════════════════════════════════════════
// compute_jackpot_contribution
// ══════════════════════════════════════════════════════════

#[test]
fn jackpot_contribution_at_one_percent() {
    assert_eq!(compute_jackpot_contribution(100, 1.0), 1);
    assert_eq!(compute_jackpot_contribution(1000, 1.0), 10);
}

#[test]
fn jackpot_contribution_at_five_percent() {
    assert_eq!(compute_jackpot_contribution(100, 5.0), 5);
}

#[test]
fn jackpot_contribution_floors_decimals() {
    // 99 * 1% = 0.99 -> floor -> 0
    assert_eq!(compute_jackpot_contribution(99, 1.0), 0);
}

#[test]
fn jackpot_contribution_zero_pct_returns_zero() {
    assert_eq!(compute_jackpot_contribution(1000, 0.0), 0);
}

#[test]
fn jackpot_contribution_zero_mise_returns_zero() {
    assert_eq!(compute_jackpot_contribution(0, 5.0), 0);
}

// ══════════════════════════════════════════════════════════
// CSV parsers
// ══════════════════════════════════════════════════════════

#[test]
fn parse_symbols_emojis() {
    let s = parse_csv_symbols("🍒,🍋,🍊");
    assert_eq!(s, vec!["🍒".to_string(), "🍋".to_string(), "🍊".to_string()]);
}

#[test]
fn parse_symbols_trims_whitespace() {
    let s = parse_csv_symbols(" 🍒 , 🍋,🍊 ");
    assert_eq!(s, vec!["🍒".to_string(), "🍋".to_string(), "🍊".to_string()]);
}

#[test]
fn parse_symbols_drops_empty_entries() {
    let s = parse_csv_symbols("🍒,,🍋");
    assert_eq!(s, vec!["🍒".to_string(), "🍋".to_string()]);
}

#[test]
fn parse_weights_basic() {
    assert_eq!(parse_csv_weights("30,25,20,15"), vec![30, 25, 20, 15]);
}

#[test]
fn parse_weights_invalid_becomes_zero() {
    assert_eq!(parse_csv_weights("30,abc,20"), vec![30, 0, 20]);
}

#[test]
fn parse_multipliers_with_decimals() {
    assert_eq!(parse_csv_multipliers("2.5,3,5.0"), vec![2.5, 3.0, 5.0]);
}

#[test]
fn parse_multipliers_invalid_becomes_zero() {
    assert_eq!(parse_csv_multipliers("2,xx,5"), vec![2.0, 0.0, 5.0]);
}

// ══════════════════════════════════════════════════════════
// Default config (sanity)
// ══════════════════════════════════════════════════════════

#[test]
fn default_config_has_seven_symbols() {
    assert_eq!(default_config().symbols.len(), 7);
}

#[test]
fn default_config_validates() {
    assert!(validate_slot_config(&default_config()).is_ok());
}

#[test]
fn default_jackpot_multiplier_is_100() {
    let cfg = default_config();
    assert_eq!(*cfg.multipliers_3x.last().unwrap(), 100.0);
}

#[test]
fn default_cherry_weight_is_highest() {
    let cfg = default_config();
    let max = *cfg.weights.iter().max().unwrap();
    assert_eq!(cfg.weights[0], max);
}

#[test]
fn default_jackpot_weight_is_lowest() {
    let cfg = default_config();
    let min = *cfg.weights.iter().min().unwrap();
    assert_eq!(*cfg.weights.last().unwrap(), min);
}

// ══════════════════════════════════════════════════════════
// Scenarios end-to-end (combinaisons spin + evaluate + payout)
// ══════════════════════════════════════════════════════════

#[test]
fn forced_jackpot_scenario_full_payout() {
    let cfg = config_with_only_one_symbol_high_weight();
    let mut rng = StdRng::seed_from_u64(42);
    let symbols = spin_with_rng(&mut rng, &cfg);
    let outcome = evaluate_spin(&symbols, &cfg);
    assert!(matches!(outcome, SpinOutcome::Jackpot { .. }));
    let payout = compute_payout(100, &outcome, 5000);
    // 100 mise * 100 (jackpot multiplier last symbol) + 5000 pool = 15000
    assert_eq!(payout, 100 * 100 + 5000);
}

#[test]
fn three_of_a_kind_non_jackpot_no_pool_added() {
    let cfg = default_config();
    // Force 3x cerise (index 0) directement
    let outcome = evaluate_spin(&[0, 0, 0], &cfg);
    let payout_with_pool = compute_payout(50, &outcome, 99999);
    let payout_no_pool = compute_payout(50, &outcome, 0);
    assert_eq!(payout_with_pool, payout_no_pool, "pool ignore sur ThreeOfAKind non-jackpot");
    assert_eq!(payout_no_pool, 100); // 50 * 2.0
}

#[test]
fn loss_scenario_returns_zero_with_jackpot_contribution_only() {
    let cfg = default_config();
    let outcome = evaluate_spin(&[0, 1, 2], &cfg);
    assert_eq!(outcome, SpinOutcome::Loss);
    assert_eq!(compute_payout(100, &outcome, 5000), 0);
    // Le jackpot pool s alimente quand meme.
    assert_eq!(compute_jackpot_contribution(100, 1.0), 1);
}
