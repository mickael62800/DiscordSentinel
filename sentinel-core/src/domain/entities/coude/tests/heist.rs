use super::*;
use crate::domain::entities::coude::economy_config::CoudeEconomyConfig;

fn ecfg() -> CoudeEconomyConfig {
    CoudeEconomyConfig::default()
}

#[test]
fn compute_chance_no_items_is_base() {
    let empty: Vec<&str> = vec![];
    assert_eq!(
        compute_success_chance(empty, &ecfg()),
        HEIST_BASE_SUCCESS_PERCENT
    );
}

#[test]
fn compute_chance_adds_individual_bonus() {
    let v = vec!["masque_braquage", "pied_de_biche"];
    assert_eq!(compute_success_chance(v, &ecfg()), 10);
}

#[test]
fn compute_chance_ignores_unknown_items() {
    let v = vec!["masque_braquage", "unknown_tool"];
    assert_eq!(compute_success_chance(v, &ecfg()), 7);
}

#[test]
fn compute_chance_deduplicates_items() {
    let v = vec!["masque_braquage", "masque_braquage"];
    assert_eq!(compute_success_chance(v, &ecfg()), 7);
}

#[test]
fn compute_chance_caps_at_max() {
    let v: Vec<&str> = HEIST_TOOLS.iter().map(|t| t.key).collect();
    assert_eq!(
        compute_success_chance(v, &ecfg()),
        HEIST_MAX_SUCCESS_PERCENT
    );
}

#[test]
fn custom_config_changes_base_and_cap() {
    // Base 10, cap 20 : sans item -> 10 ; tous les outils -> clamp 20.
    let cfg = CoudeEconomyConfig {
        heist_base_success_pct: 10,
        heist_max_success_pct: 20,
        ..CoudeEconomyConfig::default()
    };
    let empty: Vec<&str> = vec![];
    assert_eq!(compute_success_chance(empty, &cfg), 10);
    let all: Vec<&str> = HEIST_TOOLS.iter().map(|t| t.key).collect();
    assert_eq!(compute_success_chance(all, &cfg), 20);
}

#[test]
fn catalog_has_exactly_9_tools() {
    assert_eq!(HEIST_TOOLS.len(), 9);
}

#[test]
fn catalog_prices_are_ascending() {
    for pair in HEIST_TOOLS.windows(2) {
        assert!(
            pair[0].price <= pair[1].price,
            "catalog heist tools non tries par prix ascending : {} ({}) vs {} ({})",
            pair[0].key,
            pair[0].price,
            pair[1].key,
            pair[1].price
        );
    }
}

#[test]
fn find_heist_tool_works() {
    assert!(find_heist_tool("masque_braquage").is_some());
    assert!(find_heist_tool("equipe_de_pros").is_some());
    assert!(find_heist_tool("unknown").is_none());
}

#[test]
fn gain_range_is_sensible() {
    const _: () = assert!(HEIST_GAIN_MIN_PERCENT < HEIST_GAIN_MAX_PERCENT);
    const _: () = assert!(HEIST_GAIN_MAX_PERCENT <= 100);
}

// ══════════════════════════════════════════════════════════
//  Stress tests — invariants sur compute_success_chance
// ══════════════════════════════════════════════════════════

#[test]
fn chance_never_below_base() {
    // Invariant : avec n'importe quelle combinaison d'items (meme inconnus),
    // la chance doit etre >= HEIST_BASE_SUCCESS_PERCENT.
    use rand::seq::SliceRandom;
    let mut rng = rand::thread_rng();
    let all_keys: Vec<&str> = HEIST_TOOLS.iter().map(|t| t.key).collect();
    for _ in 0..100 {
        let n: usize = rand::Rng::gen_range(&mut rng, 0..=15);
        let picks: Vec<&str> = all_keys
            .choose_multiple(&mut rng, n.min(all_keys.len()))
            .copied()
            .collect();
        let chance = compute_success_chance(picks, &ecfg());
        assert!(chance >= HEIST_BASE_SUCCESS_PERCENT);
        assert!(chance <= HEIST_MAX_SUCCESS_PERCENT);
    }
}

