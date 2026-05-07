use super::*;

use std::sync::Mutex;

use chrono::Duration as ChronoDuration;
use chrono::Utc;
use uuid::Uuid;

use sentinel_core::domain::entities::coude::vendetta::VendettaStatus;

#[derive(Default)]
struct MockRepo {
    rows: Mutex<Vec<ActiveVendetta>>,
    declare_calls: Mutex<u32>,
}

#[async_trait]
impl VendettaRepository for MockRepo {
    async fn declare(
        &self,
        guild_id: &str,
        challenger_id: &str,
        target_id: &str,
        duration_hours: i64,
    ) -> Result<Uuid, DomainError> {
        *self.declare_calls.lock().unwrap() += 1;
        let id = Uuid::new_v4();
        self.rows.lock().unwrap().push(ActiveVendetta {
            id,
            guild_id: guild_id.into(),
            challenger_id: challenger_id.into(),
            target_id: target_id.into(),
            declared_at: Utc::now(),
            expires_at: Utc::now() + ChronoDuration::hours(duration_hours),
            status: VendettaStatus::Active,
            resolved_at: None,
        });
        Ok(id)
    }

    async fn get_active(
        &self,
        guild_id: &str,
        challenger_id: &str,
        target_id: &str,
    ) -> Result<Option<ActiveVendetta>, DomainError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .find(|v| {
                v.guild_id == guild_id
                    && v.challenger_id == challenger_id
                    && v.target_id == target_id
                    && v.status == VendettaStatus::Active
                    && v.expires_at > Utc::now()
            })
            .cloned())
    }

    async fn resolve(&self, id: Uuid, won: bool) -> Result<(), DomainError> {
        let mut rows = self.rows.lock().unwrap();
        let Some(v) = rows.iter_mut().find(|v| v.id == id) else {
            return Err(DomainError::Conflict("introuvable".into()));
        };
        if v.status != VendettaStatus::Active {
            return Err(DomainError::Conflict("deja resolue".into()));
        }
        v.status = if won {
            VendettaStatus::Won
        } else {
            VendettaStatus::Lost
        };
        v.resolved_at = Some(Utc::now());
        Ok(())
    }

    async fn list_by_challenger(
        &self,
        guild_id: &str,
        challenger_id: &str,
    ) -> Result<Vec<ActiveVendetta>, DomainError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .filter(|v| v.guild_id == guild_id && v.challenger_id == challenger_id)
            .cloned()
            .collect())
    }
}

fn make_service() -> (ManageCoudeVendettaService, Arc<MockRepo>) {
    let repo = Arc::new(MockRepo::default());
    let svc = ManageCoudeVendettaService::new(repo.clone());
    (svc, repo)
}

#[tokio::test]
async fn declare_self_is_rejected() {
    let (svc, _) = make_service();
    let err = svc.declare("g", "u", "u").await.unwrap_err();
    matches!(err, DomainError::ValidationError(_));
}

#[tokio::test]
async fn declare_creates_vendetta() {
    let (svc, repo) = make_service();
    let id = svc.declare("g", "c", "t").await.unwrap();
    assert_eq!(*repo.declare_calls.lock().unwrap(), 1);
    let v = svc.get_active("g", "c", "t").await.unwrap().unwrap();
    assert_eq!(v.id, id);
    assert_eq!(v.status, VendettaStatus::Active);
}

#[tokio::test]
async fn cant_declare_when_already_active() {
    let (svc, _) = make_service();
    svc.declare("g", "c", "t").await.unwrap();
    let err = svc.declare("g", "c", "t").await.unwrap_err();
    matches!(err, DomainError::Conflict(_));
}

#[tokio::test]
async fn distinct_couples_can_coexist() {
    let (svc, _) = make_service();
    svc.declare("g", "c", "t1").await.unwrap();
    // c → t2 ne conflit pas avec c → t1.
    svc.declare("g", "c", "t2").await.unwrap();
    // t1 → c (sens inverse) non plus.
    svc.declare("g", "t1", "c").await.unwrap();
    let list = svc.list_by_challenger("g", "c").await.unwrap();
    assert_eq!(list.len(), 2);
}

#[tokio::test]
async fn resolve_won_marks_status() {
    let (svc, _) = make_service();
    let id = svc.declare("g", "c", "t").await.unwrap();
    svc.resolve(id, true).await.unwrap();
    // Plus active apres resolve.
    assert!(svc.get_active("g", "c", "t").await.unwrap().is_none());
    // Mais visible dans list.
    let list = svc.list_by_challenger("g", "c").await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].status, VendettaStatus::Won);
}

#[tokio::test]
async fn resolve_lost_marks_status() {
    let (svc, _) = make_service();
    let id = svc.declare("g", "c", "t").await.unwrap();
    svc.resolve(id, false).await.unwrap();
    let list = svc.list_by_challenger("g", "c").await.unwrap();
    assert_eq!(list[0].status, VendettaStatus::Lost);
}

#[tokio::test]
async fn resolve_twice_errors() {
    let (svc, _) = make_service();
    let id = svc.declare("g", "c", "t").await.unwrap();
    svc.resolve(id, true).await.unwrap();
    let err = svc.resolve(id, false).await.unwrap_err();
    matches!(err, DomainError::Conflict(_));
}

#[tokio::test]
async fn list_by_challenger_returns_history() {
    let (svc, _) = make_service();
    let id1 = svc.declare("g", "c", "t1").await.unwrap();
    let _id2 = svc.declare("g", "c", "t2").await.unwrap();
    svc.resolve(id1, true).await.unwrap();
    let list = svc.list_by_challenger("g", "c").await.unwrap();
    assert_eq!(list.len(), 2);
}
