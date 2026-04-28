use super::*;
use crate::ports::inbound::manage_catalog::ManageCoudeCatalogUseCase;

#[tokio::test]
async fn catalog_contains_four_classes() {
    let svc = ManageCoudeCatalogService::new();
    let cat = svc.get_catalog().await.unwrap();
    assert_eq!(cat.classes.len(), 4);
    let names: Vec<&str> = cat.classes.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"bourrin"));
    assert!(names.contains(&"agile"));
    assert!(names.contains(&"fourbe"));
    assert!(names.contains(&"tank"));
}

#[tokio::test]
async fn catalog_shop_items_heal_amounts() {
    let svc = ManageCoudeCatalogService::new();
    let cat = svc.get_catalog().await.unwrap();
    let soin = cat.shop_items.iter().find(|i| i.key == "potion_soin").unwrap();
    let majeure = cat.shop_items.iter().find(|i| i.key == "potion_majeure").unwrap();
    let rage = cat.shop_items.iter().find(|i| i.key == "rage").unwrap();
    assert_eq!(soin.heal_amount, 30);
    assert_eq!(majeure.heal_amount, 80);
    assert_eq!(rage.heal_amount, 0);
}

#[tokio::test]
async fn catalog_level_table_has_max_level_entries() {
    let svc = ManageCoudeCatalogService::new();
    let cat = svc.get_catalog().await.unwrap();
    assert_eq!(cat.level_table.len() as i32, cat.max_level);
    assert_eq!(cat.level_table[0].level, 1);
    assert_eq!(cat.level_table.last().unwrap().level, cat.max_level);
}

#[tokio::test]
async fn catalog_matchmaking_buckets_cover_full_range() {
    let svc = ManageCoudeCatalogService::new();
    let cat = svc.get_catalog().await.unwrap();
    assert_eq!(cat.matchmaking_buckets.len(), 4);
    // Le dernier bucket doit etre blocked.
    let last = cat.matchmaking_buckets.last().unwrap();
    assert!(last.blocked);
    // Les handicaps decroissent avec l'ecart.
    let handicaps: Vec<f64> = cat.matchmaking_buckets.iter().map(|b| b.handicap).collect();
    for pair in handicaps.windows(2) {
        assert!(pair[0] >= pair[1], "handicap should decrease with gap");
    }
}

#[tokio::test]
async fn catalog_anti_theft_items_deprecated_empty() {
    let svc = ManageCoudeCatalogService::new();
    let cat = svc.get_catalog().await.unwrap();
    assert!(cat.anti_theft_items.is_empty());
}

#[tokio::test]
async fn catalog_hp_constants() {
    let svc = ManageCoudeCatalogService::new();
    let cat = svc.get_catalog().await.unwrap();
    assert_eq!(cat.hp_base, 100);
    assert_eq!(cat.hp_per_def, 2);
    assert_eq!(cat.max_level, 25);
}

#[tokio::test]
async fn catalog_shop_items_match_source() {
    use crate::domain::services::coude_combat_engine::shop::SHOP_ITEMS;
    let svc = ManageCoudeCatalogService::new();
    let cat = svc.get_catalog().await.unwrap();
    assert_eq!(cat.shop_items.len(), SHOP_ITEMS.len());
}

#[test]
fn default_constructor_matches_new() {
    let a = ManageCoudeCatalogService::new();
    let b = ManageCoudeCatalogService;
    // Les deux compilent — verification du Default impl.
    let _ = (a, b);
    let _d: ManageCoudeCatalogService = Default::default();
}
