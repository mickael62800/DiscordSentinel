//! Impl des abonnements anti-vol (Phase 9 Part B).

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rand::Rng;

use crate::domain::entities::{
    find_protection_item, CoudeStealProtection, StealProtectionDuration, STEAL_PROTECTION_ITEMS,
};
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_coude_steal_protections::{
    ManageCoudeStealProtectionsUseCase, StealProtectionTrigger,
};
use crate::ports::outbound::CoudeStealProtectionRepository;

pub struct ManageCoudeStealProtectionsService {
    repo: Arc<dyn CoudeStealProtectionRepository>,
}

impl ManageCoudeStealProtectionsService {
    pub fn new(repo: Arc<dyn CoudeStealProtectionRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageCoudeStealProtectionsUseCase for ManageCoudeStealProtectionsService {
    async fn list_active(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<CoudeStealProtection>, DomainError> {
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
mod tests {
    use super::*;
    use crate::domain::entities::CoudeStealProtection;
    use chrono::Duration as ChronoDuration;
    use uuid::Uuid;

    struct MockRepo {
        actives: Vec<CoudeStealProtection>,
    }

    #[async_trait]
    impl CoudeStealProtectionRepository for MockRepo {
        async fn list_active(
            &self,
            _guild_id: &str,
            _user_id: &str,
        ) -> Result<Vec<CoudeStealProtection>, DomainError> {
            Ok(self.actives.clone())
        }

        async fn upsert(
            &self,
            _guild_id: &str,
            _user_id: &str,
            _item_key: &str,
            days_to_add: i64,
        ) -> Result<DateTime<Utc>, DomainError> {
            Ok(Utc::now() + ChronoDuration::days(days_to_add))
        }

        async fn purge_expired(&self) -> Result<u64, DomainError> {
            Ok(0)
        }
    }

    fn mk_protection(item_key: &str) -> CoudeStealProtection {
        CoudeStealProtection {
            id: Uuid::new_v4(),
            guild_id: "g".into(),
            user_id: "u".into(),
            item_key: item_key.into(),
            expires_at: Utc::now() + ChronoDuration::days(7),
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn price_for_known_item_returns_grid() {
        let svc = ManageCoudeStealProtectionsService::new(Arc::new(MockRepo {
            actives: vec![],
        }));
        let price = svc
            .price_for("chien_garde", StealProtectionDuration::OneDay)
            .await
            .unwrap();
        // chien_garde : 50/jour * 1 = 50
        assert_eq!(price, 50);

        let price = svc
            .price_for("chien_garde", StealProtectionDuration::SevenDays)
            .await
            .unwrap();
        // chien_garde : 50/jour * 5.6 = 280
        assert_eq!(price, 280);
    }

    #[tokio::test]
    async fn price_for_unknown_item_errors() {
        let svc = ManageCoudeStealProtectionsService::new(Arc::new(MockRepo {
            actives: vec![],
        }));
        let err = svc
            .price_for("unknown", StealProtectionDuration::OneDay)
            .await
            .unwrap_err();
        matches!(err, DomainError::ValidationError(_));
    }

    #[tokio::test]
    async fn try_trigger_no_protection_returns_none() {
        let svc = ManageCoudeStealProtectionsService::new(Arc::new(MockRepo {
            actives: vec![],
        }));
        let out = svc.try_trigger("g", "u").await.unwrap();
        assert!(out.is_none());
    }

    #[tokio::test]
    async fn try_trigger_with_forteresse_high_probability() {
        // forteresse = 70% : sur 200 runs, on s'attend a bloquer dans ~70% des cas.
        // On teste juste que le trigger peut retourner Some au moins une fois.
        let svc = ManageCoudeStealProtectionsService::new(Arc::new(MockRepo {
            actives: vec![mk_protection("forteresse")],
        }));
        let mut any_blocked = false;
        for _ in 0..50 {
            if svc.try_trigger("g", "u").await.unwrap().is_some() {
                any_blocked = true;
                break;
            }
        }
        assert!(any_blocked, "forteresse n'a jamais bloque sur 50 essais (tres improbable)");
    }
}
