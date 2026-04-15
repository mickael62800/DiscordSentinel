//! Impl des abonnements boost voleur (Phase 9 Part C).

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::entities::{
    find_boost_item, sum_roll_bonus_for_active_keys, CoudeStealBoost, StealBoostDuration,
};
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_coude_steal_boosts::ManageCoudeStealBoostsUseCase;
use crate::ports::outbound::CoudeStealBoostRepository;

pub struct ManageCoudeStealBoostsService {
    repo: Arc<dyn CoudeStealBoostRepository>,
}

impl ManageCoudeStealBoostsService {
    pub fn new(repo: Arc<dyn CoudeStealBoostRepository>) -> Self {
        Self { repo }
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
mod tests {
    use super::*;
    use chrono::Duration as ChronoDuration;
    use uuid::Uuid;

    struct MockRepo {
        actives: Vec<CoudeStealBoost>,
    }

    #[async_trait]
    impl CoudeStealBoostRepository for MockRepo {
        async fn list_active(
            &self,
            _guild_id: &str,
            _user_id: &str,
        ) -> Result<Vec<CoudeStealBoost>, DomainError> {
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

    fn mk_boost(item_key: &str) -> CoudeStealBoost {
        CoudeStealBoost {
            id: Uuid::new_v4(),
            guild_id: "g".into(),
            user_id: "u".into(),
            item_key: item_key.into(),
            expires_at: Utc::now() + ChronoDuration::days(7),
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn price_for_known_item_uses_grid() {
        let svc = ManageCoudeStealBoostsService::new(Arc::new(MockRepo { actives: vec![] }));
        // crochet : 60/jour * 1 = 60
        let p = svc
            .price_for("crochet", StealBoostDuration::OneDay)
            .await
            .unwrap();
        assert_eq!(p, 60);
        // marteau : 500/jour * 5.6 = 2800
        let p = svc
            .price_for("marteau", StealBoostDuration::SevenDays)
            .await
            .unwrap();
        assert_eq!(p, 2800);
    }

    #[tokio::test]
    async fn total_bonus_sums_active_items() {
        let svc = ManageCoudeStealBoostsService::new(Arc::new(MockRepo {
            actives: vec![mk_boost("crochet"), mk_boost("marteau")],
        }));
        let total = svc.total_bonus("g", "u").await.unwrap();
        // 5 (crochet) + 25 (marteau) = 30
        assert_eq!(total, 30);
    }

    #[tokio::test]
    async fn total_bonus_zero_when_no_active() {
        let svc = ManageCoudeStealBoostsService::new(Arc::new(MockRepo { actives: vec![] }));
        assert_eq!(svc.total_bonus("g", "u").await.unwrap(), 0);
    }
}
