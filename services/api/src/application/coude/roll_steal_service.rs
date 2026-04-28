//! Service `RollStealUseCase` (Phase 2 #4 audit).
//!
//! Stateless : tire 2 d20 + 1 % uniforme dans la plage AFK ou active.
//! La transfer effective est faite ensuite par `record_steal` (qui
//! existe deja). Le but de cet endpoint est juste de retirer le RNG
//! du bot pour le rendre auditable cote serveur.

use async_trait::async_trait;
use rand::Rng;

use crate::domain::entities::coude::steal_roll::steal_pct_range_bp;
use crate::domain::entities::coude::steal_roll::STEAL_D20_MAX;
use crate::domain::entities::coude::steal_roll::STEAL_D20_MIN;
use crate::domain::errors::DomainError;
use crate::ports::inbound::coude::roll_steal::RollStealCommand;
use crate::ports::inbound::coude::roll_steal::RollStealUseCase;
use crate::ports::inbound::coude::roll_steal::StealRoll;
pub struct RollStealService;

impl RollStealService {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RollStealService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RollStealUseCase for RollStealService {
    async fn roll(&self, cmd: RollStealCommand) -> Result<StealRoll, DomainError> {
        let (thief_d20, victim_d20, steal_pct_bp) = {
            let mut rng = rand::thread_rng();
            let thief = rng.gen_range(STEAL_D20_MIN..=STEAL_D20_MAX);
            let victim = rng.gen_range(STEAL_D20_MIN..=STEAL_D20_MAX);
            let (lo, hi) = steal_pct_range_bp(cmd.afk);
            let pct = rng.gen_range(lo..=hi);
            (thief, victim, pct)
        };
        Ok(StealRoll { thief_d20, victim_d20, steal_pct_bp })
    }
}

#[cfg(test)]
#[path = "tests/roll_steal.rs"]
mod tests;
