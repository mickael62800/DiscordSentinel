use super::*;

#[test]
fn default_for_guild_sets_defaults() {
    let c = StrikeConfig::default_for_guild("g1");
    assert_eq!(c.guild_id, "g1");
    assert_eq!(c.window_secs, 3600);
    assert!(c.thresholds.is_empty());
    assert!(c.enabled);
}

#[test]
fn default_for_guild_copies_guild_id() {
    let c = StrikeConfig::default_for_guild("my-server");
    assert_eq!(c.guild_id, "my-server");
}

#[test]
fn default_created_at_matches_updated_at() {
    let c = StrikeConfig::default_for_guild("g");
    assert_eq!(c.created_at, c.updated_at);
}
