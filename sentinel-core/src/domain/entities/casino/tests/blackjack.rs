use super::*;

fn card(rank: &str, suit: &str) -> Card {
    Card {
        rank: rank.into(),
        suit: suit.into(),
    }
}

#[test]
fn card_value_numeric() {
    assert_eq!(card("2", "hearts").value(), 2);
    assert_eq!(card("7", "clubs").value(), 7);
    assert_eq!(card("10", "spades").value(), 10);
}

#[test]
fn card_value_faces_are_10() {
    assert_eq!(card("Jack", "hearts").value(), 10);
    assert_eq!(card("Queen", "diamonds").value(), 10);
    assert_eq!(card("King", "clubs").value(), 10);
}

#[test]
fn card_value_as_is_11() {
    assert_eq!(card("As", "spades").value(), 11);
}

#[test]
fn card_value_unknown_rank_is_zero() {
    assert_eq!(card("Joker", "hearts").value(), 0);
    assert_eq!(card("", "hearts").value(), 0);
}

#[test]
fn card_filename_format() {
    assert_eq!(card("As", "heart").filename(), "As_heart.jpg");
    assert_eq!(card("10", "club").filename(), "10_club.jpg");
    assert_eq!(card("Jack", "diamond").filename(), "Jack_diamond.jpg");
}

// ── calculate_score ──

#[test]
fn score_empty_hand_zero() {
    assert_eq!(calculate_score(&[]), 0);
}

#[test]
fn score_single_card() {
    assert_eq!(calculate_score(&[card("7", "hearts")]), 7);
    assert_eq!(calculate_score(&[card("King", "hearts")]), 10);
    assert_eq!(calculate_score(&[card("As", "hearts")]), 11);
}

#[test]
fn score_pair_sum() {
    assert_eq!(calculate_score(&[card("5", "h"), card("6", "d")]), 11);
}

#[test]
fn score_blackjack_natural_21() {
    assert_eq!(calculate_score(&[card("As", "h"), card("King", "d")]), 21);
    assert_eq!(calculate_score(&[card("As", "h"), card("Jack", "d")]), 21);
}

#[test]
fn score_two_aces_reduces_one_to_avoid_bust() {
    // As+As = 22, un As devient 1 → 12
    assert_eq!(calculate_score(&[card("As", "h"), card("As", "d")]), 12);
}

#[test]
fn score_three_aces_reduces_as_many_as_needed() {
    // As+As+As = 33 → 23 → 13 (on garde un As a 11)
    assert_eq!(
        calculate_score(&[card("As", "h"), card("As", "d"), card("As", "c")]),
        13
    );
}

#[test]
fn score_as_reduces_when_bust_with_face() {
    // As + King + 5 = 11+10+5 = 26 → As devient 1 → 16
    assert_eq!(
        calculate_score(&[card("As", "h"), card("King", "d"), card("5", "c")]),
        16
    );
}

#[test]
fn score_keeps_as_at_11_when_safe() {
    // As + 9 = 20, pas de bust, garde 11
    assert_eq!(calculate_score(&[card("As", "h"), card("9", "d")]), 20);
}

#[test]
fn score_bust_hand_above_21() {
    // 10 + 10 + 5 = 25 (pas d'As pour reduire)
    assert_eq!(
        calculate_score(&[card("10", "h"), card("King", "d"), card("5", "c")]),
        25
    );
}

#[test]
fn score_four_aces() {
    // 4 As = 44 → -10 = 34 → -10 = 24 → -10 = 14
    assert_eq!(
        calculate_score(&[
            card("As", "h"),
            card("As", "d"),
            card("As", "c"),
            card("As", "s"),
        ]),
        14
    );
}

// ── create_deck ──

#[test]
fn deck_has_52_cards() {
    let deck = create_deck();
    assert_eq!(deck.len(), 52);
}

#[test]
fn deck_has_13_ranks_per_suit() {
    let deck = create_deck();
    for suit in ["hearts", "diamonds", "clubs", "spades"] {
        let count = deck.iter().filter(|c| c.suit == suit).count();
        assert_eq!(count, 13, "suit {} should have 13 cards", suit);
    }
}

#[test]
fn deck_has_4_of_each_rank() {
    let deck = create_deck();
    for rank in [
        "2", "3", "4", "5", "6", "7", "8", "9", "10", "Jack", "Queen", "King", "As",
    ] {
        let count = deck.iter().filter(|c| c.rank == rank).count();
        assert_eq!(count, 4, "rank {} should have 4 cards", rank);
    }
}

#[test]
fn deck_shuffled_across_multiple_calls() {
    // Deux decks de suite doivent difficilement etre identiques (proba ~1/52!).
    let d1 = create_deck();
    let d2 = create_deck();
    let same: bool = d1
        .iter()
        .zip(d2.iter())
        .all(|(a, b)| a.rank == b.rank && a.suit == b.suit);
    assert!(!same, "two shuffled decks should not be identical");
}

// ── BlackjackConfig ──

