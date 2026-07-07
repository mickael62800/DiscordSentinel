use super::*;
use crate::modules::coude::api_client::{MilestoneInfo, PlayerProgression};

fn mi(level: i32, label: &str, unlocked: bool) -> MilestoneInfo {
    MilestoneInfo {
        level,
        key: format!("m{level}"),
        label: label.into(),
        emoji: "\u{1f6e1}\u{fe0f}".into(),
        description: format!("desc {level}"),
        unlocked,
    }
}

fn progression(milestones: Vec<MilestoneInfo>, next: Option<MilestoneInfo>) -> PlayerProgression {
    PlayerProgression {
        unlocked_achievements: Vec::new(),
        total_achievements: 32,
        milestones,
        next_milestone: next,
        effective_repos_cooldown_hours: 12,
    }
}

#[test]
fn low_level_shows_next() {
    let p = progression(
        vec![mi(5, "Coffre renforce", false)],
        Some(mi(5, "Coffre renforce", false)),
    );
    let s = format_profile_section(&p);
    assert!(s.contains("Aucun palier"));
    assert!(s.contains("niveau **5**"));
    assert!(s.contains("Coffre"));
}

#[test]
fn mid_level_shows_unlocked_and_next() {
    let p = progression(
        vec![
            mi(5, "Coffre renforce", true),
            mi(10, "Ultime de classe", true),
            mi(15, "Convalescence", false),
        ],
        Some(mi(15, "Convalescence", false)),
    );
    let s = format_profile_section(&p);
    assert!(s.contains("Coffre renforce"));
    assert!(s.contains("Ultime de classe"));
    assert!(s.contains("niveau **15**"));
}

#[test]
fn max_level_shows_celebration() {
    let p = progression(vec![mi(25, "Prestige", true)], None);
    let s = format_profile_section(&p);
    assert!(s.contains("Tous les paliers"));
}
