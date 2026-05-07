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

pub mod dto;
pub mod players;
pub mod combats;
pub mod bets;
pub mod economy;
pub mod inventory;
pub mod social;
pub mod taunts;
pub mod tournaments;
pub mod curses;
pub mod vendetta;
pub mod tout_ou_rien;
pub mod travaux;
pub mod steal_roll;
pub mod steal_attempts;
pub mod flavor;
pub mod prank;
pub mod bounty;
pub mod refusal;
pub mod coalition;
pub mod ultimate;
pub mod prestige;
pub mod friendly_duel;

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
