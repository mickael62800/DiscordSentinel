use super::*;
use chrono::Duration as ChronoDuration;
use uuid::Uuid;

struct MockRepo {
    actives: Vec<StealProtection>,
}

#[async_trait]
impl StealProtectionRepository for MockRepo {
    async fn list_active(
        &self,
        _guild_id: &str,
        _user_id: &str,
    ) -> Result<Vec<StealProtection>, DomainError> {
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

fn mk_protection(item_key: &str) -> StealProtection {
    StealProtection {
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
    let price = svc
        .price_for("chien_garde", StealProtectionDuration::OneDay)
        .await
        .unwrap();
    assert_eq!(price, 50);
    let price = svc
        .price_for("chien_garde", StealProtectionDuration::SevenDays)
        .await
        .unwrap();
    assert_eq!(price, 280);
}

#[tokio::test]
async fn price_for_unknown_item_errors() {
    let svc = ManageCoudeStealProtectionsService::new(Arc::new(MockRepo { actives: vec![] }));
    let err = svc
        .price_for("unknown", StealProtectionDuration::OneDay)
        .await
        .unwrap_err();
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
    assert!(
        any_blocked,
        "forteresse n'a jamais bloque sur 50 essais (tres improbable)"
    );
}

#[tokio::test]
async fn list_active_passes_through() {
    let svc = ManageCoudeStealProtectionsService::new(Arc::new(MockRepo {
        actives: vec![mk_protection("coffre_fort"), mk_protection("chien_garde")],
    }));
    let list = svc.list_active("g", "u").await.unwrap();
    assert_eq!(list.len(), 2);
}

#[tokio::test]
async fn subscribe_known_item_returns_future_date() {
    let svc = ManageCoudeStealProtectionsService::new(Arc::new(MockRepo { actives: vec![] }));
    let expires = svc
        .subscribe("g", "u", "chien_garde", StealProtectionDuration::SevenDays)
        .await
        .unwrap();
    assert!(expires > Utc::now());
}

#[tokio::test]
async fn subscribe_unknown_item_returns_validation_error() {
    let svc = ManageCoudeStealProtectionsService::new(Arc::new(MockRepo { actives: vec![] }));
    let err = svc
        .subscribe("g", "u", "wibble", StealProtectionDuration::OneDay)
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(_)));
}

#[tokio::test]
async fn price_for_all_durations() {
    let svc = ManageCoudeStealProtectionsService::new(Arc::new(MockRepo { actives: vec![] }));
    for duration in [
        StealProtectionDuration::OneDay,
        StealProtectionDuration::ThreeDays,
        StealProtectionDuration::FiveDays,
        StealProtectionDuration::SevenDays,
    ] {
        let p = svc.price_for("chien_garde", duration).await.unwrap();
        assert!(p > 0);
    }
}

#[tokio::test]
async fn try_trigger_skips_non_active_items() {
    // Un seul item actif (chien_garde = lowest block chance), les autres
    // ne sont pas roll meme s'ils sont dans le catalogue.
    let svc = ManageCoudeStealProtectionsService::new(Arc::new(MockRepo {
        actives: vec![mk_protection("chien_garde")],
    }));
    // Sur 200 iterations, on attend statistiquement qu'au moins 1 echec (miss).
    let mut misses = 0;
    for _ in 0..200 {
        if svc.try_trigger("g", "u").await.unwrap().is_none() {
            misses += 1;
        }
    }
    // chien_garde a une block_chance < 100%, donc quelques miss attendus.
    assert!(
        misses > 0,
        "chien_garde devrait louper au moins une fois sur 200"
    );
}
