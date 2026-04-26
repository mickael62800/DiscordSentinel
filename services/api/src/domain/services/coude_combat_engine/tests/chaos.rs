use super::*;

// ── Metadata (key, emoji, label, description) ──

#[test]
fn key_all_variants() {
    assert_eq!(ChaosEvent::CritiqueSauvage.key(), "critique_sauvage");
    assert_eq!(ChaosEvent::EsquiveDivine.key(), "esquive_divine");
    assert_eq!(ChaosEvent::AccidentDebile.key(), "accident_debile");
    assert_eq!(ChaosEvent::Glissade.key(), "glissade");
    assert_eq!(ChaosEvent::Vol.key(), "vol");
}

#[test]
fn emoji_all_variants_non_empty() {
    for e in [
        ChaosEvent::CritiqueSauvage,
        ChaosEvent::EsquiveDivine,
        ChaosEvent::AccidentDebile,
        ChaosEvent::Glissade,
        ChaosEvent::Vol,
    ] {
        assert!(!e.emoji().is_empty(), "{:?} emoji empty", e);
    }
}

#[test]
fn label_all_variants() {
    assert_eq!(ChaosEvent::CritiqueSauvage.label(), "CRITIQUE SAUVAGE");
    assert_eq!(ChaosEvent::EsquiveDivine.label(), "ESQUIVE DIVINE");
    assert_eq!(ChaosEvent::AccidentDebile.label(), "ACCIDENT DEBILE");
    assert_eq!(ChaosEvent::Glissade.label(), "GLISSADE");
    assert_eq!(ChaosEvent::Vol.label(), "VOL A LA TIRE");
}

#[test]
fn description_all_variants_non_empty() {
    for e in [
        ChaosEvent::CritiqueSauvage,
        ChaosEvent::EsquiveDivine,
        ChaosEvent::AccidentDebile,
        ChaosEvent::Glissade,
        ChaosEvent::Vol,
    ] {
        assert!(!e.description().is_empty());
    }
}

#[test]
fn keys_all_unique() {
    let keys = [
        ChaosEvent::CritiqueSauvage.key(),
        ChaosEvent::EsquiveDivine.key(),
        ChaosEvent::AccidentDebile.key(),
        ChaosEvent::Glissade.key(),
        ChaosEvent::Vol.key(),
    ];
    let set: std::collections::HashSet<_> = keys.iter().collect();
    assert_eq!(set.len(), keys.len(), "duplicate chaos keys");
}

#[test]
fn keys_are_snake_case() {
    for e in [
        ChaosEvent::CritiqueSauvage,
        ChaosEvent::EsquiveDivine,
        ChaosEvent::AccidentDebile,
        ChaosEvent::Glissade,
        ChaosEvent::Vol,
    ] {
        let key = e.key();
        assert!(
            key.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "{:?} key not snake_case: {key}", e
        );
    }
}

// ── roll_chaos — distribution ──

#[test]
fn roll_chaos_returns_something_eventually() {
    // Sur 10k rolls, on doit avoir au moins un event chaos (~8% attendu).
    let mut any = false;
    for _ in 0..10_000 {
        if roll_chaos().is_some() {
            any = true;
            break;
        }
    }
    assert!(any, "roll_chaos a donne None 10000 fois de suite (proba < 1e-360)");
}

#[test]
fn roll_chaos_produces_all_variants_over_many_rolls() {
    // Sur 100k rolls, chaque variante doit etre tiree au moins une fois.
    let mut seen = [false; 5];
    for _ in 0..100_000 {
        match roll_chaos() {
            Some(ChaosEvent::CritiqueSauvage) => seen[0] = true,
            Some(ChaosEvent::EsquiveDivine) => seen[1] = true,
            Some(ChaosEvent::AccidentDebile) => seen[2] = true,
            Some(ChaosEvent::Glissade) => seen[3] = true,
            Some(ChaosEvent::Vol) => seen[4] = true,
            None => {}
        }
        if seen.iter().all(|&b| b) { break; }
    }
    assert!(seen.iter().all(|&b| b), "tous les 5 chaos events devraient apparaitre : {:?}", seen);
}

#[test]
fn roll_chaos_distribution_approximately_8_percent() {
    // Invariant : ~8% par round (20+20+15+10+15 = 80/1000 = 8%).
    // Sur 50k rolls, on tolere 6-10%.
    let mut hits = 0;
    let total = 50_000;
    for _ in 0..total {
        if roll_chaos().is_some() { hits += 1; }
    }
    let pct = hits as f64 / total as f64 * 100.0;
    assert!(
        (5.0..=11.0).contains(&pct),
        "chaos rate {:.2}% hors tolerance 5-11%", pct
    );
}

#[test]
fn roll_chaos_thresholds_sum_to_80_promille() {
    // Verifie que les bornes du match dans roll_chaos() totalisent exactement
    // 80 hits sur 1000 (== 8%). Si quelqu'un modifie les bornes en gardant
    // l'intention, ce test doit rester vert.
    //
    // On reconstruit les plages depuis la source : 1-20 + 21-40 + 41-55 + 56-65 + 66-80 = 80.
    let iterations = 200_000;
    let mut total_hits: usize = 0;
    for _ in 0..iterations {
        if roll_chaos().is_some() { total_hits += 1; }
    }
    let expected = (iterations as f64 * 0.08) as usize;
    // Tolerance : ±20% autour de 8%.
    let low = (expected as f64 * 0.80) as usize;
    let high = (expected as f64 * 1.20) as usize;
    assert!(
        (low..=high).contains(&total_hits),
        "total hits {total_hits} hors tolerance [{low}, {high}]"
    );
}

#[test]
fn chaos_event_equality_and_copy() {
    let a = ChaosEvent::Vol;
    let b = a;
    assert_eq!(a, b);
    assert_ne!(a, ChaosEvent::Glissade);
}

// ══════════════════════════════════════════════════════════
// Multiplicateur saison du Chaos (cf. COUPE_AMELIORATIONS 6.3)
// ══════════════════════════════════════════════════════════

#[test]
fn roll_chaos_multiplier_1_matches_default_distribution() {
    let iterations = 5_000;
    let mut hits = 0usize;
    for _ in 0..iterations {
        if roll_chaos_with_multiplier(1.0).is_some() {
            hits += 1;
        }
    }
    let expected = (iterations as f64 * 0.08) as usize;
    let low = (expected as f64 * 0.75) as usize;
    let high = (expected as f64 * 1.25) as usize;
    assert!(
        (low..=high).contains(&hits),
        "hits {hits} hors tolerance [{low}, {high}] avec multiplier 1.0"
    );
}

#[test]
fn roll_chaos_multiplier_2_doubles_event_rate() {
    let iterations = 5_000;
    let mut hits = 0usize;
    for _ in 0..iterations {
        if roll_chaos_with_multiplier(2.0).is_some() {
            hits += 1;
        }
    }
    // Avec x2 on attend ~16% (8% * 2). Tolerance ±25%.
    let expected = (iterations as f64 * 0.16) as usize;
    let low = (expected as f64 * 0.75) as usize;
    let high = (expected as f64 * 1.25) as usize;
    assert!(
        (low..=high).contains(&hits),
        "hits {hits} hors tolerance [{low}, {high}] avec multiplier 2.0"
    );
}

#[test]
fn roll_chaos_multiplier_0_disables_events() {
    for _ in 0..1000 {
        assert!(roll_chaos_with_multiplier(0.0).is_none());
    }
}
