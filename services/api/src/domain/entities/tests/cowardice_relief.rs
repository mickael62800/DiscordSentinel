use super::*;

#[test]
fn full_hp_counts_as_cowardice() {
    assert!(should_count_as_cowardice(200, 200));
}

#[test]
fn half_hp_counts_as_cowardice() {
    assert!(should_count_as_cowardice(100, 200));
}

#[test]
fn exactly_20_pct_is_relief() {
    // 20% pile = relief (le defenseur est bas, on est pas mesquin).
    assert!(!should_count_as_cowardice(40, 200));
}

#[test]
fn below_20_pct_is_legitimate_refusal() {
    assert!(!should_count_as_cowardice(39, 200));
    assert!(!should_count_as_cowardice(10, 200));
    assert!(!should_count_as_cowardice(1, 200));
}

#[test]
fn zero_hp_is_legitimate_refusal() {
    assert!(!should_count_as_cowardice(0, 200));
}

#[test]
fn defensive_hp_max_zero_falls_back_to_count() {
    // hp_max=0 ne devrait pas arriver, mais si oui on garde le compteur
    assert!(should_count_as_cowardice(0, 0));
    assert!(should_count_as_cowardice(50, 0));
}

#[test]
fn defensive_negative_hp_treated_as_zero() {
    // hp negatif (cas absurde) : pct clampe a 0 -> relief
    assert!(!should_count_as_cowardice(-5, 200));
}

#[test]
fn defensive_hp_above_max_clamped_to_full() {
    // hp > hp_max (overflow buff) : pct clampe a 1 -> compte
    assert!(should_count_as_cowardice(500, 200));
}

#[test]
fn threshold_constant_is_20_percent() {
    assert_eq!(COWARDICE_RELIEF_HP_PCT, 0.20);
}
