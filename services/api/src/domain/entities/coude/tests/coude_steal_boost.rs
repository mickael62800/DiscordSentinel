use super::*;

#[test]
fn catalog_sorted_by_bonus_ascending() {
    for pair in STEAL_BOOST_ITEMS.windows(2) {
        assert!(
            pair[0].roll_bonus <= pair[1].roll_bonus,
            "catalogue non trie par bonus croissant"
        );
    }
}

#[test]
fn catalog_cost_per_bonus_point_decreases_or_stays_stable() {
    let mut last_ratio: Option<f64> = None;
    for i in STEAL_BOOST_ITEMS {
        let ratio = i.base_cost_per_day as f64 / i.roll_bonus as f64;
        if let Some(prev) = last_ratio {
            assert!(ratio >= prev * 0.8, "ratio cout/bonus regresse trop");
        }
        last_ratio = Some(ratio);
    }
}

#[test]
fn find_boost_item_works() {
    assert!(find_boost_item("crochet").is_some());
    assert!(find_boost_item("marteau").is_some());
    assert!(find_boost_item("unknown").is_none());
}

#[test]
fn sum_roll_bonus_empty_is_zero() {
    let empty: Vec<String> = vec![];
    assert_eq!(sum_roll_bonus_for_active_keys(empty), 0);
}

#[test]
fn sum_roll_bonus_stacks_all_actives() {
    let actives = vec!["crochet", "marteau"];
    assert_eq!(sum_roll_bonus_for_active_keys(actives), 30);
}

#[test]
fn sum_roll_bonus_ignores_unknown_keys() {
    let actives = vec!["crochet", "unknown_item"];
    assert_eq!(sum_roll_bonus_for_active_keys(actives), 5);
}

#[test]
fn sum_all_items_gives_expected_total() {
    let all: Vec<&str> = STEAL_BOOST_ITEMS.iter().map(|i| i.key).collect();
    assert_eq!(sum_roll_bonus_for_active_keys(all), 75);
}
