//! Tests unitaires de la Roue du Destin.

use super::*;
use rand::rngs::StdRng;
use rand::SeedableRng;

// ══════════════════════════════════════════════════════════
// WHEEL_CASES — invariants
// ══════════════════════════════════════════════════════════

#[test]
fn ten_cases_defined() {
    assert_eq!(WHEEL_CASES.len(), 10);
}

#[test]
fn all_cases_have_distinct_keys() {
    let mut keys: Vec<_> = WHEEL_CASES.iter().map(|c| c.key).collect();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), 10);
}

#[test]
fn weights_sum_to_100() {
    let total: u32 = WHEEL_CASES.iter().map(|c| c.weight).sum();
    assert_eq!(total, 100, "somme des poids = 100 (% lisibles)");
}

#[test]
fn at_least_one_case_has_positive_payout() {
    assert!(WHEEL_CASES.iter().any(|c| c.payout > 0));
}

#[test]
fn at_least_one_case_has_negative_payout() {
    assert!(WHEEL_CASES.iter().any(|c| c.payout < 0));
}

#[test]
fn neutral_case_blanche_exists() {
    let blanche = WHEEL_CASES.iter().find(|c| c.key == "blanche").unwrap();
    assert_eq!(blanche.payout, 0);
}

#[test]
fn licorne_is_rarest_jackpot() {
    let licorne = WHEEL_CASES.iter().find(|c| c.key == "licorne").unwrap();
    assert_eq!(licorne.weight, 1);
    assert_eq!(licorne.payout, 10000);
}

#[test]
fn blanche_is_most_common() {
    let blanche = WHEEL_CASES.iter().find(|c| c.key == "blanche").unwrap();
    let max_weight = WHEEL_CASES.iter().map(|c| c.weight).max().unwrap();
    assert_eq!(blanche.weight, max_weight);
}

// ══════════════════════════════════════════════════════════
// spin_with_rng — determinisme + distribution
// ══════════════════════════════════════════════════════════

#[test]
fn spin_with_seed_42_is_deterministic() {
    let mut r1 = StdRng::seed_from_u64(42);
    let mut r2 = StdRng::seed_from_u64(42);
    let s1 = spin_with_rng(&mut r1);
    let s2 = spin_with_rng(&mut r2);
    assert_eq!(s1, s2);
}

#[test]
fn spin_returns_valid_case_index() {
    let mut rng = StdRng::seed_from_u64(7);
    for _ in 0..100 {
        let s = spin_with_rng(&mut rng);
        assert!(s.case_index < WHEEL_CASES.len());
        assert_eq!(s.case, WHEEL_CASES[s.case_index]);
    }
}

#[test]
fn spin_distribution_respects_weights_statistically() {
    // Sur 10 000 spins, blanche (poids 25) doit sortir ~25× plus que licorne
    // (poids 1). Tolerance large.
    let mut rng = StdRng::seed_from_u64(123);
    let mut blanche_count = 0;
    let mut licorne_count = 0;
    for _ in 0..10_000 {
        let s = spin_with_rng(&mut rng);
        match s.case.key {
            "blanche" => blanche_count += 1,
            "licorne" => licorne_count += 1,
            _ => {}
        }
    }
    let licorne_count = licorne_count.max(1); // eviter div par 0
    let ratio = blanche_count as f64 / licorne_count as f64;
    assert!(
        ratio > 10.0 && ratio < 80.0,
        "ratio blanche/licorne = {ratio}, attendu ~25"
    );
}

#[test]
fn spin_can_produce_different_results_with_different_seeds() {
    let mut r1 = StdRng::seed_from_u64(1);
    let mut r2 = StdRng::seed_from_u64(2);
    let mut diff_count = 0;
    for _ in 0..30 {
        if spin_with_rng(&mut r1) != spin_with_rng(&mut r2) {
            diff_count += 1;
        }
    }
    assert!(
        diff_count > 5,
        "deux seeds differents -> spins differents (au moins 5)"
    );
}

// ══════════════════════════════════════════════════════════
// is_memorable_case
// ══════════════════════════════════════════════════════════

#[test]
fn jackpot_is_memorable() {
    assert!(is_memorable_case("jackpot"));
}

#[test]
fn licorne_is_memorable() {
    assert!(is_memorable_case("licorne"));
}

#[test]
fn bombe_is_memorable() {
    assert!(is_memorable_case("bombe"));
}

#[test]
fn blanche_is_not_memorable() {
    assert!(!is_memorable_case("blanche"));
}

#[test]
fn pq_is_not_memorable() {
    assert!(!is_memorable_case("pq"));
}

#[test]
fn unknown_key_is_not_memorable() {
    assert!(!is_memorable_case("foo"));
}

// ══════════════════════════════════════════════════════════
// Bounds metier
// ══════════════════════════════════════════════════════════

#[test]
fn no_payout_above_jackpot_threshold() {
    // Sanity : eviter qu une modif rajoute par erreur une case +1M coins.
    for c in WHEEL_CASES {
        assert!(
            c.payout <= 10000,
            "case {} payout {} > seuil 10000",
            c.key,
            c.payout
        );
        assert!(
            c.payout >= -2000,
            "case {} payout {} < seuil -2000",
            c.key,
            c.payout
        );
    }
}

