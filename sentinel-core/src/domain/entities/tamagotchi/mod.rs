//! Domaine du jeu Tamagotchi (compagnon virtuel).
//!
//! Jeu independant : stats/combat/ELO propres. Seuls les coins sont partages
//! (wallet commun). Ce module contient les entites et la logique pure
//! (decroissance des jauges, maladie, mort) — testable sans I/O.

pub mod pet;
pub mod species;

pub use pet::*;
pub use species::*;
