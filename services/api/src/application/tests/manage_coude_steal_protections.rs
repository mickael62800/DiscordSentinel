use super::*;
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
    let svc = ManageCoudeStealProtectionsService::new(Arc::new(MockRepo { actives: vec![] }));
    let price = svc.price_for("chien_garde", StealProtectionDuration::OneDay).await.unwrap();
    assert_eq!(price, 50);
    let price = svc.price_for("chien_garde", StealProtectionDuration::SevenDays).await.unwrap();
    assert_eq!(price, 280);
}

#[tokio::test]
async fn price_for_unknown_item_errors() {
    let svc = ManageCoudeStealProtectionsService::new(Arc::new(MockRepo { actives: vec![] }));
    let err = svc.price_for("unknown", StealProtectionDuration::OneDay).await.unwrap_err();
    matches!(err, DomainError::ValidationError(_));
}

#[tokio::test]
async fn try_trigger_no_protection_returns_none() {
    let svc = ManageCoudeStealProtectionsService::new(Arc::new(MockRepo { actives: vec![] }));
    let out = svc.try_trigger("g", "u").await.unwrap();
    assert!(out.is_none());
}

#[tokio::test]
async fn try_trigger_with_forteresse_high_probability() {
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
