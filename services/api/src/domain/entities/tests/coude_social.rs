use super::*;

#[test]
fn leaderboard_category_parse_all_valid() {
    assert_eq!(LeaderboardCategory::parse("richest"), Some(LeaderboardCategory::Richest));
    assert_eq!(LeaderboardCategory::parse("thieves"), Some(LeaderboardCategory::Thieves));
    assert_eq!(LeaderboardCategory::parse("cowards"), Some(LeaderboardCategory::Cowards));
    assert_eq!(LeaderboardCategory::parse("chaos"), Some(LeaderboardCategory::Chaos));
    assert_eq!(LeaderboardCategory::parse("level"), Some(LeaderboardCategory::Level));
}

#[test]
fn leaderboard_category_parse_invalid_returns_none() {
    assert_eq!(LeaderboardCategory::parse(""), None);
    assert_eq!(LeaderboardCategory::parse("RICHEST"), None); // case-sensitive
    assert_eq!(LeaderboardCategory::parse("unknown"), None);
    assert_eq!(LeaderboardCategory::parse("rich"), None);
}
