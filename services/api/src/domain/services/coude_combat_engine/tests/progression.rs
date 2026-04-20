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
