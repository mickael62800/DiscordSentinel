use super::*;
use crate::domain::entities::{CoudeInsurance, CoudeInventoryItem, CoudePrime, NewCoudePrime};
use crate::ports::inbound::manage_coude_inventory::ManageCoudeInventoryUseCase;
use chrono::Utc as ChronoUtc;
use std::sync::Mutex as StdMutex;
use uuid::Uuid;

#[derive(Default)]
struct MockRepo {
    primes: StdMutex<Vec<CoudePrime>>,
    items_added: StdMutex<Vec<(String, String, String)>>,
}

#[async_trait]
impl CoudeInventoryRepository for MockRepo {
    async fn list_inventory(&self, _: &str, _: &str) -> Result<Vec<CoudeInventoryItem>, DomainError> { Ok(vec![]) }
    async fn add_item(&self, g: &str, u: &str, k: &str) -> Result<(), DomainError> {
        self.items_added.lock().unwrap().push((g.into(), u.into(), k.into()));
        Ok(())
    }
    async fn use_item(&self, _: &str, _: &str, _: &str) -> Result<bool, DomainError> { Ok(true) }
    async fn has_item(&self, _: &str, _: &str, _: &str) -> Result<bool, DomainError> { Ok(false) }
    async fn create_prime(&self, new: NewCoudePrime) -> Result<CoudePrime, DomainError> {
        let prime = CoudePrime {
            id: Uuid::new_v4(),
            guild_id: new.guild_id,
            target_id: new.target_id,
            target_name: new.target_name,
            placed_by_id: new.placed_by_id,
            placed_by_name: new.placed_by_name,
            amount: new.amount,
            claimed: false,
            claimed_by_id: None,
            claimed_by_name: None,
            claimed_at: None,
            created_at: ChronoUtc::now(),
        };
        self.primes.lock().unwrap().push(prime.clone());
        Ok(prime)
    }
    async fn list_active_primes(&self, _: &str, _: &str) -> Result<Vec<CoudePrime>, DomainError> { Ok(vec![]) }
    async fn claim_primes(&self, _: &str, _: &str, _: &str, _: &str) -> Result<i64, DomainError> { Ok(0) }
    async fn buy_insurance(&self, _: &str, _: &str, _: bool, _: i64) -> Result<bool, DomainError> { Ok(true) }
    async fn get_active_insurance(&self, _: &str, _: &str) -> Result<Option<CoudeInsurance>, DomainError> { Ok(None) }
    async fn expire_insurance(&self, _: Uuid) -> Result<bool, DomainError> { Ok(true) }
}

fn new_prime(amount: i64, target: &str, placer: &str) -> NewCoudePrime {
    NewCoudePrime {
        guild_id: "g".into(),
        target_id: target.into(),
        target_name: "T".into(),
        placed_by_id: placer.into(),
        placed_by_name: "P".into(),
        amount,
    }
}

fn make_svc() -> ManageCoudeInventoryService {
    ManageCoudeInventoryService::new(Arc::new(MockRepo::default()))
}

#[tokio::test]
async fn create_prime_rejects_zero_amount() {
    let svc = make_svc();
    let err = svc.create_prime(new_prime(0, "t", "p")).await.unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(_)));
}

#[tokio::test]
async fn create_prime_rejects_negative_amount() {
    let svc = make_svc();
    let err = svc.create_prime(new_prime(-100, "t", "p")).await.unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(_)));
}

#[tokio::test]
async fn create_prime_rejects_self_prime() {
    let svc = make_svc();
    let err = svc.create_prime(new_prime(100, "same", "same")).await.unwrap_err();
    match err {
        DomainError::ValidationError(msg) => assert!(msg.contains("soi-meme")),
        other => panic!("Expected ValidationError, got {:?}", other),
    }
}

#[tokio::test]
async fn create_prime_accepts_valid_request() {
    let svc = make_svc();
    let prime = svc.create_prime(new_prime(500, "victim", "hunter")).await.unwrap();
    assert_eq!(prime.amount, 500);
    assert_eq!(prime.target_id, "victim");
    assert_eq!(prime.placed_by_id, "hunter");
    assert!(!prime.claimed);
}

#[tokio::test]
async fn add_item_rejects_empty_key() {
    let svc = make_svc();
    assert!(svc.add_item("g", "u", "").await.is_err());
    assert!(svc.add_item("g", "u", "   ").await.is_err());
}

#[tokio::test]
async fn add_item_accepts_valid_key() {
    let svc = make_svc();
    assert!(svc.add_item("g", "u", "masque_braquage").await.is_ok());
}

#[tokio::test]
async fn expire_insurance_not_found_returns_error() {
    // Le mock renvoie true par defaut, donc on construit un mock custom qui renvoie false.
    #[derive(Default)]
    struct FailExpireRepo;
    #[async_trait]
    impl CoudeInventoryRepository for FailExpireRepo {
        async fn list_inventory(&self, _: &str, _: &str) -> Result<Vec<CoudeInventoryItem>, DomainError> { Ok(vec![]) }
        async fn add_item(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
        async fn use_item(&self, _: &str, _: &str, _: &str) -> Result<bool, DomainError> { Ok(false) }
        async fn has_item(&self, _: &str, _: &str, _: &str) -> Result<bool, DomainError> { Ok(false) }
        async fn create_prime(&self, _: NewCoudePrime) -> Result<CoudePrime, DomainError> { unimplemented!() }
        async fn list_active_primes(&self, _: &str, _: &str) -> Result<Vec<CoudePrime>, DomainError> { Ok(vec![]) }
        async fn claim_primes(&self, _: &str, _: &str, _: &str, _: &str) -> Result<i64, DomainError> { Ok(0) }
        async fn buy_insurance(&self, _: &str, _: &str, _: bool, _: i64) -> Result<bool, DomainError> { Ok(true) }
        async fn get_active_insurance(&self, _: &str, _: &str) -> Result<Option<CoudeInsurance>, DomainError> { Ok(None) }
        async fn expire_insurance(&self, _: Uuid) -> Result<bool, DomainError> { Ok(false) }
    }
    let svc = ManageCoudeInventoryService::new(Arc::new(FailExpireRepo));
    let err = svc.expire_insurance(Uuid::new_v4()).await.unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)));
}
