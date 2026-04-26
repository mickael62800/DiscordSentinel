use super::*;
use rand::rngs::StdRng;
use rand::SeedableRng;

#[test]
fn ten_mythic_events_defined() {
    assert_eq!(MYTHIC_EVENTS.len(), 10);
}

#[test]
fn all_keys_distinct() {
    let mut keys: Vec<_> = MYTHIC_EVENTS.iter().map(|e| e.key).collect();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), MYTHIC_EVENTS.len());
}

#[test]
fn probabilities_sum_below_5_percent() {
    let total: f64 = MYTHIC_EVENTS.iter().map(|e| e.probability).sum();
    assert!(total < 0.05, "somme des proba = {total}, doit rester < 5%");
    assert!(total > 0.005, "somme des proba = {total}, trop bas (event jamais)");
}

#[test]
fn no_event_with_zero_probability() {
    for e in MYTHIC_EVENTS {
        assert!(e.probability > 0.0, "{} a une proba 0", e.key);
    }
}

#[test]
fn all_announces_have_emoji() {
    for e in MYTHIC_EVENTS {
        assert!(!e.emoji.is_empty(), "{} sans emoji", e.key);
        assert!(e.announce.contains(e.emoji), "{} : announce ne contient pas son emoji", e.key);
    }
}

#[test]
fn no_announce_has_unbalanced_placeholder() {
    for e in MYTHIC_EVENTS {
        let cleaned = e
            .announce
            .replace("{atk}", "")
            .replace("{def}", "")
            .replace("{winner}", "")
            .replace("{loser}", "");
        assert!(!cleaned.contains('{'), "{} : placeholder inconnu", e.key);
        assert!(!cleaned.contains('}'), "{} : placeholder inconnu", e.key);
    }
}

#[test]
fn rolls_none_most_of_the_time() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut none_count = 0;
    let total = 10_000;
    for _ in 0..total {
        if roll_mythic_event(&mut rng).is_none() {
            none_count += 1;
        }
    }
    // On veut que >= 95% des combats n aient pas de mythique.
    let none_rate = none_count as f64 / total as f64;
    assert!(none_rate > 0.95, "taux de none = {:.4}, attendu > 0.95", none_rate);
}

#[test]
fn rolls_can_produce_each_event_eventually() {
    let mut rng = StdRng::seed_from_u64(7);
    let mut seen: std::collections::HashSet<&'static str> = std::collections::HashSet::new();
    // 200_000 spins doit suffire pour voir au moins 5 des 10 mythiques
    // (les 5 plus rares peuvent encore manquer).
    for _ in 0..200_000 {
        if let Some(ev) = roll_mythic_event(&mut rng) {
            seen.insert(ev.key);
        }
    }
    assert!(seen.len() >= 5, "vu seulement {} mythiques sur 200k spins", seen.len());
}

#[test]
fn format_substitutes_all_placeholders() {
    let trefle = MYTHIC_EVENTS.iter().find(|e| e.key == "trefle_quatre_feuilles").unwrap();
    let s = format_mythic_announce(trefle, "Alice", "Bob", Some("Alice"), Some("Bob"));
    assert!(s.contains("Alice"));
    assert!(s.contains("Bob"));
    assert!(!s.contains("{winner}"));
    assert!(!s.contains("{loser}"));
}

#[test]
fn format_handles_none_winner_loser() {
    let invasion = MYTHIC_EVENTS.iter().find(|e| e.key == "invasion_poulets").unwrap();
    let s = format_mythic_announce(invasion, "A", "B", None, None);
    assert!(!s.contains("{winner}"));
    assert!(!s.contains("{loser}"));
    // Fallback "le gagnant" / "le perdant" ne doit pas apparaitre dans
    // invasion_poulets car son template ne les utilise pas.
    assert_eq!(s, invasion.announce);
}

#[test]
fn mechanical_flags_count_matches_expectation() {
    // Au fur et a mesure que les effets mythiques sont branches dans
    // resolve_combat_now_service, la liste ci-dessous doit etre etendue.
    let branched: Vec<&str> = MYTHIC_EVENTS
        .iter()
        .filter(|e| e.mechanical_implemented)
        .map(|e| e.key)
        .collect();
    let expected = [
        "licorne_rose",
        "etoile_filante",
        "jackpot_divin",
        "revanche_outre_tombe",
        "invasion_poulets",
        "distributeur_pq",
        "trefle_quatre_feuilles",
        "magicien",
        "bombe_nucleaire",
    ];
    assert_eq!(
        branched, expected,
        "le set de mythiques branches mecaniquement a change"
    );
}
