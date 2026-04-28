//! Moteur de combat Coup de Coude + donnees metier (catalogues, progression).
//!
//! ## Phase 8 — source unique de verite
//!
//! Tout le metier Coude vit ici (domain layer, pur, sans IO) :
//! - `combat` : moteur de resolution multi-rounds
//! - `classes` : catalogue des classes (stats, passifs)
//! - `shop` : catalogue des items
//! - `progression` : formules XP / level / handicap / titres
//! - `chaos` : evenements chaos + roll aleatoire
//!
//! Le bot et le worker n'ont plus aucune copie locale de ces donnees.
//! Ils recuperent le catalogue via le RPC `CoudeCatalogService.GetCatalog`
//! au boot, puis font des lookups sur le cache.
//!
//! ## Ajouter un nouvel item / classe / chaos event
//!
//! 1. Editer le fichier approprie ici
//! 2. Rebuild l'API, le bot recuperera automatiquement la nouvelle version
//!    au prochain boot (ou via un refresh du catalog si on l'ajoute)
//! 3. Zero code cote bot a toucher sauf si l'effet combat est nouveau

pub mod chaos;
pub mod classes;
pub mod combat;
pub mod progression;
pub mod shop;

/// Donnees joueur minimales necessaires au moteur de combat.
/// Cree depuis un `SELECT` sur `coude_players` dans le worker.
#[derive(Debug, Clone)]
pub struct PlayerLite {
    pub user_id: String,
    pub class: Option<String>,
    pub level: i32,
    pub atk: i32,
    pub def: i32,
    pub cowardice_count: i32,
    pub hp_current: Option<i32>,
}

/// Evenement serveur minimal (seul `event_type` est lu par le moteur
/// actuellement : "happy_hour", "bloodbath"...).
#[derive(Debug, Clone)]
pub struct ServerEventLite {
    pub event_type: String,
}
