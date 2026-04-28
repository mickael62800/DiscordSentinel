//! Handlers HTTP Blackjack, éclatés entre :
//! - `dto` : DTOs requête/réponse + helpers de conversion domaine → HTTP
//! - `game` : cycle de vie d'une partie solo (start/hit/stand/double/get_active)
//! - `tables` : multiplayer — tables, joueurs, clôture, listing des parties
//!
//! Le `pub use` au bas de ce fichier préserve l'API `handlers::blackjack::*`
//! attendue par `router.rs`.

use uuid::Uuid;

use crate::adapters::inbound::http::errors::ApiError;
use crate::domain::errors::DomainError;

pub mod dto;
pub mod game;
pub mod tables;

/// Parse un UUID de game_id. Erreur 400 explicite si invalide.
pub(in crate::adapters::inbound::http::handlers::casino::blackjack) fn parse_uuid(
    s: &str,
) -> Result<Uuid, ApiError> {
    Uuid::parse_str(s).map_err(|_| {
        ApiError::from(DomainError::ValidationError(
            "ID de partie invalide.".into(),
        ))
    })
}
