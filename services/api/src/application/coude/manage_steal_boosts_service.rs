//! Impl des abonnements boost voleur (Phase 9 Part C).

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::entities::{
    find_boost_item, sum_roll_bonus_for_active_keys, CoudeBalanceParams, CoudeStealBoost,
    StealBoostDuration,
};
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_steal_boosts::ManageCoudeStealBoostsUseCase;
use crate::ports::outbound::{BotConfigRepository, CoudeStealBoostRepository};

pub struct ManageCoudeStealBoostsService {
    repo: Arc<dyn CoudeStealBoostRepository>,
    bot_config_repo: Option<Arc<dyn BotConfigRepository>>,
}

impl ManageCoudeStealBoostsService {
    pub fn new(repo: Arc<dyn CoudeStealBoostRepository>) -> Self {
        Self {
            repo,
            bot_config_repo: None,
        }
    }

    /// Branche la lecture de `bot_guild_config` pour appliquer le gate
    /// `steal_max_active_boosts`. Optionnel : sans config_repo, le cap
    /// n'est pas applique (comportement historique).
    pub fn with_bot_config_repo(
        mut self,
        bot_config_repo: Arc<dyn BotConfigRepository>,
    ) -> Self {
        self.bot_config_repo = Some(bot_config_repo);
        self
    }

    async fn load_balance(&self, guild_id: &str) -> CoudeBalanceParams {
        let Some(repo) = self.bot_config_repo.as_ref() else {
            return CoudeBalanceParams::default();
        };
        crate::application::guild_settings::load_balance_params(&**repo, guild_id).await
    }
}

#[async_trait]
impl ManageCoudeStealBoostsUseCase for ManageCoudeStealBoostsService {
    async fn list_active(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<CoudeStealBoost>, DomainError> {
        self.repo.list_active(guild_id, user_id).await
    }

    async fn price_for(
        &self,
        item_key: &str,
        duration: StealBoostDuration,
    ) -> Result<i64, DomainError> {
        let item = find_boost_item(item_key).ok_or_else(|| {
            DomainError::ValidationError(format!("Item de boost inconnu : {item_key}"))
        })?;
        Ok(duration.total_cost(item.base_cost_per_day))
    }

    async fn subscribe(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
        duration: StealBoostDuration,
    ) -> Result<DateTime<Utc>, DomainError> {
        if find_boost_item(item_key).is_none() {
            return Err(DomainError::ValidationError(format!(
                "Item de boost inconnu : {item_key}"
            )));
        }

        // Gate : cap sur le nombre de boosts actifs simultanes.
        // On ne compte pas l'item courant (re-souscription = prolongation),
        // seulement les autres deja actifs. `cap = 0` signifie illimite.
        let params = self.load_balance(guild_id).await;
        let cap = params.steal_max_active_boosts;
        if cap > 0 {
            let actives = self.repo.list_active(guild_id, user_id).await?;
            let other_active = actives
                .iter()
                .filter(|b| b.item_key != item_key)
                .count() as u64;
            if other_active >= cap {
                return Err(DomainError::ValidationError(format!(
                    "Trop de boosts actifs ({other_active}/{cap}). Attends qu'un boost expire avant d'en souscrire un nouveau."
                )));
            }
        }

        self.repo
            .upsert(guild_id, user_id, item_key, duration.days())
            .await
    }

    async fn total_bonus(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i32, DomainError> {
        let actives = self.repo.list_active(guild_id, user_id).await?;
        let keys: Vec<&str> = actives.iter().map(|b| b.item_key.as_str()).collect();
        Ok(sum_roll_bonus_for_active_keys(keys))
    }
}

#[cfg(test)]
#[path = "tests/manage_steal_boosts.rs"]
mod tests;
