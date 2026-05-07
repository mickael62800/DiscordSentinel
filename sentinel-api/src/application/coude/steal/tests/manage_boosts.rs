use super::*;
use sentinel_core::domain::entities::system::bot_config::BotDefinition;
use sentinel_core::domain::entities::system::bot_config::BotGuildConfig;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;
use chrono::Duration as ChronoDuration;
use uuid::Uuid;

struct MockRepo {
    actives: Vec<StealBoost>,
}

#[async_trait]
impl StealBoostRepository for MockRepo {
    async fn list_active(&self, _guild_id: &str, _user_id: &str) -> Result<Vec<StealBoost>, DomainError> {
        Ok(self.actives.clone())
    }
    async fn upsert(&self, _guild_id: &str, _user_id: &str, _item_key: &str, days_to_add: i64) -> Result<DateTime<Utc>, DomainError> {
        Ok(Utc::now() + ChronoDuration::days(days_to_add))
    }
    async fn purge_expired(&self) -> Result<u64, DomainError> { Ok(0) }
}

fn mk_boost(item_key: &str) -> StealBoost {
    StealBoost {
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
    let p = svc.price_for("crochet", StealBoostDuration::OneDay).await.unwrap();
    assert_eq!(p, 60);
    let p = svc.price_for("marteau", StealBoostDuration::SevenDays).await.unwrap();
    assert_eq!(p, 2800);
}

#[tokio::test]
async fn total_bonus_sums_active_items() {
    let svc = ManageCoudeStealBoostsService::new(Arc::new(MockRepo {
        actives: vec![mk_boost("crochet"), mk_boost("marteau")],
    }));
    let total = svc.total_bonus("g", "u").await.unwrap();
    assert_eq!(total, 30);
}

#[tokio::test]
async fn total_bonus_zero_when_no_active() {
    let svc = ManageCoudeStealBoostsService::new(Arc::new(MockRepo { actives: vec![] }));
    assert_eq!(svc.total_bonus("g", "u").await.unwrap(), 0);
}

struct MockBotConfigRepo {
    cap: Option<&'static str>,
}

#[async_trait]
impl BotConfigRepository for MockBotConfigRepo {
    async fn get_definitions(&self) -> Result<Vec<BotDefinition>, DomainError> { Ok(vec![]) }
    async fn get_config(&self, _guild_id: &str, _bot_name: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
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
    async fn get_all_config(&self, _guild_id: &str) -> Result<Vec<BotGuildConfig>, DomainError> { Ok(vec![]) }
    async fn set_config(&self, _guild_id: &str, _bot_name: &str, _key: &str, _value: &str) -> Result<(), DomainError> { Ok(()) }
    async fn delete_config(&self, _guild_id: &str, _bot_name: &str, _key: &str) -> Result<(), DomainError> { Ok(()) }
}

#[tokio::test]
async fn subscribe_refuse_quand_cap_atteint() {
    let svc = ManageCoudeStealBoostsService::new(Arc::new(MockRepo {
        actives: vec![mk_boost("crochet"), mk_boost("marteau")],
    }))
    .with_bot_config_repo(Arc::new(MockBotConfigRepo { cap: Some("2") }));
    let err = svc.subscribe("g", "u", "fumigene", StealBoostDuration::OneDay).await.unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(_)));
}

#[tokio::test]
async fn subscribe_autorise_re_souscription_meme_au_cap() {
    let svc = ManageCoudeStealBoostsService::new(Arc::new(MockRepo {
        actives: vec![mk_boost("crochet"), mk_boost("marteau")],
    }))
    .with_bot_config_repo(Arc::new(MockBotConfigRepo { cap: Some("2") }));
    let res = svc.subscribe("g", "u", "crochet", StealBoostDuration::OneDay).await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn subscribe_cap_zero_ne_bloque_jamais() {
    let svc = ManageCoudeStealBoostsService::new(Arc::new(MockRepo {
        actives: vec![
            mk_boost("crochet"), mk_boost("marteau"),
            mk_boost("fumigene"), mk_boost("passe_partout"),
        ],
    }))
    .with_bot_config_repo(Arc::new(MockBotConfigRepo { cap: Some("0") }));
    let res = svc.subscribe("g", "u", "deguisement", StealBoostDuration::OneDay).await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn subscribe_sans_config_repo_applique_default_3() {
    let svc = ManageCoudeStealBoostsService::new(Arc::new(MockRepo {
        actives: vec![mk_boost("marteau"), mk_boost("fumigene")],
    }));
    let res = svc.subscribe("g", "u", "crochet", StealBoostDuration::OneDay).await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn list_active_passes_through() {
    let svc = ManageCoudeStealBoostsService::new(Arc::new(MockRepo {
        actives: vec![mk_boost("crochet")],
    }));
    let list = svc.list_active("g", "u").await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].item_key, "crochet");
}

#[tokio::test]
async fn price_for_unknown_item_returns_validation_error() {
    let svc = ManageCoudeStealBoostsService::new(Arc::new(MockRepo { actives: vec![] }));
    let err = svc.price_for("wibble", StealBoostDuration::OneDay).await.unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(_)));
}

#[tokio::test]
async fn subscribe_unknown_item_returns_validation_error() {
    let svc = ManageCoudeStealBoostsService::new(Arc::new(MockRepo { actives: vec![] }));
    let err = svc.subscribe("g", "u", "wibble", StealBoostDuration::OneDay).await.unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(_)));
}

#[tokio::test]
async fn price_for_all_durations_returns_positive() {
    let svc = ManageCoudeStealBoostsService::new(Arc::new(MockRepo { actives: vec![] }));
    for d in [StealBoostDuration::OneDay, StealBoostDuration::ThreeDays, StealBoostDuration::SevenDays] {
        let p = svc.price_for("crochet", d).await.unwrap();
        assert!(p > 0);
    }
}

#[tokio::test]
async fn subscribe_cap_config_invalid_fallback_to_default() {
    // Config avec valeur non-parseable → fallback au defaut (3).
    let svc = ManageCoudeStealBoostsService::new(Arc::new(MockRepo {
        actives: vec![mk_boost("crochet"), mk_boost("marteau")],
    }))
    .with_bot_config_repo(Arc::new(MockBotConfigRepo { cap: Some("not_a_number") }));
    // Default 3 : 2 actifs, ajout OK.
    let res = svc.subscribe("g", "u", "fumigene", StealBoostDuration::OneDay).await;
    assert!(res.is_ok());
}
