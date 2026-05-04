//! Domaine coude (Coup de Coude / Coude Game) : 6 jobs metier du jeu.
//!   - expire_combats     : timeouts des combats /coude pending
//!   - resolve_betting    : resolution des paris pending
//!   - hp_regen           : regeneration HP passive
//!   - resolve_tournament : resolution du tournoi hebdo
//!   - redistribute_cashbox : redistribution caisse communautaire
//!   - daily_chaos        : "Roue du Destin" (delay aleatoire 2-6h)
//!
//! Porte de coude-worker (Phase 3 fusion). Le job `daily_chaos` a un
//! delay aleatoire (pas un interval fixe) -> il garde sa propre task
//! comme dans l'ancien scheduler.

pub mod daily_chaos;
pub mod expire_combats;
pub mod hp_regen;
pub mod redistribute_cashbox;
pub mod resolve_betting;
pub mod resolve_tournament;
