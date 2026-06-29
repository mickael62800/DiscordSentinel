//! Domain Game Portal — entites pures de la plateforme de jeux.
//!
//! Aucune dependance infrastructure ici (pas de sqlx, pas de bollard,
//! pas de reqwest). Logique metier + types.

pub mod audit;
pub mod config;
pub mod player_session;
pub mod quota;
pub mod server;
pub mod template;
