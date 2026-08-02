use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use std::sync::Mutex;
use uuid::Uuid;

use crate::application::moderation::manage_infractions_service::ManageInfractionsService;
use crate::domain::entities::moderation::detection_flags::DetectionFlags;
use crate::domain::entities::moderation::infraction::Infraction;
use crate::domain::enums::moderation::action::Action;
use crate::domain::errors::DomainError;
use crate::ports::inbound::moderation::manage_infractions::InfractionFilters;
use crate::ports::inbound::moderation::manage_infractions::ManageInfractionsUseCase;
use crate::ports::outbound::moderation::infraction_repository::InfractionRepository;

fn sample(id: &str) -> Infraction {
    Infraction {
        id: Uuid::parse_str(id).unwrap_or_else(|_| Uuid::new_v4()),
        guild_id: "g".into(),
        channel_id: "c".into(),
        user_id: "u".into(),
        username: "u".into(),
        display_name: None,
        message_id: "m".into(),
        content: "".into(),
        flags: DetectionFlags {
            spam: false,
            insult: false,
            profanity: false,
            link: false,
            phishing: false,
        },
        score: 0.0,
        action: Action::Warn,
        reason: "".into(),
        duration: None,
        created_at: Utc::now(),
    }
}

#[derive(Default)]
struct MockRepo {
    list_calls: Mutex<Vec<(String, i64, i64)>>,
    all_calls: Mutex<Vec<(i64, i64)>>,
    deletes: Mutex<Vec<String>>,
    delete_older: Mutex<Vec<(String, i32)>>,
    find_by_id_returns: Mutex<Option<Infraction>>,
    delete_returns: Mutex<bool>,
    infractions: Mutex<Vec<Infraction>>,
}

#[async_trait]
impl InfractionRepository for MockRepo {
    async fn save(&self, _: &Infraction) -> Result<(), DomainError> {
        Ok(())
    }
    async fn find_by_guild(
        &self,
        g: &str,
        f: &InfractionFilters,
    ) -> Result<Vec<Infraction>, DomainError> {
        self.list_calls
            .lock()
            .unwrap()
            .push((g.into(), f.limit, f.offset));
        Ok(self.infractions.lock().unwrap().clone())
    }
    async fn find_all(&self, limit: i64, offset: i64) -> Result<Vec<Infraction>, DomainError> {
        self.all_calls.lock().unwrap().push((limit, offset));
        Ok(self.infractions.lock().unwrap().clone())
    }
    async fn count_today(&self) -> Result<u64, DomainError> {
        Ok(13)
    }
    async fn find_by_id(&self, _: &str) -> Result<Option<Infraction>, DomainError> {
        Ok(self.find_by_id_returns.lock().unwrap().clone())
    }
    async fn delete_by_id(&self, id: &str) -> Result<bool, DomainError> {
        self.deletes.lock().unwrap().push(id.into());
        Ok(*self.delete_returns.lock().unwrap())
    }
    async fn delete_older_than_days(&self, g: &str, d: i32) -> Result<u64, DomainError> {
        self.delete_older.lock().unwrap().push((g.into(), d));
        Ok(100)
    }
}

#[tokio::test]
async fn list_infractions_forwards_filters() {
    let r = Arc::new(MockRepo::default());
    let svc = ManageInfractionsService::new(r.clone());
    let f = InfractionFilters {
        user_id: None,
        action: None,
        limit: 25,
        offset: 5,
    };
    svc.list_infractions("g1", f).await.unwrap();
    assert_eq!(r.list_calls.lock().unwrap()[0], ("g1".into(), 25, 5));
}

#[tokio::test]
async fn list_all_infractions_forwards_limit_offset() {
    let r = Arc::new(MockRepo::default());
    let svc = ManageInfractionsService::new(r.clone());
    svc.list_all_infractions(100, 0).await.unwrap();
    assert_eq!(r.all_calls.lock().unwrap()[0], (100, 0));
}

#[tokio::test]
async fn count_today_forwards() {
    let svc = ManageInfractionsService::new(Arc::new(MockRepo::default()));
    assert_eq!(svc.count_today().await.unwrap(), 13);
}

#[tokio::test]
async fn find_by_id_returns_some_or_none() {
    let r = Arc::new(MockRepo::default());
    let svc = ManageInfractionsService::new(r.clone());
    assert!(svc.find_by_id("x").await.unwrap().is_none());
    *r.find_by_id_returns.lock().unwrap() = Some(sample("00000000-0000-0000-0000-000000000001"));
    assert!(svc.find_by_id("x").await.unwrap().is_some());
}

#[tokio::test]
async fn delete_infraction_forwards_bool_result() {
    let r = Arc::new(MockRepo::default());
    *r.delete_returns.lock().unwrap() = true;
    let svc = ManageInfractionsService::new(r.clone());
    assert!(svc.delete_infraction("abc").await.unwrap());
    assert_eq!(r.deletes.lock().unwrap()[0], "abc");
}

#[tokio::test]
async fn delete_older_than_days_forwards() {
    let r = Arc::new(MockRepo::default());
    let svc = ManageInfractionsService::new(r.clone());
    let n = svc.delete_older_than_days("g", 90).await.unwrap();
    assert_eq!(n, 100);
    assert_eq!(r.delete_older.lock().unwrap()[0], ("g".into(), 90));
}
