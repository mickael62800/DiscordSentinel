#![allow(dead_code)]
//! Cache local en lecture seule du catalogue Coup de Coude.
//!
//! Phase 8 refacto : le bot ne contient PLUS aucune donnee metier locale
//! (classes, shop, progression). Il fetche le catalogue une fois au boot
//! via le RPC `CoudeSocialService.GetCatalog`, stocke le resultat dans ce
//! cache (`CatalogCache` inserted dans la TypeMap Serenity) et fait des
//! lookups synchrones sans jamais recalculer quoi que ce soit.
//!
//! Pour modifier une classe / un item / la formule d'XP : editer le
//! fichier correspondant dans `services/api/src/domain/services/
//! coude_combat_engine/` cote API, rebuild, le bot recupere la nouvelle
//! version au prochain boot. **Zero fichier cote bot a toucher.**

use serenity::prelude::TypeMapKey;

#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub name: String,
    pub emoji: String,
    pub base_atk: i32,
    pub base_def: i32,
    pub atk_growth: i32,
    pub def_growth: i32,
    pub dodge_chance: f64,
    pub steal_bonus: f64,
    pub description: String,
    pub passif_key: String,
    pub passif_description: String,
    pub passif_reveal: String,
}

#[derive(Debug, Clone)]
pub struct ShopItemInfo {
    pub key: String,
    pub name: String,
    pub emoji: String,
    pub price: i64,
    pub description: String,
    pub category: String,
    pub heal_amount: i32,
}

#[derive(Debug, Clone)]
pub struct LevelEntry {
    pub level: i32,
    pub title: String,
    pub xp_cumul: i64,
}

#[derive(Debug, Clone)]
pub struct MatchmakingBucket {
    pub gap_min: i32,
    pub gap_max: i32,
    pub handicap: f64,
    pub blocked: bool,
}

#[derive(Debug, Clone)]
pub struct AntiTheftItem {
    pub key: String,
    pub block_chance_percent: u32,
}

/// Cache immuable du catalogue Coude, charge au boot.
#[derive(Debug, Clone)]
pub struct CatalogCache {
    pub classes: Vec<ClassInfo>,
    pub shop_items: Vec<ShopItemInfo>,
    pub level_table: Vec<LevelEntry>,
    pub matchmaking_buckets: Vec<MatchmakingBucket>,
    pub anti_theft_items: Vec<AntiTheftItem>,
    pub max_level: i32,
    pub hp_base: i32,
    pub hp_per_def: i32,
}

impl CatalogCache {
    /// Lookup classe par nom (fallback bourrin si inconnue).
    pub fn get_class(&self, name: &str) -> &ClassInfo {
        self.classes
            .iter()
            .find(|c| c.name == name)
            .unwrap_or_else(|| {
                self.classes
                    .first()
                    .expect("catalog.classes vide — fatal")
            })
    }

    /// Lookup item par clef.
    pub fn get_item(&self, key: &str) -> Option<&ShopItemInfo> {
        self.shop_items.iter().find(|i| i.key == key)
    }

    /// Verifie si un item est une potion consommable (a heal_amount > 0).
    pub fn is_potion(&self, key: &str) -> bool {
        self.get_item(key).map(|i| i.heal_amount > 0).unwrap_or(false)
    }

    /// Quantite de HP restauree par une potion (0 si pas une potion).
    pub fn potion_heal_amount(&self, key: &str) -> i32 {
        self.get_item(key).map(|i| i.heal_amount).unwrap_or(0)
    }

    /// Titre correspondant a un niveau (lookup O(n) acceptable car ≤ 25 entrees).
    pub fn title_for_level(&self, level: i32) -> &str {
        self.level_table
            .iter()
            .find(|e| e.level == level)
            .map(|e| e.title.as_str())
            .unwrap_or("Debutant")
    }

    /// XP cumul requis pour atteindre un niveau.
    pub fn xp_for_level(&self, level: i32) -> i64 {
        self.level_table
            .iter()
            .find(|e| e.level == level)
            .map(|e| e.xp_cumul)
            .unwrap_or(0)
    }

    /// Handicap matchmaking : (multiplicateur, est_bloque).
    pub fn matchmaking_handicap(&self, attacker_level: i32, defender_level: i32) -> (f64, bool) {
        let gap = (attacker_level - defender_level).abs();
        for bucket in &self.matchmaking_buckets {
            if gap >= bucket.gap_min && gap <= bucket.gap_max {
                return (bucket.handicap, bucket.blocked);
            }
        }
        (1.0, false)
    }

    /// Formule d'affichage HP max = hp_base + def_effective * hp_per_def.
    pub fn display_hp(&self, effective_def: i32) -> i32 {
        self.hp_base + effective_def * self.hp_per_def
    }
}

/// Cle TypeMap pour stocker le cache dans `ctx.data`.
pub struct CatalogCacheKey;

impl TypeMapKey for CatalogCacheKey {
    type Value = std::sync::Arc<CatalogCache>;
}
