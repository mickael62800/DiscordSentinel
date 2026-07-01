//! Service `RollStealUseCase` (Phase 2 #4 audit).
//!
//! Stateless : tire 2 d20 + 1 % uniforme dans la plage AFK ou active.
//! La transfer effective est faite ensuite par `record_steal` (qui
//! existe deja). Le but de cet endpoint est juste de retirer le RNG
//! du bot pour le rendre auditable cote serveur.

use std::sync::Arc;

use async_trait::async_trait;
use rand::Rng;

use crate::domain::entities::coude::economy_config::CoudeEconomyConfig;
use crate::domain::entities::coude::steal::roll::steal_pct_range_bp;
use crate::domain::entities::coude::steal::roll::STEAL_D20_MAX;
use crate::domain::entities::coude::steal::roll::STEAL_D20_MIN;
use crate::domain::errors::DomainError;
use crate::ports::inbound::coude::roll_steal::RollStealCommand;
use crate::ports::inbound::coude::roll_steal::RollStealUseCase;
use crate::ports::inbound::coude::roll_steal::StealRoll;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;
#[derive(Default)]
pub struct RollStealService {
    bot_config_repo: Option<Arc<dyn BotConfigRepository>>,
}

impl RollStealService {
    pub fn new() -> Self {
        Self {
            bot_config_repo: None,
        }
    }

    /// Branche le repo de config bot : les bornes de % volé (AFK/actif)
    /// deviennent réglables par serveur via `coude-bot`. Sans repo :
    /// bornes par défaut historiques.
    pub fn with_bot_config_repo(mut self, repo: Arc<dyn BotConfigRepository>) -> Self {
        self.bot_config_repo = Some(repo);
        self
    }

    async fn load_economy(&self, guild_id: &str) -> CoudeEconomyConfig {
        match &self.bot_config_repo {
            Some(repo) => {
                crate::application::coude::guild_settings::load_economy_config(&**repo, guild_id)
                    .await
            }
            None => CoudeEconomyConfig::default(),
        }
    }
}

#[async_trait]
impl RollStealUseCase for RollStealService {
    async fn roll(&self, cmd: RollStealCommand) -> Result<StealRoll, DomainError> {
        let econ = self.load_economy(&cmd.guild_id).await;
        let (thief_d20, victim_d20, steal_pct_bp) = {
            let mut rng = rand::thread_rng();
            let thief = rng.gen_range(STEAL_D20_MIN..=STEAL_D20_MAX);
            let victim = rng.gen_range(STEAL_D20_MIN..=STEAL_D20_MAX);
            let (lo, hi) = steal_pct_range_bp(cmd.afk, &econ);
            let pct = rng.gen_range(lo..=hi);
            (thief, victim, pct)
        };
        Ok(StealRoll {
            thief_d20,
            victim_d20,
            steal_pct_bp,
        })
    }
}

// Tests supprimes lors du refactor steal/protection - voir tests/manage_protections.rs
