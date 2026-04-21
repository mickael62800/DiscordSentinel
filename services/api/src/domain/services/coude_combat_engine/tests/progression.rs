use super::*;

#[test]
fn test_matchmaking_handicap() {
    assert_eq!(matchmaking_handicap(5, 5), (1.0, false));
    assert_eq!(matchmaking_handicap(5, 3), (1.0, false));
    assert_eq!(matchmaking_handicap(8, 5), (0.8, false));
    assert_eq!(matchmaking_handicap(10, 5), (0.8, false));
    assert_eq!(matchmaking_handicap(12, 5), (0.6, false));
    assert_eq!(matchmaking_handicap(20, 5), (0.0, true));
}

#[test]
fn title_for_level_all_ranges() {
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
    // Fallback `_ => "Debutant"` : hors plage.
    assert_eq!(title_for_level(0), "Debutant");
    assert_eq!(title_for_level(26), "Debutant");
    assert_eq!(title_for_level(-5), "Debutant");
    assert_eq!(title_for_level(i32::MAX), "Debutant");
    assert_eq!(title_for_level(i32::MIN), "Debutant");
}

#[test]
fn xp_for_level_formula_monotonic() {
    assert_eq!(xp_for_level(0), 0);
    assert_eq!(xp_for_level(1), 100); // 50 + 50
    assert_eq!(xp_for_level(10), 5500);
    assert_eq!(xp_for_level(25), 32500);
    let mut prev = 0_i64;
    for n in 1..=25 {
        let cur = xp_for_level(n);
        assert!(cur > prev);
        prev = cur;
    }
}

#[test]
fn max_level_constant() {
    assert_eq!(MAX_LEVEL, 25);
}
