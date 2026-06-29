use super::*;

#[test]
fn leaderboard_category_parse_all_valid() {
    assert_eq!(
        LeaderboardCategory::parse("richest"),
        Some(LeaderboardCategory::Richest)
    );
    assert_eq!(
        LeaderboardCategory::parse("thieves"),
        Some(LeaderboardCategory::Thieves)
    );
    assert_eq!(
        LeaderboardCategory::parse("cowards"),
        Some(LeaderboardCategory::Cowards)
    );
    assert_eq!(
        LeaderboardCategory::parse("chaos"),
        Some(LeaderboardCategory::Chaos)
    );
    assert_eq!(
        LeaderboardCategory::parse("level"),
        Some(LeaderboardCategory::Level)
    );
}

#[test]
fn leaderboard_category_parse_invalid_returns_none() {
    assert_eq!(LeaderboardCategory::parse(""), None);
    assert_eq!(LeaderboardCategory::parse("RICHEST"), None); // case-sensitive
    assert_eq!(LeaderboardCategory::parse("unknown"), None);
    assert_eq!(LeaderboardCategory::parse("rich"), None);
}

// ── daily_chaos_amount ──

#[test]
fn daily_chaos_amount_returns_floor_when_above_1() {
    assert_eq!(daily_chaos_amount(1000, 0.20), Some(200));
    assert_eq!(daily_chaos_amount(50, 0.20), Some(10));
    assert_eq!(daily_chaos_amount(11, 0.20), Some(2)); // 2.2 -> 2
}

#[test]
fn daily_chaos_amount_floor_below_1_returns_none() {
    // 4 * 0.20 = 0.8 -> floor 0 -> None
    assert!(daily_chaos_amount(4, 0.20).is_none());
    assert!(daily_chaos_amount(0, 0.20).is_none());
}

#[test]
fn daily_chaos_amount_at_boundary_5_returns_some_1() {
    // 5 * 0.20 = 1.0 -> Some(1)
    assert_eq!(daily_chaos_amount(5, 0.20), Some(1));
}

#[test]
fn daily_chaos_amount_supports_custom_percent() {
    assert_eq!(daily_chaos_amount(100, 0.50), Some(50));
    assert_eq!(daily_chaos_amount(100, 0.01), Some(1));
}

// ── clamp_leaderboard_limit ──

#[test]
fn clamp_leaderboard_in_range_passes_through() {
    assert_eq!(clamp_leaderboard_limit(10), 10);
    assert_eq!(clamp_leaderboard_limit(50), 50);
}

#[test]
fn clamp_leaderboard_below_1_clamped_to_1() {
    assert_eq!(clamp_leaderboard_limit(0), 1);
    assert_eq!(clamp_leaderboard_limit(-5), 1);
}

#[test]
fn clamp_leaderboard_above_100_clamped_to_100() {
    assert_eq!(clamp_leaderboard_limit(101), 100);
    assert_eq!(clamp_leaderboard_limit(10_000), 100);
}

// ── constants ──

#[test]
fn constants_match_business_spec() {
    assert_eq!(DAILY_CHAOS_MAX, 5);
    assert_eq!(DEFAULT_CHAOS_PERCENT, 0.20);
    assert_eq!(MIN_COINS_ELIGIBLE, 10);
    assert_eq!(LEADERBOARD_MIN_LIMIT, 1);
    assert_eq!(LEADERBOARD_MAX_LIMIT, 100);
}
