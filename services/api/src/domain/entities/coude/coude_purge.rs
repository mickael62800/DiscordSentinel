//! Regle metier : quelles tables sont videes quand un admin purge
//! toutes les donnees Coup de Coude d'une guild (operation irreversible,
//! double-check frontend obligatoire).
//!
//! Ordre important : tables filles d'abord. Meme si la plupart ont
//! CASCADE, on explicite l'ordre pour rester lisible et deterministe.

/// Tables purgees par `DELETE /api/coude/{guild_id}/purge`, dans l'ordre
/// d'execution.
pub const COUDE_PURGE_TABLES: &[&str] = &[
    "coude_insurances",
    "coude_bets",
    "coude_combats",
    "coude_primes",
    "coude_inventory",
    "coude_events",
    "coude_players",
];

#[cfg(test)]
#[path = "tests/coude_purge.rs"]
mod tests;
