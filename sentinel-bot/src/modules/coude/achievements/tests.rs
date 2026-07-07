use super::*;
use crate::modules::coude::api_client::PlayerProgression;

fn progression(unlocked: &[&str]) -> PlayerProgression {
    PlayerProgression {
        unlocked_achievements: unlocked.iter().map(|s| s.to_string()).collect(),
        total_achievements: ACHIEVEMENTS.len() as i32,
        milestones: Vec::new(),
        next_milestone: None,
        effective_repos_cooldown_hours: 12,
    }
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
fn emoji_for_key_maps_known_keys() {
    assert_eq!(emoji_for_key("first_blood"), Some("\u{1fa78}"));
    assert_eq!(emoji_for_key("inconnu"), None);
}

#[test]
fn format_compact_empty_returns_friendly_message() {
    let s = format_unlocked_compact(&progression(&[]));
    assert!(s.contains("Aucun"));
}

#[test]
fn format_compact_shows_emoji_and_count() {
    let s = format_unlocked_compact(&progression(&["first_blood", "rich"]));
    assert!(s.contains("\u{1fa78}"));
    assert!(s.contains("\u{1f4b0}"));
    assert!(s.contains("/"));
    assert!(s.contains("succes"));
}
