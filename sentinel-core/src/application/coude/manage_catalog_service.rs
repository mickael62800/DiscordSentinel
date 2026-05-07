//! Impl du use case `ManageCoudeCatalogUseCase`. Retourne le catalogue
//! complet (classes, shop, progression, matchmaking) depuis les donnees
//! domain pures dans `coude_combat_engine`.
//!
//! Zero IO, zero DB — c'est de la lecture de static data.

use async_trait::async_trait;

use crate::domain::errors::DomainError;
use crate::domain::services::coude::coude_combat_engine::classes;
use crate::domain::services::coude::coude_combat_engine::progression;
use crate::domain::services::coude::coude_combat_engine::shop;
use crate::ports::inbound::coude::manage_catalog::AntiTheftItemInfo;
use crate::ports::inbound::coude::manage_catalog::ClassInfo;
use crate::ports::inbound::coude::manage_catalog::Catalog;
use crate::ports::inbound::coude::manage_catalog::LevelEntry;
use crate::ports::inbound::coude::manage_catalog::ManageCoudeCatalogUseCase;
use crate::ports::inbound::coude::manage_catalog::MatchmakingBucket;
use crate::ports::inbound::coude::manage_catalog::ShopItemInfo;
pub struct ManageCoudeCatalogService;

impl ManageCoudeCatalogService {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ManageCoudeCatalogService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ManageCoudeCatalogUseCase for ManageCoudeCatalogService {
    async fn get_catalog(&self) -> Result<Catalog, DomainError> {
        // ── Classes (depuis le domain) ──
        let classes_data: Vec<ClassInfo> = [
            &classes::CLASS_BOURRIN,
            &classes::CLASS_AGILE,
            &classes::CLASS_FOURBE,
            &classes::CLASS_TANK,
        ]
        .iter()
        .map(|c| ClassInfo {
            name: c.name.to_string(),
            emoji: c.emoji.to_string(),
            base_atk: c.base_atk,
            base_def: c.base_def,
            atk_growth: c.atk_growth,
            def_growth: c.def_growth,
            dodge_chance: c.dodge_chance,
            steal_bonus: c.steal_bonus,
            description: c.description.to_string(),
            passif_key: c.passif_key.to_string(),
            passif_description: c.passif_description.to_string(),
            passif_reveal: c.passif_reveal.to_string(),
        })
        .collect();

        // ── Shop items ──
        // Le `heal_amount` est derive de la clef : c'est la seule info non
        // presente dans la struct `ShopItem` statique. Ajouter une nouvelle
        // potion ? Ajouter son key ici.
        let shop_items: Vec<ShopItemInfo> = shop::SHOP_ITEMS
            .iter()
            .map(|i| ShopItemInfo {
                key: i.key.to_string(),
                name: i.name.to_string(),
                emoji: i.emoji.to_string(),
                price: i.price,
                description: i.description.to_string(),
                category: i.category.to_string(),
                heal_amount: match i.key {
                    "potion_soin" => 30,
                    "potion_majeure" => 80,
                    _ => 0,
                },
            })
            .collect();

        // ── Level table (1 → MAX_LEVEL) ──
        let level_table: Vec<LevelEntry> = (1..=progression::MAX_LEVEL)
            .map(|lvl| LevelEntry {
                level: lvl,
                title: progression::title_for_level(lvl).to_string(),
                xp_cumul: progression::xp_for_level(lvl),
            })
            .collect();

        // ── Matchmaking buckets ──
        // Derive des buckets statiques utilises par `matchmaking_handicap`.
        let matchmaking_buckets = vec![
            MatchmakingBucket { gap_min: 0, gap_max: 2, handicap: 1.0, blocked: false },
            MatchmakingBucket { gap_min: 3, gap_max: 5, handicap: 0.8, blocked: false },
            MatchmakingBucket { gap_min: 6, gap_max: 9, handicap: 0.6, blocked: false },
            MatchmakingBucket { gap_min: 10, gap_max: 999, handicap: 0.0, blocked: true },
        ];

        // ── Anti-vol items ──
        let anti_theft_items: Vec<AntiTheftItemInfo> = shop::ANTI_THEFT_ITEMS
            .iter()
            .map(|(k, chance)| AntiTheftItemInfo {
                key: (*k).to_string(),
                block_chance_percent: *chance,
            })
            .collect();

        Ok(Catalog {
            classes: classes_data,
            shop_items,
            level_table,
            matchmaking_buckets,
            anti_theft_items,
            max_level: progression::MAX_LEVEL,
            hp_base: 100,
            hp_per_def: 2,
        })
    }
}

#[cfg(test)]
#[path = "tests/manage_catalog.rs"]
mod tests;
