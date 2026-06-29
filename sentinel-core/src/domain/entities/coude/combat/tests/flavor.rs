use super::*;
use rand::rngs::StdRng;
use rand::SeedableRng;

#[test]
fn returns_none_when_proba_above_threshold() {
    let mut rng = StdRng::seed_from_u64(0);
    assert!(pick_flavor_line(&mut rng, 0.20, "A", "B").is_none());
    assert!(pick_flavor_line(&mut rng, 0.99, "A", "B").is_none());
}

#[test]
fn returns_some_when_proba_below_threshold() {
    let mut rng = StdRng::seed_from_u64(0);
    assert!(pick_flavor_line(&mut rng, 0.0, "A", "B").is_some());
    assert!(pick_flavor_line(&mut rng, 0.19, "A", "B").is_some());
}

#[test]
fn substitutes_attacker_name() {
    let mut rng = StdRng::seed_from_u64(0);
    for _ in 0..50 {
        if let Some(line) = pick_flavor_line(&mut rng, 0.0, "Alice", "Bob") {
            assert!(!line.contains("{atk}"));
            assert!(!line.contains("{def}"));
            // Si le template original mentionnait {atk}, "Alice" doit y etre.
        }
    }
}

#[test]
fn at_least_20_lines_in_catalog() {
    assert!(
        FLAVOR_LINES.len() >= 20,
        "ambition : >= 20 lignes pour la rejouabilite"
    );
}

#[test]
fn no_template_contains_unbalanced_placeholders() {
    for tmpl in FLAVOR_LINES {
        // Pas de placeholders inconnus type {foo}.
        let cleaned = tmpl.replace("{atk}", "").replace("{def}", "");
        assert!(
            !cleaned.contains('{'),
            "template avec placeholder inconnu : {tmpl}"
        );
        assert!(
            !cleaned.contains('}'),
            "template avec placeholder inconnu : {tmpl}"
        );
    }
}

#[test]
fn distribution_picks_different_lines() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for _ in 0..500 {
        if let Some(line) = pick_flavor_line(&mut rng, 0.0, "A", "B") {
            seen.insert(line);
        }
    }
    assert!(
        seen.len() >= 10,
        "doit varier sur 500 tirages forced (got {})",
        seen.len()
    );
}

#[test]
fn probability_constant_is_20_percent() {
    assert_eq!(FLAVOR_LINE_PROBABILITY, 0.20);
}
