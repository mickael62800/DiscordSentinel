use super::*;

use std::sync::Mutex;

use chrono::Duration as ChronoDuration;
use chrono::Utc;
use uuid::Uuid;

#[derive(Default)]
struct MockRepo {
    active: Mutex<Option<ActiveSafetyNet>>,
    activate_calls: Mutex<u32>,
}

#[async_trait]
impl CoudeSafetyNetRepository for MockRepo {
    async fn activate(
        &self,
        guild_id: &str,
        user_id: &str,
        duration_hours: i64,
    ) -> Result<Uuid, DomainError> {
        *self.activate_calls.lock().unwrap() += 1;
        let id = Uuid::new_v4();
        let net = ActiveSafetyNet {
            id,
            guild_id: guild_id.into(),
            user_id: user_id.into(),
            activated_at: Utc::now(),
            expires_at: Utc::now() + ChronoDuration::hours(duration_hours),
        };
        *self.active.lock().unwrap() = Some(net);
        Ok(id)
    }

    async fn get_active(
        &self,
        _guild_id: &str,
        _user_id: &str,
    ) -> Result<Option<ActiveSafetyNet>, DomainError> {
        Ok(self.active.lock().unwrap().clone())
    }

    async fn list_active(
        &self,
        _guild_id: &str,
    ) -> Result<Vec<ActiveSafetyNet>, DomainError> {
        Ok(self
            .active
            .lock()
            .unwrap()
            .clone()
            .into_iter()
            .collect())
    }
}

#[tokio::test]
async fn try_activate_below_threshold_creates_net() {
    let repo = Arc::new(MockRepo::default());
    let svc = ManageCoudeSafetyNetService::new(repo.clone());
    let net = svc.try_activate("g", "u", 10).await.unwrap();
    assert!(net.is_some());
    assert_eq!(*repo.activate_calls.lock().unwrap(), 1);
}

#[tokio::test]
async fn try_activate_above_threshold_skips() {
    let repo = Arc::new(MockRepo::default());
    let svc = ManageCoudeSafetyNetService::new(repo.clone());
    let net = svc.try_activate("g", "u", 100).await.unwrap();
    assert!(net.is_none());
    assert_eq!(*repo.activate_calls.lock().unwrap(), 0);
}

#[tokio::test]
async fn try_activate_idempotent_when_already_active() {
    let repo = Arc::new(MockRepo::default());
    let svc = ManageCoudeSafetyNetService::new(repo.clone());

    // 1ere activation OK.
    svc.try_activate("g", "u", 10).await.unwrap();
    assert_eq!(*repo.activate_calls.lock().unwrap(), 1);

    // 2eme activation -> skip car deja actif.
    let second = svc.try_activate("g", "u", 5).await.unwrap();
    assert!(second.is_none());
    assert_eq!(*repo.activate_calls.lock().unwrap(), 1);
}

#[tokio::test]
async fn try_activate_at_exact_threshold_does_not_trigger() {
    let repo = Arc::new(MockRepo::default());
    let svc = ManageCoudeSafetyNetService::new(repo.clone());
    let net = svc.try_activate("g", "u", 50).await.unwrap();
    assert!(net.is_none());
}

#[tokio::test]
async fn get_active_returns_repo_value() {
    let repo = Arc::new(MockRepo::default());
    let svc = ManageCoudeSafetyNetService::new(repo.clone());
    assert!(svc.get_active("g", "u").await.unwrap().is_none());

    svc.try_activate("g", "u", 10).await.unwrap();
    assert!(svc.get_active("g", "u").await.unwrap().is_some());
}

#[tokio::test]
async fn list_active_returns_all() {
    let repo = Arc::new(MockRepo::default());
    let svc = ManageCoudeSafetyNetService::new(repo.clone());
    assert_eq!(svc.list_active("g").await.unwrap().len(), 0);
    svc.try_activate("g", "u", 10).await.unwrap();
    assert_eq!(svc.list_active("g").await.unwrap().len(), 1);
}
