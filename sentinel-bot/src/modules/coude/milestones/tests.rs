use super::*;

#[test]
fn five_milestones_defined() {
    assert_eq!(MILESTONES.len(), 5);
}

#[test]
fn milestone_levels_are_5_10_15_20_25() {
    let levels: Vec<i32> = MILESTONES.iter().map(|m| m.level).collect();
    assert_eq!(levels, vec![5, 10, 15, 20, 25]);
}

#[test]
fn keys_are_distinct() {
    let mut keys: Vec<_> = MILESTONES.iter().map(|m| m.key).collect();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), MILESTONES.len());
}

#[test]
fn unlocked_at_level_1_is_empty() {
    assert!(unlocked_for(1).is_empty());
    assert!(unlocked_for(4).is_empty());
}

#[test]
fn unlocked_at_5_includes_first() {
    let u = unlocked_for(5);
    assert_eq!(u.len(), 1);
    assert_eq!(u[0].key, "extra_insurance_slot");
}

#[test]
fn unlocked_at_15_includes_three() {
    let u = unlocked_for(15);
    assert_eq!(u.len(), 3);
    assert_eq!(u[0].level, 5);
    assert_eq!(u[1].level, 10);
    assert_eq!(u[2].level, 15);
}

#[test]
fn unlocked_at_25_includes_all() {
    let u = unlocked_for(25);
    assert_eq!(u.len(), 5);
}

#[test]
fn unlocked_at_100_includes_all() {
    let u = unlocked_for(100);
    assert_eq!(u.len(), 5);
}

#[test]
fn next_for_low_level() {
    let n = next_for(1).unwrap();
    assert_eq!(n.level, 5);
}

#[test]
fn next_for_at_palier_returns_following() {
    let n = next_for(5).unwrap();
    assert_eq!(n.level, 10);
}

#[test]
fn next_for_max_returns_none() {
    assert!(next_for(25).is_none());
    assert!(next_for(100).is_none());
}

#[test]
fn format_profile_section_low_level_shows_next() {
    let s = format_profile_section(1);
    assert!(s.contains("Aucun palier"));
    assert!(s.contains("niveau **5**"));
    assert!(s.contains("Coffre"));
}

#[test]
fn format_profile_section_mid_level_shows_unlocked_and_next() {
    let s = format_profile_section(12);
    assert!(s.contains("Coffre renforce"));
    assert!(s.contains("Ultime de classe"));
    assert!(s.contains("niveau **15**"));
}

#[test]
fn format_profile_section_max_level_shows_celebration() {
    let s = format_profile_section(25);
    assert!(s.contains("Tous les paliers"));
}

#[test]
fn all_milestones_have_emoji_and_label() {
    for m in MILESTONES {
        assert!(!m.emoji.is_empty(), "{} sans emoji", m.key);
        assert!(!m.label.is_empty(), "{} sans label", m.key);
        assert!(!m.description.is_empty(), "{} sans description", m.key);
    }
}

#[test]
fn mechanical_flags_match_branched_set() {
    // A mettre a jour au fur et a mesure que les effets mecaniques
    // sont branches dans les services.
    let branched: Vec<&str> = MILESTONES
        .iter()
        .filter(|m| m.mechanical_implemented)
        .map(|m| m.key)
        .collect();
    assert_eq!(branched, vec!["repos_short_cooldown", "riposte_first"]);
}

#[test]
fn effective_repos_cooldown_low_level_returns_base() {
    assert_eq!(effective_repos_cooldown_hours(12, 1), 12);
    assert_eq!(effective_repos_cooldown_hours(12, 14), 12);
}

#[test]
fn effective_repos_cooldown_at_milestone_caps_to_8() {
    assert_eq!(effective_repos_cooldown_hours(12, 15), 8);
    assert_eq!(effective_repos_cooldown_hours(12, 25), 8);
    assert_eq!(effective_repos_cooldown_hours(24, 15), 8);
}

#[test]
fn effective_repos_cooldown_already_short_unchanged() {
    // Si le serveur configure deja un cooldown <= 8h, on ne le rallonge pas.
    assert_eq!(effective_repos_cooldown_hours(6, 15), 6);
    assert_eq!(effective_repos_cooldown_hours(8, 15), 8);
}
