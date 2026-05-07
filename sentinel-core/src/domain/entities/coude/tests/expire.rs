use super::*;

#[test]
fn cowardice_penalty_is_20_percent_of_mise() {
    assert_eq!(cowardice_penalty(100), 20);
    assert_eq!(cowardice_penalty(500), 100);
    assert_eq!(cowardice_penalty(1000), 200);
}

#[test]
fn cowardice_penalty_floor_is_1_coin() {
    // mise 0 ou 1 -> 0.2 ou 0.0 arrondi, mais min 1.
    assert_eq!(cowardice_penalty(0), 1);
    assert_eq!(cowardice_penalty(1), 1);
    assert_eq!(cowardice_penalty(4), 1); // 0.8 -> max(1.0) -> 1
}

#[test]
fn cowardice_penalty_at_threshold_5() {
    // 5 * 0.20 = 1.0 -> 1
    assert_eq!(cowardice_penalty(5), 1);
}

#[test]
fn cowardice_penalty_handles_large_mise() {
    assert_eq!(cowardice_penalty(1_000_000), 200_000);
}