#[test]
fn chance_monotonic_with_more_items() {
    // Invariant : ajouter un item valide ne peut pas FAIRE BAISSER la chance.
    let mut acc: Vec<&str> = vec![];
    let mut prev = compute_success_chance(acc.clone(), &ecfg());
    for tool in HEIST_TOOLS {
        acc.push(tool.key);
        let current = compute_success_chance(acc.clone(), &ecfg());
        assert!(
            current >= prev,
            "ajouter {} a fait baisser la chance : {prev} → {current}",
            tool.key
        );
        prev = current;
    }
}

#[test]
fn chance_saturates_at_max_with_all_tools() {
    // La somme des bonus_percent = 50 (verifie ailleurs). Avec base = 5, le
    // total atteint ou depasse 50 (HEIST_MAX_SUCCESS_PERCENT). Verifier que
    // le clamping fonctionne sur une liste dupliquee.
    let all: Vec<&str> = HEIST_TOOLS.iter().map(|t| t.key).collect();
    // Double la liste pour provoquer le dedup + clamping.
    let mut doubled = all.clone();
    doubled.extend(all.iter().copied());
    assert_eq!(
        compute_success_chance(doubled, &ecfg()),
        HEIST_MAX_SUCCESS_PERCENT
    );
}

#[test]
fn chance_invariant_dedup_equivalence() {
    // Invariant : compute_success_chance([x, x, x]) == compute_success_chance([x])
    for tool in HEIST_TOOLS {
        let single = compute_success_chance(vec![tool.key], &ecfg());
        let triple = compute_success_chance(vec![tool.key, tool.key, tool.key], &ecfg());
        assert_eq!(single, triple, "dedup cassé pour {}", tool.key);
    }
}

#[test]
fn chance_empty_input_equals_base() {
    let empty_str: Vec<String> = vec![];
    assert_eq!(
        compute_success_chance(empty_str, &ecfg()),
        HEIST_BASE_SUCCESS_PERCENT
    );
    let empty_ref: Vec<&str> = vec![];
    assert_eq!(
        compute_success_chance(empty_ref, &ecfg()),
        HEIST_BASE_SUCCESS_PERCENT
    );
}

#[test]
fn catalog_bonus_sum_equals_max() {
    // Invariant : base + sum(bonus) == max, pour qu'un joueur possedant
    // tous les outils touche pile son bonus complet (pas de clipping silencieux).
    //
    // Fix applique : HEIST_MAX_SUCCESS_PERCENT passe de 50 a 55 pour absorber
    // la somme des 9 bonus (base 5 + sum 50 = 55). Ce test garde-fou contre
    // une regression si on touche aux constantes sans recompter.
    let sum_bonuses: u32 = HEIST_TOOLS.iter().map(|t| t.bonus_percent).sum();
    assert_eq!(sum_bonuses, 50, "somme bonus attendue a 50");
    assert_eq!(HEIST_BASE_SUCCESS_PERCENT, 5);
    assert_eq!(HEIST_MAX_SUCCESS_PERCENT, 55);
    assert_eq!(
        HEIST_BASE_SUCCESS_PERCENT + sum_bonuses,
        HEIST_MAX_SUCCESS_PERCENT,
        "base + sum(bonus) doit etre exactement == max pour eviter le clipping silencieux"
    );
}

#[test]
fn all_tools_reach_exactly_max_chance() {
    // Avec les 9 outils, compute_success_chance doit rendre EXACTEMENT 55.
    let all: Vec<&str> = HEIST_TOOLS.iter().map(|t| t.key).collect();
    assert_eq!(compute_success_chance(all, &ecfg()), 55);
}

// ── PrisonState::is_active ──

#[test]
fn prison_state_is_active_true_when_released_at_future() {
    let state = PrisonState {
        guild_id: "g".into(),
        user_id: "u".into(),
        released_at: chrono::Utc::now() + chrono::Duration::hours(1),
        reason: "heist fail".into(),
        created_at: chrono::Utc::now(),
    };
    assert!(state.is_active());
}

#[test]
fn prison_state_is_active_false_when_released_at_past() {
    let state = PrisonState {
        guild_id: "g".into(),
        user_id: "u".into(),
        released_at: chrono::Utc::now() - chrono::Duration::hours(1),
        reason: "heist fail".into(),
        created_at: chrono::Utc::now() - chrono::Duration::hours(25),
    };
    assert!(!state.is_active());
}
