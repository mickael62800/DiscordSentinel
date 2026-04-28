use super::*;

#[test]
fn duration_cost_multipliers_decrease_per_day() {
    let base = 100_i64;
    let one = StealProtectionDuration::OneDay.total_cost(base);
    let three = StealProtectionDuration::ThreeDays.total_cost(base);
    let five = StealProtectionDuration::FiveDays.total_cost(base);
    let seven = StealProtectionDuration::SevenDays.total_cost(base);

    assert_eq!(one, 100);
    assert_eq!(three, 270);
    assert_eq!(five, 425);
    assert_eq!(seven, 560);

    let per_day_one = one as f64 / 1.0;
    let per_day_three = three as f64 / 3.0;
    let per_day_five = five as f64 / 5.0;
    let per_day_seven = seven as f64 / 7.0;
    assert!(per_day_one > per_day_three);
    assert!(per_day_three > per_day_five);
    assert!(per_day_five > per_day_seven);
}

#[test]
fn duration_round_trip() {
    for d in [
        StealProtectionDuration::OneDay,
        StealProtectionDuration::ThreeDays,
        StealProtectionDuration::FiveDays,
        StealProtectionDuration::SevenDays,
    ] {
        assert_eq!(StealProtectionDuration::from_key(d.as_key()), Some(d));
    }
}

#[test]
fn duration_days_values_correct() {
    assert_eq!(StealProtectionDuration::OneDay.days(), 1);
    assert_eq!(StealProtectionDuration::ThreeDays.days(), 3);
    assert_eq!(StealProtectionDuration::FiveDays.days(), 5);
    assert_eq!(StealProtectionDuration::SevenDays.days(), 7);
}

#[test]
fn duration_from_key_unknown_returns_none() {
    assert_eq!(StealProtectionDuration::from_key(""), None);
    assert_eq!(StealProtectionDuration::from_key("2d"), None);
    assert_eq!(StealProtectionDuration::from_key("unknown"), None);
    assert_eq!(StealProtectionDuration::from_key("1D"), None); // case-sensitive
}

#[test]
fn catalog_sorted_by_block_chance_ascending() {
    for pair in STEAL_PROTECTION_ITEMS.windows(2) {
        assert!(
            pair[0].block_chance_percent <= pair[1].block_chance_percent,
            "catalogue non trie par block_chance croissant"
        );
    }
}

#[test]
fn find_protection_item_works() {
    assert!(find_protection_item("chien_garde").is_some());
    assert!(find_protection_item("coffre_fort").is_some());
    assert!(find_protection_item("forteresse").is_some());
    assert!(find_protection_item("unknown").is_none());
}
