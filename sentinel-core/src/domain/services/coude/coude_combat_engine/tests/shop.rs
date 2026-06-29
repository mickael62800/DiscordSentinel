use super::*;

#[test]
fn get_item_returns_known_items() {
    assert!(get_item("rage").is_some());
    assert!(get_item("potion_soin").is_some());
    assert!(get_item("explosion").is_some());
    assert!(get_item("masque_braquage").is_some());
}

#[test]
fn get_item_returns_none_for_unknown() {
    assert!(get_item("").is_none());
    assert!(get_item("unknown").is_none());
    assert!(get_item("RAGE").is_none()); // case-sensitive
}

#[test]
fn is_potion_identifies_potions() {
    assert!(is_potion("potion_soin"));
    assert!(is_potion("potion_majeure"));
    assert!(!is_potion("rage"));
    assert!(!is_potion("explosion"));
    assert!(!is_potion(""));
}

#[test]
fn potion_heal_amount_correct_values() {
    assert_eq!(potion_heal_amount("potion_soin"), 30);
    assert_eq!(potion_heal_amount("potion_majeure"), 80);
    assert_eq!(potion_heal_amount("rage"), 0);
    assert_eq!(potion_heal_amount("unknown"), 0);
    assert_eq!(potion_heal_amount(""), 0);
}

#[test]
fn potion_majeure_heals_more_than_potion_soin() {
    // Invariant : la potion majeure coute plus et soigne plus.
    assert!(potion_heal_amount("potion_majeure") > potion_heal_amount("potion_soin"));
    let soin = get_item("potion_soin").unwrap();
    let majeure = get_item("potion_majeure").unwrap();
    assert!(majeure.price > soin.price);
}

#[test]
fn shop_item_is_attaque_and_is_defense_mutually_exclusive() {
    for item in SHOP_ITEMS {
        let a = item.is_attaque();
        let d = item.is_defense();
        assert!(!(a && d), "{} cannot be both attaque AND defense", item.key);
    }
}

#[test]
fn all_shop_items_have_unique_keys() {
    let keys: std::collections::HashSet<&str> = SHOP_ITEMS.iter().map(|i| i.key).collect();
    assert_eq!(keys.len(), SHOP_ITEMS.len(), "duplicate shop keys detected");
}

#[test]
fn all_shop_items_have_positive_price() {
    for item in SHOP_ITEMS {
        assert!(item.price > 0, "{} has non-positive price", item.key);
    }
}

#[test]
fn all_shop_items_have_known_category() {
    for item in SHOP_ITEMS {
        assert!(
            matches!(item.category, "attaque" | "defense" | "braquage"),
            "{} has unknown category: {}",
            item.key,
            item.category
        );
    }
}

#[test]
fn all_shop_items_have_non_empty_metadata() {
    for item in SHOP_ITEMS {
        assert!(!item.key.is_empty());
        assert!(!item.name.is_empty(), "{} name empty", item.key);
        assert!(!item.emoji.is_empty(), "{} emoji empty", item.key);
        assert!(
            !item.description.is_empty(),
            "{} description empty",
            item.key
        );
    }
}

#[test]
fn shop_has_at_least_one_item_per_category() {
    let has_attaque = SHOP_ITEMS.iter().any(|i| i.is_attaque());
    let has_defense = SHOP_ITEMS.iter().any(|i| i.is_defense());
    let has_braquage = SHOP_ITEMS.iter().any(|i| i.category == "braquage");
    assert!(has_attaque);
    assert!(has_defense);
    assert!(has_braquage);
}

#[test]
fn anti_theft_items_deprecated_empty() {
    // Phase 9 Part B : ANTI_THEFT_ITEMS doit rester vide.
    assert!(
        ANTI_THEFT_ITEMS.is_empty(),
        "ANTI_THEFT_ITEMS should be empty (deprecated)"
    );
}

#[test]
fn explosion_is_defense_category() {
    // Explosion est une carte "defender only" classée defense.
    let explosion = get_item("explosion").unwrap();
    assert_eq!(explosion.category, "defense");
    assert!(explosion.is_defense());
}
