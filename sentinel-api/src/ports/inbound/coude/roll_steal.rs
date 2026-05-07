//! Use case `roll_steal` (Phase 2 #4 audit).
//!
//! Tire les d20 thief/victim + le % de wallet vole, cote serveur, pour
//! que la decision RNG soit auditable. Le bot consomme ces valeurs et
//! garde toute la presentation (templates, embeds, calcul des bonus
//! class/DEF/boost).

use async_trait::async_trait;

use sentinel_core::domain::errors::DomainError;

#[derive(Debug, Clone)]
pub struct RollStealCommand {
    /// Cible AFK (n'a pas clique sur "Se defendre"). Influe le % stolen.
    pub afk: bool,
}

#[derive(Debug, Clone)]
pub struct StealRoll {
    /// d20 du voleur, dans [1, 20].
    pub thief_d20: i32,
    /// d20 de la cible, dans [1, 20].
    pub victim_d20: i32,
    /// % de wallet vole, en basis points (1bp = 0.01%). Le bot divise
    /// par 10_000 pour obtenir le ratio applique au solde de la victime.
    pub steal_pct_bp: u32,
}

#[async_trait]
pub trait RollStealUseCase: Send + Sync {
    async fn roll(&self, cmd: RollStealCommand) -> Result<StealRoll, DomainError>;
}
