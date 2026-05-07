//! Use case "catalogue Coup de Coude" — expose les donnees statiques
//! (classes, shop, progression) au bot via un RPC unique appele au boot.
//!
//! Phase 8 refacto : elimine la duplication entre `bots/coude-bot/src/game/*`
//! et `sentinel-api/src/domain/services/coude_combat_engine/*`. L'API
//! devient la SEULE source de verite. Le bot fetch le catalog au boot,
//! cache en memoire, et fait des lookups locaux sans jamais recalculer.

use async_trait::async_trait;

use crate::domain::errors::DomainError;

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
    /// HP restauree par cet item (0 si ce n'est pas une potion consommable).
    /// Fourni par l'API pour que le bot n'ait pas a coder `is_potion` ou
    /// `potion_heal_amount` en dur.
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
pub struct AntiTheftItemInfo {
    pub key: String,
    pub block_chance_percent: u32,
}

#[derive(Debug, Clone)]
pub struct Catalog {
    pub classes: Vec<ClassInfo>,
    pub shop_items: Vec<ShopItemInfo>,
    pub level_table: Vec<LevelEntry>,
    pub matchmaking_buckets: Vec<MatchmakingBucket>,
    pub anti_theft_items: Vec<AntiTheftItemInfo>,
    pub max_level: i32,
    /// Formule affichage : HP max = `hp_base + def_effective * hp_per_def`.
    /// Permet au bot d'afficher une barre de HP sans connaitre la formule.
    pub hp_base: i32,
    pub hp_per_def: i32,
}

#[async_trait]
pub trait ManageCoudeCatalogUseCase: Send + Sync {
    async fn get_catalog(&self) -> Result<Catalog, DomainError>;
}
