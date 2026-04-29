//! Impl des abonnements anti-vol (Phase 9 Part B).

use std::sync::Arc;

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use rand::Rng;

use crate::domain::entities::coude::steal::protection::find_protection_item;
use crate::domain::entities::coude::steal::protection::StealProtection;
use crate::domain::entities::coude::steal::protection::StealProtectionDuration;
use crate::domain::entities::coude::steal::protection::STEAL_PROTECTION_ITEMS;
use crate::domain::errors::DomainError;
use crate::ports::inbound::coude::manage_steal_protections::ManageCoudeStealProtectionsUseCase;
use crate::ports::inbound::coude::manage_steal_protections::StealProtectionTrigger;
use crate::ports::outbound::coude::steal_protection_repository::StealProtectionRepository;

pub struct ManageCoudeStealProtectionsService {
    repo: Arc<dyn StealProtectionRepository>,
}

impl ManageCoudeStealProtectionsService {
    pub fn new(repo: Arc<dyn StealProtectionRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageCoudeStealProtectionsUseCase for ManageCoudeStealProtectionsService {
    async fn list_active(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<StealProtection>, DomainError> {
        self.repo.list_active(guild_id, user_id).await
    }

    async fn price_for(
        &self,
        item_key: &str,
        duration: StealProtectionDuration,
    ) -> Result<i64, DomainError> {
        let item = find_protection_item(item_key).ok_or_else(|| {
            DomainError::ValidationError(format!("Item de protection inconnu : {item_key}"))
        })?;
        Ok(duration.total_cost(item.base_cost_per_day))
    }

    async fn subscribe(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
        duration: StealProtectionDuration,
    ) -> Result<DateTime<Utc>, DomainError> {
        // Valide que l'item existe dans le catalogue.
        if find_protection_item(item_key).is_none() {
            return Err(DomainError::ValidationError(format!(
                "Item de protection inconnu : {item_key}"
            )));
        }
        self.repo
            .upsert(guild_id, user_id, item_key, duration.days())
            .await
    }

    async fn try_trigger(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<Option<StealProtectionTrigger>, DomainError> {
        let actives = self.repo.list_active(guild_id, target_id).await?;
        if actives.is_empty() {
            return Ok(None);
        }

        // Index les actives par item_key pour une lookup rapide.
        let active_keys: std::collections::HashSet<&str> =
            actives.iter().map(|p| p.item_key.as_str()).collect();

        // Iter le catalogue de l'item LE PLUS efficace vers le moins, pour
        // que le coffre_fort roll avant chien_garde. Le premier blocage
        // arrete la chaine.
        let mut items_desc: Vec<_> = STEAL_PROTECTION_ITEMS.iter().collect();
        items_desc.sort_by(|a, b| b.block_chance_percent.cmp(&a.block_chance_percent));

        // Scope le ThreadRng avant l'Ok final pour satisfaire Send.
        let trigger = {
            let mut rng = rand::thread_rng();
            let mut triggered: Option<StealProtectionTrigger> = None;
            for item in items_desc {
                if !active_keys.contains(item.key) {
                    continue;
                }
                let roll: u32 = rng.gen_range(1..=100);
                if roll <= item.block_chance_percent {
                    triggered = Some(StealProtectionTrigger {
                        item_key: item.key.to_string(),
                        item_name: item.name.to_string(),
                        rolled_value: roll,
                        block_chance_percent: item.block_chance_percent,
                    });
                    break;
                }
            }
            triggered
        };

        Ok(trigger)
    }
}

#[cfg(test)]
#[path = "tests/manage_steal_protections.rs"]
mod tests;
