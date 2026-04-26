use super::*;

#[test]
fn four_ultimates_one_per_class() {
    assert_eq!(CLASS_ULTIMATES.len(), 4);
    let classes: Vec<&str> = CLASS_ULTIMATES.iter().map(|u| u.class_key).collect();
    let expected = ["bourrin", "agile", "fourbe", "tank"];
    for k in expected {
        assert!(classes.contains(&k), "manque l ultimate {k}");
    }
}

#[test]
fn unlock_level_is_10() {
    assert_eq!(ULTIMATE_UNLOCK_LEVEL, 10);
}

#[test]
fn lookup_known_class() {
    let u = ultimate_for_class("bourrin").unwrap();
    assert_eq!(u.label, "Echange de carcasses");
    assert_eq!(u.cooldown_days, 7);
}

#[test]
fn lookup_unknown_returns_none() {
    assert!(ultimate_for_class("foo").is_none());
    assert!(ultimate_for_class("").is_none());
}

#[test]
fn fourbe_has_extended_cooldown() {
    let u = ultimate_for_class("fourbe").unwrap();
    assert_eq!(u.cooldown_days, 14);
}

#[test]
fn format_below_unlock_level_shows_locked() {
    let s = format_ultimate_for_class("bourrin", 9);
    assert!(s.contains("Verrouille"));
    assert!(s.contains("niveau 10"));
}

#[test]
fn format_at_unlock_level_shows_description() {
    let s = format_ultimate_for_class("bourrin", 10);
    assert!(s.contains("carcasses"));
    assert!(s.contains("cooldown"));
}

#[test]
fn format_unknown_class_returns_friendly() {
    let s = format_ultimate_for_class("foo", 25);
    assert!(s.contains("pas d ultimate"));
}

#[test]
fn all_ultimates_have_emoji_label_description() {
    for u in CLASS_ULTIMATES {
        assert!(!u.emoji.is_empty(), "{} sans emoji", u.class_key);
        assert!(!u.label.is_empty(), "{} sans label", u.class_key);
        assert!(!u.description.is_empty(), "{} sans description", u.class_key);
    }
}

#[test]
fn mechanical_flags_match_branched_set() {
    let branched: Vec<&str> = CLASS_ULTIMATES
        .iter()
        .filter(|u| u.mechanical_implemented)
        .map(|u| u.class_key)
        .collect();
    // Bourrin / Agile / Tank branches ; Fourbe restant declaratif.
    assert_eq!(branched, vec!["bourrin", "agile", "tank"]);
}
