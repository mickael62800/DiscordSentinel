use super::*;

#[test]
fn no_flags_on_normal_combat() {
    // Gagnant fini a 50% HP en 5 rounds, jamais en bas, premier d20 = 15
    let f = detect_outcome_flags(100, 200, 0, 5, Some(15), 0);
    assert!(!f.is_any_set());
}

#[test]
fn clutch_when_under_10_pct() {
    // Gagnant fini a 5% HP (10/200)
    let f = detect_outcome_flags(10, 200, 0, 8, Some(15), 0);
    assert!(f.clutch);
    assert!(!f.perfect);
}

#[test]
fn clutch_at_exactly_10_pct() {
    // Borne stricte : 10% pile = clutch (<=)
    let f = detect_outcome_flags(20, 200, 0, 8, Some(15), 0);
    assert!(f.clutch);
}

#[test]
fn no_clutch_when_winner_dead() {
    // Gagnant a 0 HP (mort par d'autres mecaniques) : pas de clutch
    let f = detect_outcome_flags(0, 200, 0, 8, Some(15), 50);
    assert!(!f.clutch);
}

#[test]
fn perfect_at_exactly_90_pct() {
    let f = detect_outcome_flags(180, 200, 0, 5, Some(15), 0);
    assert!(f.perfect);
    assert!(!f.clutch);
}

#[test]
fn perfect_at_100_pct() {
    let f = detect_outcome_flags(200, 200, 0, 5, Some(15), 0);
    assert!(f.perfect);
}

#[test]
fn no_perfect_at_89_pct() {
    let f = detect_outcome_flags(178, 200, 0, 5, Some(15), 0);
    assert!(!f.perfect);
}

#[test]
fn comeback_when_low_hp_rounds_at_least_two() {
    // Gagnant fini en pleine forme MAIS a passe 3 rounds sous 20% HP
    let f = detect_outcome_flags(150, 200, 3, 8, Some(15), 0);
    assert!(f.comeback);
    assert!(!f.clutch);
}

#[test]
fn no_comeback_with_only_one_low_round() {
    let f = detect_outcome_flags(150, 200, 1, 8, Some(15), 0);
    assert!(!f.comeback);
}

#[test]
fn ridicule_when_one_round_and_first_d20_is_1() {
    let f = detect_outcome_flags(0, 200, 0, 1, Some(1), 0);
    assert!(f.ridicule);
}

#[test]
fn no_ridicule_when_one_round_but_d20_not_1() {
    let f = detect_outcome_flags(0, 200, 0, 1, Some(20), 0);
    assert!(!f.ridicule);
}

#[test]
fn no_ridicule_when_more_than_one_round() {
    let f = detect_outcome_flags(0, 200, 0, 5, Some(1), 0);
    assert!(!f.ridicule);
}

#[test]
fn no_ridicule_when_d20_unknown() {
    let f = detect_outcome_flags(0, 200, 0, 1, None, 0);
    assert!(!f.ridicule);
}

#[test]
fn zero_pointe_when_both_at_zero() {
    let f = detect_outcome_flags(0, 200, 0, 5, Some(15), 0);
    assert!(f.zero_pointe);
}

#[test]
fn no_zero_pointe_when_loser_alive() {
    let f = detect_outcome_flags(0, 200, 0, 5, Some(15), 10);
    assert!(!f.zero_pointe);
}

#[test]
fn flags_can_combine_clutch_and_comeback() {
    // Gagnant fini a 5% (clutch) ET a passe 3 rounds en bas HP (comeback)
    let f = detect_outcome_flags(10, 200, 3, 8, Some(15), 0);
    assert!(f.clutch);
    assert!(f.comeback);
    assert_eq!(f.labels(), vec!["🔥 CLUTCH", "⚡ COMEBACK"]);
}

#[test]
fn labels_empty_when_no_flag() {
    let f = CombatOutcomeFlags::default();
    assert!(f.labels().is_empty());
    assert!(!f.is_any_set());
}

#[test]
fn labels_order_is_stable() {
    let mut f = CombatOutcomeFlags::default();
    f.clutch = true;
    f.comeback = true;
    f.perfect = true;
    f.ridicule = true;
    f.zero_pointe = true;
    let labels = f.labels();
    assert_eq!(labels.len(), 5);
    assert!(labels[0].contains("CLUTCH"));
    assert!(labels[1].contains("COMEBACK"));
    assert!(labels[2].contains("PERFECT"));
    assert!(labels[3].contains("RIDICULE"));
    assert!(labels[4].contains("ZERO"));
}

#[test]
fn defensive_against_invalid_hp_max() {
    // hp_max = 0 -> evite division par zero, retourne flags vides
    let f = detect_outcome_flags(0, 0, 0, 5, Some(15), 0);
    assert!(!f.clutch && !f.perfect);
}