#[test]
fn all_weights_positive() {
    for c in WHEEL_CASES {
        assert!(
            c.weight > 0,
            "case {} a un poids 0 -> ne sortira jamais",
            c.key
        );
    }
}

// ══════════════════════════════════════════════════════════
// Heartbreak (cf. COUPE_AMELIORATIONS 5.1)
// ══════════════════════════════════════════════════════════

#[test]
fn heartbreak_blocks_licorne_in_10000_spins() {
    let mut rng = StdRng::seed_from_u64(42);
    for _ in 0..10_000 {
        let outcome = spin_with_rng_curses(&mut rng, true);
        assert_ne!(
            outcome.case.key, "licorne",
            "Heartbreak doit bloquer la licorne"
        );
    }
}

#[test]
fn no_heartbreak_can_yield_licorne() {
    let mut rng = StdRng::seed_from_u64(7);
    let mut saw_licorne = false;
    for _ in 0..50_000 {
        let outcome = spin_with_rng_curses(&mut rng, false);
        if outcome.case.key == "licorne" {
            saw_licorne = true;
            break;
        }
    }
    assert!(
        saw_licorne,
        "sans Heartbreak la licorne doit pouvoir tomber sur 50k spins"
    );
}

// ══════════════════════════════════════════════════════════
// WheelConfig — defauts + garde-fous
// ══════════════════════════════════════════════════════════

#[test]
fn default_config_matches_wheel_cases() {
    let cfg = WheelConfig::default();
    assert_eq!(cfg.segments.len(), WHEEL_CASES.len());
    for (seg, case) in cfg.segments.iter().zip(WHEEL_CASES) {
        assert_eq!(seg.payout, case.payout);
        assert_eq!(seg.weight, case.weight);
    }
}

#[test]
fn normalized_clamps_payout_to_50000() {
    let mut cfg = WheelConfig::default();
    cfg.segments[0].payout = 9_999_999;
    cfg.segments[1].payout = -9_999_999;
    let cfg = cfg.normalized();
    assert_eq!(cfg.segments[0].payout, WHEEL_PAYOUT_CLAMP);
    assert_eq!(cfg.segments[1].payout, -WHEEL_PAYOUT_CLAMP);
}

#[test]
fn normalized_restores_default_weights_when_all_zero() {
    let mut cfg = WheelConfig::default();
    for seg in &mut cfg.segments {
        seg.weight = 0;
    }
    let cfg = cfg.normalized();
    let total: u32 = cfg.segments.iter().map(|s| s.weight).sum();
    assert!(total > 0, "poids restaures depuis les defauts");
    assert_eq!(cfg.segments[0].weight, WHEEL_CASES[0].weight);
}

#[test]
fn normalized_falls_back_on_length_mismatch() {
    let cfg = WheelConfig {
        segments: vec![WheelSegment {
            payout: 1,
            weight: 1,
        }],
    }
    .normalized();
    assert_eq!(cfg, WheelConfig::default());
}

#[test]
fn spin_cfg_uses_configured_payout() {
    // Une seule case a un poids > 0 -> elle sort a coup sur, avec SON payout.
    let mut cfg = WheelConfig::default();
    for seg in &mut cfg.segments {
        seg.weight = 0;
    }
    cfg.segments[3].weight = 10;
    cfg.segments[3].payout = 777;
    let mut rng = StdRng::seed_from_u64(1);
    let outcome = spin_with_rng_cfg(&mut rng, &cfg);
    assert_eq!(outcome.case_index, 3);
    assert_eq!(outcome.case.payout, 777);
    assert_eq!(outcome.case.key, WHEEL_CASES[3].key);
}

#[test]
fn spin_cfg_does_not_panic_when_block_zeroes_all_weights() {
    // Config degeneree : seul licorne a du poids, et on la bloque.
    let mut cfg = WheelConfig::default();
    for seg in &mut cfg.segments {
        seg.weight = 0;
    }
    let licorne_idx = WHEEL_CASES.iter().position(|c| c.key == "licorne").unwrap();
    cfg.segments[licorne_idx].weight = 5;
    let mut rng = StdRng::seed_from_u64(2);
    // Ne doit pas paniquer (fallback : on ignore le blocage).
    let _ = spin_with_rng_curses_cfg(&mut rng, true, &cfg);
}

#[test]
fn heartbreak_keeps_other_cases_distribution() {
    let mut rng = StdRng::seed_from_u64(123);
    let mut blanche = 0;
    let mut bombe = 0;
    for _ in 0..10_000 {
        let outcome = spin_with_rng_curses(&mut rng, true);
        match outcome.case.key {
            "blanche" => blanche += 1,
            "bombe" => bombe += 1,
            _ => {}
        }
    }
    assert!(
        blanche > 2_000,
        "blanche (poids 25/99) doit sortir frequemment"
    );
    assert!(bombe > 100, "bombe (poids 2/99) doit sortir parfois");
}
