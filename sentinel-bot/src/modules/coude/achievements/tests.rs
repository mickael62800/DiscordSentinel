use super::*;
use crate::modules::coude::api_client::Player;

fn empty_player() -> Player {
    Player {
        guild_id: "g".into(),
        user_id: "u".into(),
        username: "x".into(),
        coins: 0,
        total_wins: 0,
        total_losses: 0,
        total_draws: 0,
        total_earned: 0,
        total_lost: 0,
        total_stolen: 0,
        cowardice_count: 0,
        chaos_events: 0,
        casino_wins: 0,
        casino_losses: 0,
        level: 1,
        xp: 0,
        stat_points: 0,
        atk: 0,
        def: 0,
        class: None,
        title: None,
        class_changed_at: None,
        hp_current: None,
        hp_max: None,
        hp_last_regen: None,
        repos_last_used: None,
        season: None,
        created_at: String::new(),
        updated_at: String::new(),
    }
}

#[test]
fn fresh_player_has_no_achievements() {
    let p = empty_player();
    assert!(unlocked_for(&p).is_empty());
}

#[test]
fn first_blood_unlocks_at_first_win() {
    let mut p = empty_player();
    p.total_wins = 1;
    let unlocked: Vec<&str> = unlocked_for(&p).iter().map(|a| a.key).collect();
    assert!(unlocked.contains(&"first_blood"));
    assert!(!unlocked.contains(&"veteran"), "10 wins requis");
}

#[test]
fn legend_requires_100_wins() {
    let mut p = empty_player();
    p.total_wins = 100;
    let unlocked: Vec<&str> = unlocked_for(&p).iter().map(|a| a.key).collect();
    assert!(unlocked.contains(&"legend"));
    assert!(unlocked.contains(&"butcher"));
    assert!(unlocked.contains(&"veteran"));
    assert!(unlocked.contains(&"first_blood"));
}

#[test]
fn millionaire_at_100k() {
    let mut p = empty_player();
    p.coins = 100_000;
    let unlocked: Vec<&str> = unlocked_for(&p).iter().map(|a| a.key).collect();
    assert!(unlocked.contains(&"millionaire"));
    assert!(unlocked.contains(&"rich"));
    assert!(!unlocked.contains(&"magnate"));
}

#[test]
fn no_quarter_requires_no_draw() {
    let mut p = empty_player();
    p.total_wins = 20;
    p.total_draws = 0;
    assert!(unlocked_for(&p).iter().any(|a| a.key == "no_quarter"));

    p.total_draws = 1;
    assert!(!unlocked_for(&p).iter().any(|a| a.key == "no_quarter"));
}

#[test]
fn specialist_requires_class() {
    let mut p = empty_player();
    p.class = None;
    assert!(!unlocked_for(&p).iter().any(|a| a.key == "specialist"));
    p.class = Some("".into());
    assert!(!unlocked_for(&p).iter().any(|a| a.key == "specialist"));
    p.class = Some("bourrin".into());
    assert!(unlocked_for(&p).iter().any(|a| a.key == "specialist"));
}

#[test]
fn at_least_30_achievements() {
    assert!(ACHIEVEMENTS.len() >= 30, "got {}", ACHIEVEMENTS.len());
}

#[test]
fn all_keys_distinct() {
    let mut keys: Vec<_> = ACHIEVEMENTS.iter().map(|a| a.key).collect();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), ACHIEVEMENTS.len());
}

#[test]
fn all_have_emoji_and_label() {
    for a in ACHIEVEMENTS {
        assert!(!a.emoji.is_empty(), "{} sans emoji", a.key);
        assert!(!a.label.is_empty(), "{} sans label", a.key);
        assert!(!a.description.is_empty(), "{} sans description", a.key);
    }
}

#[test]
fn format_compact_empty_returns_friendly_message() {
    let p = empty_player();
    let s = format_unlocked_compact(&p);
    assert!(s.contains("Aucun"));
}

#[test]
fn format_compact_shows_emoji_and_count() {
    let mut p = empty_player();
    p.total_wins = 1;
    p.coins = 10_000;
    let s = format_unlocked_compact(&p);
    assert!(s.contains("🩸"));
    assert!(s.contains("💰"));
    assert!(s.contains("/"));
    assert!(s.contains("succes"));
}
