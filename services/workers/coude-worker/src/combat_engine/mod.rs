//! Moteur de combat Coup de Coude — copie autonome du moteur utilise par
//! `bots/coude-bot/src/game/*` pour que le worker puisse resoudre les
//! combats en phase betting avec la logique complete (rounds, HP, classes,
//! chaos, items) au lieu d'un simple random.
//!
//! ## Pourquoi un duplicate
//!
//! L'ideal serait de partager ce moteur via un crate commun, mais :
//! - Il ne depend de rien de non-trivial (rand uniquement, pas d'I/O)
//! - Le porter dans `sentinel-shared` amenerait beaucoup de code non utile
//!   a cote bot (chaos/classes/progression)
//! - Un crate dedie `coude-domain` serait plus propre mais c'est du refactor
//!   Pour eviter la duplication, le moteur cote bot doit etre garde en
//!   synchro manuelle (c'est une contrainte acceptable vu qu'il bouge peu)
//!
//! ## Adaptations par rapport a `bots/coude-bot/src/game/`
//!
//! - Le type `Player` du bot (defini dans `api_client.rs`) est remplace par
//!   `PlayerLite` (meme champs essentiels mais sans dependance serenity)
//! - Le type `ServerEvent` du bot devient `ServerEventLite`
//! - Pour le reste (chaos/classes/progression), copie identique.

pub mod chaos;
pub mod classes;
pub mod combat;
pub mod progression;

pub use combat::resolve_combat;

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
