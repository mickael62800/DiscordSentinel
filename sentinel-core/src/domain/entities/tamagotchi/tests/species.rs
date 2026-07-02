//! Tests des especes : roundtrip str, exhaustivite ALL, affinites distinctes.

use super::*;

#[test]
fn from_str_roundtrip_all() {
    for s in Species::ALL {
        assert_eq!(Species::from_str(s.as_str()), Some(s));
    }
}

#[test]
fn from_str_unknown_is_none() {
    assert_eq!(Species::from_str("dragon"), None);
    assert_eq!(Species::from_str(""), None);
    assert_eq!(Species::from_str("Loup"), None); // sensible a la casse
}

#[test]
fn all_contains_six_distinct() {
    assert_eq!(Species::ALL.len(), 6);
    // Toutes les chaines sont distinctes.
    let mut names: Vec<&str> = Species::ALL.iter().map(|s| s.as_str()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 6);
}

#[test]
fn display_non_empty_for_all() {
    for s in Species::ALL {
        assert!(!s.display().is_empty());
    }
}

#[test]
fn base_stats_are_positive() {
    for s in Species::ALL {
        let b = s.base_stats();
        assert!(b.str_ > 0 && b.vit > 0 && b.agi > 0, "{}", s.as_str());
    }
}

#[test]
fn base_stats_affinities_match_identity() {
    // Le sanglier est le plus fort en FORCE parmi toutes les especes.
    let strongest = Species::ALL
        .iter()
        .max_by_key(|s| s.base_stats().str_)
        .unwrap();
    assert_eq!(*strongest, Species::Sanglier);
    // La tortue a la meilleure VITALITE.
    let tankiest = Species::ALL
        .iter()
        .max_by_key(|s| s.base_stats().vit)
        .unwrap();
    assert_eq!(*tankiest, Species::Tortue);
    // Renard et Lapin sont les plus AGILES (16).
    assert_eq!(Species::Renard.base_stats().agi, 16);
    assert_eq!(Species::Lapin.base_stats().agi, 16);
}
