//! Handlers HTTP de Coup de Coude, éclatés par domaine métier.
//!
//! Chaque sous-module délègue à un use case (`state.coude_*_uc`) — les
//! handlers eux-mêmes sont de simples adaptateurs DTO ↔ domaine.
//!
//! Le routeur référence les handlers via `handlers::coude::{nom_handler}`,
//! le `pub use *::*` ci-dessous préserve cette API publique.

use uuid::Uuid;

use crate::adapters::inbound::http::errors::ApiError;
use sentinel_core::domain::errors::DomainError;

pub mod bets;
pub mod combats;
pub mod curses;
pub mod dto;
pub mod economy;
pub mod flavor;
pub mod friendly_duel;
pub mod inventory;
pub mod players;
pub mod prank;
pub mod refusal;
pub mod social;
pub mod steal_attempts;
pub mod steal_roll;
pub mod taunts;
pub mod tournaments;
pub mod tout_ou_rien;

/// Parse l'`id` UUID textuel reçu en path. Erreur 400 explicite si invalide.
///
/// Partagé entre `combats::*` et `bets::*` (qui référencent tous les deux
/// des combats par leur ID textuel).
pub(in crate::adapters::inbound::http::handlers::coude) fn parse_combat_id(
    raw: &str,
) -> Result<Uuid, ApiError> {
    Uuid::parse_str(raw).map_err(|_| {
        ApiError::from(DomainError::ValidationError(
            "ID de combat invalide (UUID attendu)".into(),
        ))
    })
}
