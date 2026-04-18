//! Impl des abonnements boost voleur (Phase 9 Part C).

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::entities::{
    find_boost_item, sum_roll_bonus_for_active_keys, CoudeBalanceParams, CoudeStealBoost,
    StealBoostDuration,
};
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_coude_steal_boosts::ManageCoudeStealBoostsUseCase;
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
        match repo.get_config(guild_id, "coude-bot").await {
            Ok(entries) => {
                let map: std::collections::HashMap<String, String> = entries
                    .into_iter()
                    .map(|e| (e.config_key, e.config_value))
                    .collect();
                CoudeBalanceParams::from_config(&map)
            }
            Err(_) => CoudeBalanceParams::default(),
        }
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

    // ── Gate steal_max_active_boosts ──

    use crate::domain::entities::{BotDefinition, BotGuildConfig};
    use crate::ports::outbound::BotConfigRepository;

    struct MockBotConfigRepo {
        cap: Option<&'static str>, // valeur de steal_max_active_boosts
    }

    #[async_trait]
    impl BotConfigRepository for MockBotConfigRepo {
        async fn get_definitions(&self) -> Result<Vec<BotDefinition>, DomainError> {
            Ok(vec![])
        }
        async fn get_config(
            &self,
            _guild_id: &str,
            _bot_name: &str,
        ) -> Result<Vec<BotGuildConfig>, DomainError> {
            let mut out = Vec::new();
            if let Some(cap) = self.cap {
                out.push(BotGuildConfig {
                    id: Uuid::new_v4(),
                    guild_id: "g".into(),
                    bot_name: "coude-bot".into(),
                    config_key: "steal_max_active_boosts".into(),
                    config_value: cap.into(),
                    updated_at: Utc::now(),
                });
            }
            Ok(out)
        }
        async fn get_all_config(
            &self,
            _guild_id: &str,
        ) -> Result<Vec<BotGuildConfig>, DomainError> {
            Ok(vec![])
        }
        async fn set_config(
            &self,
            _guild_id: &str,
            _bot_name: &str,
            _key: &str,
            _value: &str,
        ) -> Result<(), DomainError> {
            Ok(())
        }
        async fn delete_config(
            &self,
            _guild_id: &str,
            _bot_name: &str,
            _key: &str,
        ) -> Result<(), DomainError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn subscribe_refuse_quand_cap_atteint() {
        // Cap = 2, 2 boosts actifs (autres que celui demande) → refus.
        let svc = ManageCoudeStealBoostsService::new(Arc::new(MockRepo {
            actives: vec![mk_boost("crochet"), mk_boost("marteau")],
        }))
        .with_bot_config_repo(Arc::new(MockBotConfigRepo { cap: Some("2") }));
        let err = svc
            .subscribe("g", "u", "fumigene", StealBoostDuration::OneDay)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::ValidationError(_)));
    }

    #[tokio::test]
    async fn subscribe_autorise_re_souscription_meme_au_cap() {
        // Cap = 2, 2 actifs mais on re-souscrit `crochet` (deja actif) →
        // autorise (ne compte pas le re-subscribe).
        let svc = ManageCoudeStealBoostsService::new(Arc::new(MockRepo {
            actives: vec![mk_boost("crochet"), mk_boost("marteau")],
        }))
        .with_bot_config_repo(Arc::new(MockBotConfigRepo { cap: Some("2") }));
        let res = svc
            .subscribe("g", "u", "crochet", StealBoostDuration::OneDay)
            .await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn subscribe_cap_zero_ne_bloque_jamais() {
        let svc = ManageCoudeStealBoostsService::new(Arc::new(MockRepo {
            actives: vec![
                mk_boost("crochet"),
                mk_boost("marteau"),
                mk_boost("fumigene"),
                mk_boost("passe_partout"),
            ],
        }))
        .with_bot_config_repo(Arc::new(MockBotConfigRepo { cap: Some("0") }));
        let res = svc
            .subscribe("g", "u", "deguisement", StealBoostDuration::OneDay)
            .await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn subscribe_sans_config_repo_applique_default_3() {
        // Sans bot_config_repo branche → fallback CoudeBalanceParams::default()
        // qui impose un cap de 3. 2 actifs (autres) → OK.
        let svc = ManageCoudeStealBoostsService::new(Arc::new(MockRepo {
            actives: vec![mk_boost("marteau"), mk_boost("fumigene")],
        }));
        let res = svc
            .subscribe("g", "u", "crochet", StealBoostDuration::OneDay)
            .await;
        assert!(res.is_ok());
    }
}
