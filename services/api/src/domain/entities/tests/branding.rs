use super::*;

#[test]
fn taglines_are_non_empty() {
    assert!(!COUDE_TAGLINE.is_empty());
    assert!(!COUDE_TAGLINE_SHORT.is_empty());
    assert!(!SENTINEL_TAGLINE.is_empty());
}

#[test]
fn taglines_are_distinct() {
    assert_ne!(COUDE_TAGLINE, COUDE_TAGLINE_SHORT);
    assert_ne!(COUDE_TAGLINE, SENTINEL_TAGLINE);
}

#[test]
fn coude_tagline_mentions_chaos() {
    assert!(COUDE_TAGLINE.to_lowercase().contains("chaos"));
    assert!(COUDE_TAGLINE_SHORT.to_lowercase().contains("chaos"));
}

#[test]
fn combat_footer_includes_round_count_singular() {
    let s = coude_combat_footer(1);
    assert!(s.contains("1 round"));
    assert!(!s.contains("rounds"));
}

#[test]
fn combat_footer_includes_round_count_plural() {
    let s = coude_combat_footer(7);
    assert!(s.contains("7 rounds"));
}

#[test]
fn combat_footer_zero_rounds_uses_singular() {
    // Edge case : 0 round (theoriquement impossible, mais robuste).
    let s = coude_combat_footer(0);
    assert!(s.contains("0 round"));
    assert!(!s.contains("rounds"));
}

#[test]
fn combat_footer_includes_tagline_short() {
    let s = coude_combat_footer(3);
    assert!(s.contains("chaos"));
}

#[test]
fn bet_footer_includes_pot() {
    let s = coude_bet_footer(12345);
    assert!(s.contains("12345"));
    assert!(s.contains("Pot total"));
}

#[test]
fn bet_footer_includes_tagline() {
    let s = coude_bet_footer(100);
    assert!(s.contains("chaos"));
}

#[test]
fn bet_footer_zero_pot() {
    let s = coude_bet_footer(0);
    assert!(s.contains("0"));
}
