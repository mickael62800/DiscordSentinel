use super::*;
use sentinel_core::domain::entities::coude::inventory::Insurance;
use sentinel_core::domain::entities::coude::inventory::InventoryItem;
use sentinel_core::domain::entities::coude::inventory::Prime;
use sentinel_core::domain::entities::coude::inventory::NewCoudePrime;
use crate::ports::inbound::coude::manage_inventory::ManageCoudeInventoryUseCase;
use chrono::Utc as ChronoUtc;
use std::sync::Mutex as StdMutex;
use uuid::Uuid;

#[derive(Default)]
struct MockRepo {
    primes: StdMutex<Vec<Prime>>,
    items_added: StdMutex<Vec<(String, String, String)>>,
}

#[async_trait]
impl InventoryRepository for MockRepo {
    async fn list_inventory(&self, _: &str, _: &str) -> Result<Vec<InventoryItem>, DomainError> { Ok(vec![]) }
    async fn add_item(&self, g: &str, u: &str, k: &str) -> Result<(), DomainError> {
        self.items_added.lock().unwrap().push((g.into(), u.into(), k.into()));
        Ok(())
    }
    async fn use_item(&self, _: &str, _: &str, _: &str) -> Result<bool, DomainError> { Ok(true) }
    async fn has_item(&self, _: &str, _: &str, _: &str) -> Result<bool, DomainError> { Ok(false) }
    async fn create_prime(&self, new: NewCoudePrime) -> Result<Prime, DomainError> {
        let prime = Prime {
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
    async fn list_active_primes(&self, _: &str, _: &str) -> Result<Vec<Prime>, DomainError> { Ok(vec![]) }
    async fn claim_primes(&self, _: &str, _: &str, _: &str, _: &str) -> Result<i64, DomainError> { Ok(0) }
    async fn buy_insurance(&self, _: &str, _: &str, _: bool, _: i64) -> Result<bool, DomainError> { Ok(true) }
    async fn get_active_insurance(&self, _: &str, _: &str) -> Result<Option<Insurance>, DomainError> { Ok(None) }
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
    impl InventoryRepository for FailExpireRepo {
        async fn list_inventory(&self, _: &str, _: &str) -> Result<Vec<InventoryItem>, DomainError> { Ok(vec![]) }
        async fn add_item(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
        async fn use_item(&self, _: &str, _: &str, _: &str) -> Result<bool, DomainError> { Ok(false) }
        async fn has_item(&self, _: &str, _: &str, _: &str) -> Result<bool, DomainError> { Ok(false) }
        async fn create_prime(&self, _: NewCoudePrime) -> Result<Prime, DomainError> { unimplemented!() }
        async fn list_active_primes(&self, _: &str, _: &str) -> Result<Vec<Prime>, DomainError> { Ok(vec![]) }
        async fn claim_primes(&self, _: &str, _: &str, _: &str, _: &str) -> Result<i64, DomainError> { Ok(0) }
        async fn buy_insurance(&self, _: &str, _: &str, _: bool, _: i64) -> Result<bool, DomainError> { Ok(true) }
        async fn get_active_insurance(&self, _: &str, _: &str) -> Result<Option<Insurance>, DomainError> { Ok(None) }
        async fn expire_insurance(&self, _: Uuid) -> Result<bool, DomainError> { Ok(false) }
    }
    let svc = ManageCoudeInventoryService::new(Arc::new(FailExpireRepo));
    let err = svc.expire_insurance(Uuid::new_v4()).await.unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)));
}

// ── pass-through methods (delegation assertions) ──

