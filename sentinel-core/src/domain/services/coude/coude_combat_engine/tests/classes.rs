use super::*;

#[test]
fn get_class_returns_correct_class_for_each_name() {
    assert_eq!(get_class("bourrin").name, "bourrin");
    assert_eq!(get_class("agile").name, "agile");
    assert_eq!(get_class("fourbe").name, "fourbe");
    assert_eq!(get_class("tank").name, "tank");
}

#[test]
fn get_class_unknown_defaults_to_bourrin() {
    assert_eq!(get_class("").name, "bourrin");
    assert_eq!(get_class("ninja").name, "bourrin");
    assert_eq!(get_class("BOURRIN").name, "bourrin"); // case-sensitive
}

#[test]
fn is_valid_class_accepts_only_four_classes() {
    assert!(is_valid_class("bourrin"));
    assert!(is_valid_class("agile"));
    assert!(is_valid_class("fourbe"));
    assert!(is_valid_class("tank"));
    assert!(!is_valid_class(""));
    assert!(!is_valid_class("unknown"));
    assert!(!is_valid_class("BOURRIN"));
}

#[test]
fn all_classes_contains_four() {
    assert_eq!(ALL_CLASSES.len(), 4);
    let names: Vec<&str> = ALL_CLASSES.iter().map(|c| c.name).collect();
    assert!(names.contains(&"bourrin"));
    assert!(names.contains(&"agile"));
    assert!(names.contains(&"fourbe"));
    assert!(names.contains(&"tank"));
}

#[test]
fn bourrin_has_high_atk_low_def() {
    let c = &CLASS_BOURRIN;
    assert!(c.base_atk > c.base_def, "bourrin should have ATK > DEF");
    assert_eq!(c.dodge_chance, 0.0);
    assert_eq!(c.steal_bonus, 0.0);
}

#[test]
fn tank_has_high_def_low_atk() {
    let c = &CLASS_TANK;
    assert!(c.base_def > c.base_atk, "tank should have DEF > ATK");
    assert_eq!(c.dodge_chance, 0.0);
}

#[test]
fn agile_has_dodge_chance() {
    assert!(CLASS_AGILE.dodge_chance > 0.0);
    assert!(CLASS_AGILE.dodge_chance <= 1.0);
}

#[test]
fn fourbe_has_steal_bonus() {
    assert!(CLASS_FOURBE.steal_bonus > 0.0);
    assert!(CLASS_FOURBE.steal_bonus <= 1.0);
}

#[test]
fn all_classes_have_non_empty_metadata() {
    for c in ALL_CLASSES {
        assert!(!c.name.is_empty(), "class name empty");
        assert!(!c.emoji.is_empty(), "class emoji empty");
        assert!(!c.description.is_empty(), "class description empty");
        assert!(!c.passif_key.is_empty(), "passif_key empty");
        assert!(!c.passif_description.is_empty(), "passif_description empty");
        assert!(!c.passif_reveal.is_empty(), "passif_reveal empty");
    }
}

#[test]
fn all_classes_have_unique_names_and_passifs() {
    let names: std::collections::HashSet<&str> = ALL_CLASSES.iter().map(|c| c.name).collect();
    assert_eq!(names.len(), ALL_CLASSES.len(), "duplicate class names");

    let passifs: std::collections::HashSet<&str> = ALL_CLASSES.iter().map(|c| c.passif_key).collect();
    assert_eq!(passifs.len(), ALL_CLASSES.len(), "duplicate passif_key");
}

#[test]
fn all_classes_have_positive_base_stats() {
    for c in ALL_CLASSES {
        assert!(c.base_atk > 0, "{} base_atk must be > 0", c.name);
        assert!(c.base_def > 0, "{} base_def must be > 0", c.name);
        assert!(c.atk_growth >= 0, "{} atk_growth must be >= 0", c.name);
        assert!(c.def_growth >= 0, "{} def_growth must be >= 0", c.name);
    }
}

#[test]
fn passif_reveal_contains_player_placeholder() {
    // Les messages de révélation doivent inclure {joueur} pour substitution.
    for c in ALL_CLASSES {
        assert!(c.passif_reveal.contains("{joueur}"), "{} passif_reveal missing {{joueur}}", c.name);
    }
}