#[test]
fn config_default_sane_values() {
    let c = BlackjackConfig::default();
    assert_eq!(c.min_bet, 10);
    assert_eq!(c.max_bet, 1000);
    assert_eq!(c.starting_coins, 200);
    assert_eq!(c.blackjack_payout, 1.5);
    assert!(c.min_bet < c.max_bet);
}

#[test]
fn config_from_kv_pairs_empty_returns_default() {
    let c = BlackjackConfig::from_kv_pairs(&[]);
    assert_eq!(c, BlackjackConfig::default());
}

#[test]
fn config_from_kv_pairs_parses_known_keys() {
    let pairs = vec![
        ("min_bet".into(), "20".into()),
        ("max_bet".into(), "5000".into()),
        ("starting_coins".into(), "500".into()),
        ("blackjack_payout".into(), "2.0".into()),
    ];
    let c = BlackjackConfig::from_kv_pairs(&pairs);
    assert_eq!(c.min_bet, 20);
    assert_eq!(c.max_bet, 5000);
    assert_eq!(c.starting_coins, 500);
    assert_eq!(c.blackjack_payout, 2.0);
}

#[test]
fn config_ignores_invalid_parse() {
    let pairs = vec![
        ("min_bet".into(), "abc".into()),
        ("max_bet".into(), "".into()),
    ];
    let c = BlackjackConfig::from_kv_pairs(&pairs);
    assert_eq!(c, BlackjackConfig::default());
}

#[test]
fn config_rejects_non_positive_bets() {
    // min_bet = 0 ou negatif → ignore (garde default).
    let pairs = vec![
        ("min_bet".into(), "0".into()),
        ("max_bet".into(), "-10".into()),
    ];
    let c = BlackjackConfig::from_kv_pairs(&pairs);
    assert_eq!(c.min_bet, 10); // default
    assert_eq!(c.max_bet, 1000); // default
}

#[test]
fn config_rejects_non_positive_payout() {
    let pairs = vec![("blackjack_payout".into(), "0".into())];
    let c = BlackjackConfig::from_kv_pairs(&pairs);
    assert_eq!(c.blackjack_payout, 1.5); // default
}

#[test]
fn config_accepts_zero_starting_coins() {
    let pairs = vec![("starting_coins".into(), "0".into())];
    let c = BlackjackConfig::from_kv_pairs(&pairs);
    assert_eq!(c.starting_coins, 0);
}

#[test]
fn config_rejects_negative_starting_coins() {
    let pairs = vec![("starting_coins".into(), "-100".into())];
    let c = BlackjackConfig::from_kv_pairs(&pairs);
    assert_eq!(c.starting_coins, 200); // default
}

#[test]
fn config_fallback_defaults_when_min_exceeds_max() {
    // min_bet=500, max_bet=100 → incoherent → reset des deux aux defauts.
    let pairs = vec![
        ("min_bet".into(), "500".into()),
        ("max_bet".into(), "100".into()),
    ];
    let c = BlackjackConfig::from_kv_pairs(&pairs);
    assert_eq!(c.min_bet, 10);
    assert_eq!(c.max_bet, 1000);
}

#[test]
fn config_ignores_unknown_keys() {
    let pairs = vec![
        ("unknown_key".into(), "42".into()),
        ("min_bet".into(), "25".into()),
    ];
    let c = BlackjackConfig::from_kv_pairs(&pairs);
    assert_eq!(c.min_bet, 25);
    assert_eq!(c.max_bet, 1000);
}

#[test]
fn deck_total_value_matches_expected_sum() {
    // Sans reduction des As : 4 * (2+3+4+5+6+7+8+9+10+10+10+10+11) = 4 * 95 = 380
    let deck = create_deck();
    let total: i32 = deck.iter().map(|c| c.value()).sum();
    assert_eq!(total, 380);
}

#[test]
fn default_blackjack_max_players_is_seven() {
    // Regle metier : une table blackjack accueille par defaut 7 joueurs max.
    assert_eq!(DEFAULT_BLACKJACK_MAX_PLAYERS, 7);
}

#[test]
fn final_statuses_has_six_entries() {
    assert_eq!(BLACKJACK_FINAL_STATUSES.len(), 6);
}

#[test]
fn game_over_accepts_all_final_statuses() {
    for s in BLACKJACK_FINAL_STATUSES {
        assert!(is_blackjack_game_over(s), "expected {} to be final", s);
    }
}

#[test]
fn game_over_rejects_in_progress_statuses() {
    assert!(!is_blackjack_game_over("playing"));
    assert!(!is_blackjack_game_over("waiting"));
    assert!(!is_blackjack_game_over(""));
    assert!(!is_blackjack_game_over("PLAYER_WIN")); // case-sensitive
}

#[test]
fn shoe_has_six_decks_of_standard_52() {
    assert_eq!(BLACKJACK_SHOE_DECKS, 6);
    assert_eq!(BLACKJACK_SHOE_TOTAL_CARDS, 312);
    assert_eq!(BLACKJACK_SHOE_TOTAL_CARDS, BLACKJACK_SHOE_DECKS * 52);
}