#[derive(Default)]
struct RichMockRepo {
    inventory: StdMutex<Vec<InventoryItem>>,
    primes: StdMutex<Vec<Prime>>,
    insurance: StdMutex<Option<Insurance>>,
    use_item_return: StdMutex<bool>,
    has_item_return: StdMutex<bool>,
    claim_amount: StdMutex<i64>,
    buy_insurance_return: StdMutex<bool>,
    use_calls: StdMutex<Vec<(String, String, String)>>,
    claim_calls: StdMutex<Vec<(String, String, String, String)>>,
    buy_calls: StdMutex<Vec<(String, String, bool, i64)>>,
    expire_calls: StdMutex<Vec<Uuid>>,
}
#[async_trait]
impl InventoryRepository for RichMockRepo {
    async fn list_inventory(&self, _: &str, _: &str) -> Result<Vec<InventoryItem>, DomainError> {
        Ok(self.inventory.lock().unwrap().clone())
    }
    async fn add_item(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn use_item(&self, g: &str, u: &str, k: &str) -> Result<bool, DomainError> {
        self.use_calls.lock().unwrap().push((g.into(), u.into(), k.into()));
        Ok(*self.use_item_return.lock().unwrap())
    }
    async fn has_item(&self, _: &str, _: &str, _: &str) -> Result<bool, DomainError> {
        Ok(*self.has_item_return.lock().unwrap())
    }
    async fn create_prime(&self, _: NewCoudePrime) -> Result<Prime, DomainError> { unimplemented!() }
    async fn list_active_primes(&self, _: &str, _: &str) -> Result<Vec<Prime>, DomainError> {
        Ok(self.primes.lock().unwrap().clone())
    }
    async fn claim_primes(&self, g: &str, t: &str, c: &str, n: &str) -> Result<i64, DomainError> {
        self.claim_calls.lock().unwrap().push((g.into(), t.into(), c.into(), n.into()));
        Ok(*self.claim_amount.lock().unwrap())
    }
    async fn buy_insurance(&self, g: &str, u: &str, s: bool, d: i64) -> Result<bool, DomainError> {
        self.buy_calls.lock().unwrap().push((g.into(), u.into(), s, d));
        Ok(*self.buy_insurance_return.lock().unwrap())
    }
    async fn get_active_insurance(&self, _: &str, _: &str) -> Result<Option<Insurance>, DomainError> {
        Ok(self.insurance.lock().unwrap().clone())
    }
    async fn expire_insurance(&self, id: Uuid) -> Result<bool, DomainError> {
        self.expire_calls.lock().unwrap().push(id);
        Ok(true)
    }
}

#[tokio::test]
async fn list_inventory_returns_repo_value() {
    let repo = Arc::new(RichMockRepo::default());
    repo.inventory.lock().unwrap().push(InventoryItem {
        guild_id: "g".into(), user_id: "u".into(),
        item_key: "potion".into(), quantity: 3,
    });
    let svc = ManageCoudeInventoryService::new(repo);
    let items = svc.list_inventory("g", "u").await.unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].item_key, "potion");
}

#[tokio::test]
async fn use_item_delegates_and_returns_value() {
    let repo = Arc::new(RichMockRepo::default());
    *repo.use_item_return.lock().unwrap() = true;
    let svc = ManageCoudeInventoryService::new(repo.clone());
    assert!(svc.use_item("g", "u", "masque").await.unwrap());
    assert_eq!(repo.use_calls.lock().unwrap()[0], ("g".into(), "u".into(), "masque".into()));
}

#[tokio::test]
async fn has_item_returns_repo_bool() {
    let repo = Arc::new(RichMockRepo::default());
    *repo.has_item_return.lock().unwrap() = true;
    let svc = ManageCoudeInventoryService::new(repo);
    assert!(svc.has_item("g", "u", "k").await.unwrap());
}

#[tokio::test]
async fn list_active_primes_returns_repo_value() {
    let repo = Arc::new(RichMockRepo::default());
    repo.primes.lock().unwrap().push(Prime {
        id: Uuid::new_v4(),
        guild_id: "g".into(), target_id: "v".into(), target_name: "V".into(),
        placed_by_id: "p".into(), placed_by_name: "P".into(),
        amount: 1000, claimed: false, claimed_by_id: None, claimed_by_name: None,
        claimed_at: None, created_at: ChronoUtc::now(),
    });
    let svc = ManageCoudeInventoryService::new(repo);
    assert_eq!(svc.list_active_primes("g", "v").await.unwrap().len(), 1);
}

#[tokio::test]
async fn claim_primes_delegates_args_and_returns_total() {
    let repo = Arc::new(RichMockRepo::default());
    *repo.claim_amount.lock().unwrap() = 2500;
    let svc = ManageCoudeInventoryService::new(repo.clone());
    let amount = svc.claim_primes("g", "t", "claimer", "Claimer Name").await.unwrap();
    assert_eq!(amount, 2500);
    assert_eq!(repo.claim_calls.lock().unwrap()[0],
        ("g".into(), "t".into(), "claimer".into(), "Claimer Name".into()));
}

#[tokio::test]
async fn buy_insurance_delegates_args() {
    let repo = Arc::new(RichMockRepo::default());
    *repo.buy_insurance_return.lock().unwrap() = true;
    let svc = ManageCoudeInventoryService::new(repo.clone());
    assert!(svc.buy_insurance("g", "u", true, 3600).await.unwrap());
    assert_eq!(repo.buy_calls.lock().unwrap()[0],
        ("g".into(), "u".into(), true, 3600));
}

#[tokio::test]
async fn get_active_insurance_returns_none_by_default() {
    let svc = ManageCoudeInventoryService::new(Arc::new(RichMockRepo::default()));
    assert!(svc.get_active_insurance("g", "u").await.unwrap().is_none());
}

#[tokio::test]
async fn expire_insurance_ok_when_repo_returns_true() {
    let repo = Arc::new(RichMockRepo::default());
    let svc = ManageCoudeInventoryService::new(repo.clone());
    let id = Uuid::new_v4();
    svc.expire_insurance(id).await.unwrap();
    assert_eq!(repo.expire_calls.lock().unwrap()[0], id);
}

#[tokio::test]
async fn add_item_delegates_args_when_valid() {
    let repo = Arc::new(MockRepo::default());
    let svc = ManageCoudeInventoryService::new(repo.clone());
    svc.add_item("g", "u", "lockpick").await.unwrap();
    assert_eq!(repo.items_added.lock().unwrap()[0],
        ("g".into(), "u".into(), "lockpick".into()));
}
