use super::*;
use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use std::sync::Arc;
use std::sync::Mutex;
use uuid::Uuid;

use crate::domain::entities::coude::inventory::Insurance;
use crate::domain::entities::coude::inventory::InventoryItem;
use crate::domain::entities::coude::inventory::Prime;
use crate::domain::entities::coude::steal::boost::StealBoost;
use crate::domain::entities::coude::steal::protection::StealProtection;
use crate::domain::entities::coude::inventory::NewCoudePrime;
use crate::domain::entities::coude::steal::boost::StealBoostDuration;
use crate::domain::entities::coude::steal::protection::StealProtectionDuration;
use crate::domain::errors::DomainError;
use crate::ports::inbound::coude::manage_steal_protections::ManageCoudeStealProtectionsUseCase;
use crate::ports::inbound::coude::manage_steal_protections::StealProtectionTrigger;
use crate::ports::inbound::coude::manage_steal_boosts::ManageCoudeStealBoostsUseCase;
use crate::ports::inbound::coude::manage_inventory::ManageCoudeInventoryUseCase;

// ── Mocks ──

#[derive(Default)]
struct MockInventoryUc {
    inventory: Mutex<Vec<InventoryItem>>,
    add_calls: Mutex<Vec<(String, String, String)>>,
    use_calls: Mutex<Vec<(String, String, String)>>,
    use_return: Mutex<bool>,
    has_return: Mutex<bool>,
    create_prime_calls: Mutex<Vec<NewCoudePrime>>,
    active_primes: Mutex<Vec<Prime>>,
    claim_return: Mutex<i64>,
    buy_insurance_return: Mutex<bool>,
    active_insurance: Mutex<Option<Insurance>>,
    expire_calls: Mutex<Vec<Uuid>>,
}

#[async_trait]
impl ManageCoudeInventoryUseCase for MockInventoryUc {
    async fn list_inventory(&self, _: &str, _: &str) -> Result<Vec<InventoryItem>, DomainError> {
        Ok(self.inventory.lock().unwrap().clone())
    }
    async fn add_item(&self, g: &str, u: &str, k: &str) -> Result<(), DomainError> {
        self.add_calls.lock().unwrap().push((g.into(), u.into(), k.into()));
        Ok(())
    }
    async fn use_item(&self, g: &str, u: &str, k: &str) -> Result<bool, DomainError> {
        self.use_calls.lock().unwrap().push((g.into(), u.into(), k.into()));
        Ok(*self.use_return.lock().unwrap())
    }
    async fn has_item(&self, _: &str, _: &str, _: &str) -> Result<bool, DomainError> {
        Ok(*self.has_return.lock().unwrap())
    }
    async fn create_prime(&self, new: NewCoudePrime) -> Result<Prime, DomainError> {
        let p = Prime {
            id: Uuid::new_v4(),
            guild_id: new.guild_id.clone(),
            target_id: new.target_id.clone(),
            target_name: new.target_name.clone(),
            placed_by_id: new.placed_by_id.clone(),
            placed_by_name: new.placed_by_name.clone(),
            amount: new.amount,
            claimed: false,
            claimed_by_id: None,
            claimed_by_name: None,
            claimed_at: None,
            created_at: Utc::now(),
        };
        self.create_prime_calls.lock().unwrap().push(new);
        Ok(p)
    }
    async fn list_active_primes(&self, _: &str, _: &str) -> Result<Vec<Prime>, DomainError> {
        Ok(self.active_primes.lock().unwrap().clone())
    }
    async fn claim_primes(&self, _: &str, _: &str, _: &str, _: &str) -> Result<i64, DomainError> {
        Ok(*self.claim_return.lock().unwrap())
    }
    async fn buy_insurance(&self, _: &str, _: &str, _: bool, _: i64) -> Result<bool, DomainError> {
        Ok(*self.buy_insurance_return.lock().unwrap())
    }
    async fn get_active_insurance(&self, _: &str, _: &str) -> Result<Option<Insurance>, DomainError> {
        Ok(self.active_insurance.lock().unwrap().clone())
    }
    async fn expire_insurance(&self, id: Uuid) -> Result<(), DomainError> {
        self.expire_calls.lock().unwrap().push(id);
        Ok(())
    }
}

#[derive(Default)]
struct MockProtectionsUc {
    list_return: Mutex<Vec<StealProtection>>,
    price_return: Mutex<i64>,
    subscribe_return: Mutex<Option<DateTime<Utc>>>,
    trigger_return: Mutex<Option<StealProtectionTrigger>>,
    price_calls: Mutex<Vec<(String, StealProtectionDuration)>>,
    subscribe_calls: Mutex<Vec<(String, String, String, StealProtectionDuration)>>,
}

#[async_trait]
impl ManageCoudeStealProtectionsUseCase for MockProtectionsUc {
    async fn list_active(&self, _: &str, _: &str) -> Result<Vec<StealProtection>, DomainError> {
        Ok(self.list_return.lock().unwrap().clone())
    }
    async fn price_for(&self, item: &str, d: StealProtectionDuration) -> Result<i64, DomainError> {
        self.price_calls.lock().unwrap().push((item.into(), d));
        Ok(*self.price_return.lock().unwrap())
    }
    async fn subscribe(&self, g: &str, u: &str, item: &str, d: StealProtectionDuration) -> Result<DateTime<Utc>, DomainError> {
        self.subscribe_calls.lock().unwrap().push((g.into(), u.into(), item.into(), d));
        Ok(self.subscribe_return.lock().unwrap().unwrap_or(Utc::now()))
    }
    async fn try_trigger(&self, _: &str, _: &str) -> Result<Option<StealProtectionTrigger>, DomainError> {
        Ok(self.trigger_return.lock().unwrap().clone())
    }
}

#[derive(Default)]
struct MockBoostsUc {
    list_return: Mutex<Vec<StealBoost>>,
    price_return: Mutex<i64>,
    subscribe_return: Mutex<Option<DateTime<Utc>>>,
    total_return: Mutex<i32>,
}

#[async_trait]
impl ManageCoudeStealBoostsUseCase for MockBoostsUc {
    async fn list_active(&self, _: &str, _: &str) -> Result<Vec<StealBoost>, DomainError> {
        Ok(self.list_return.lock().unwrap().clone())
    }
    async fn price_for(&self, _: &str, _: StealBoostDuration) -> Result<i64, DomainError> {
        Ok(*self.price_return.lock().unwrap())
    }
    async fn subscribe(&self, _: &str, _: &str, _: &str, _: StealBoostDuration) -> Result<DateTime<Utc>, DomainError> {
        Ok(self.subscribe_return.lock().unwrap().unwrap_or(Utc::now()))
    }
    async fn total_bonus(&self, _: &str, _: &str) -> Result<i32, DomainError> {
        Ok(*self.total_return.lock().unwrap())
    }
}

fn grpc(
    uc: Arc<MockInventoryUc>,
    p: Arc<MockProtectionsUc>,
    b: Arc<MockBoostsUc>,
) -> InventoryGrpc {
    InventoryGrpc {
        uc,
        steal_protections_uc: p,
        steal_boosts_uc: b,
    }
}

fn default_grpc() -> InventoryGrpc {
    grpc(
        Arc::new(MockInventoryUc::default()),
        Arc::new(MockProtectionsUc::default()),
        Arc::new(MockBoostsUc::default()),
    )
}

// ── list_inventory ──

#[tokio::test]
async fn list_inventory_empty() {
    let g = default_grpc();
    let resp = g.list_inventory(Request::new(proto::UserInGuildRequest {
        guild_id: "g".into(), user_id: "u".into(),
    })).await.unwrap();
    assert!(resp.into_inner().items.is_empty());
}

#[tokio::test]
async fn list_inventory_returns_items() {
    let uc = Arc::new(MockInventoryUc::default());
    uc.inventory.lock().unwrap().push(InventoryItem {
        guild_id: "g".into(), user_id: "u".into(),
        item_key: "potion".into(), quantity: 5,
    });
    let g = grpc(uc, Arc::new(MockProtectionsUc::default()), Arc::new(MockBoostsUc::default()));
    let resp = g.list_inventory(Request::new(proto::UserInGuildRequest {
        guild_id: "g".into(), user_id: "u".into(),
    })).await.unwrap();
    let items = resp.into_inner().items;
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].item_key, "potion");
    assert_eq!(items[0].quantity, 5);
}

// ── add_item ──

#[tokio::test]
async fn add_item_delegates() {
    let uc = Arc::new(MockInventoryUc::default());
    let g = grpc(uc.clone(), Arc::new(MockProtectionsUc::default()), Arc::new(MockBoostsUc::default()));
    let _ = g.add_item(Request::new(proto::AddItemRequest {
        guild_id: "g".into(), user_id: "u".into(), item_key: "potion".into(),
    })).await.unwrap();
    let calls = uc.add_calls.lock().unwrap();
    assert_eq!(calls[0], ("g".into(), "u".into(), "potion".into()));
}

// ── use_item ──

#[tokio::test]
async fn use_item_returns_consumed_true() {
    let uc = Arc::new(MockInventoryUc::default());
    *uc.use_return.lock().unwrap() = true;
    let g = grpc(uc, Arc::new(MockProtectionsUc::default()), Arc::new(MockBoostsUc::default()));
    let resp = g.use_item(Request::new(proto::UseItemRequest {
        guild_id: "g".into(), user_id: "u".into(), item_key: "potion".into(),
    })).await.unwrap();
    assert!(resp.into_inner().consumed);
}

#[tokio::test]
async fn use_item_returns_consumed_false_if_empty() {
    let uc = Arc::new(MockInventoryUc::default());
    *uc.use_return.lock().unwrap() = false;
    let g = grpc(uc, Arc::new(MockProtectionsUc::default()), Arc::new(MockBoostsUc::default()));
    let resp = g.use_item(Request::new(proto::UseItemRequest {
        guild_id: "g".into(), user_id: "u".into(), item_key: "potion".into(),
    })).await.unwrap();
    assert!(!resp.into_inner().consumed);
}

// ── has_item ──

#[tokio::test]
async fn has_item_returns_bool_value() {
    let uc = Arc::new(MockInventoryUc::default());
    *uc.has_return.lock().unwrap() = true;
    let g = grpc(uc, Arc::new(MockProtectionsUc::default()), Arc::new(MockBoostsUc::default()));
    let resp = g.has_item(Request::new(proto::HasItemRequest {
        guild_id: "g".into(), user_id: "u".into(), item_key: "potion".into(),
    })).await.unwrap();
    assert!(resp.into_inner().value);
}

// ── create_prime ──

#[tokio::test]
async fn create_prime_delegates_and_returns_proto() {
    let uc = Arc::new(MockInventoryUc::default());
    let g = grpc(uc.clone(), Arc::new(MockProtectionsUc::default()), Arc::new(MockBoostsUc::default()));
    let resp = g.create_prime(Request::new(proto::CreatePrimeRequest {
        guild_id: "g".into(),
        target_id: "t".into(), target_name: "Target".into(),
        placed_by_id: "p".into(), placed_by_name: "Placer".into(),
        amount: 500,
    })).await.unwrap();
    let prime = resp.into_inner();
    assert_eq!(prime.amount, 500);
    assert_eq!(prime.target_name, "Target");
    assert!(!prime.claimed);
    let calls = uc.create_prime_calls.lock().unwrap();
    assert_eq!(calls[0].amount, 500);
}

// ── list_active_primes ──

#[tokio::test]
async fn list_active_primes_empty() {
    let g = default_grpc();
    let resp = g.list_active_primes(Request::new(proto::ListActivePrimesRequest {
        guild_id: "g".into(), target_id: "t".into(),
    })).await.unwrap();
    assert!(resp.into_inner().primes.is_empty());
}

// ── claim_primes ──

#[tokio::test]
async fn claim_primes_returns_total_amount() {
    let uc = Arc::new(MockInventoryUc::default());
    *uc.claim_return.lock().unwrap() = 1500;
    let g = grpc(uc, Arc::new(MockProtectionsUc::default()), Arc::new(MockBoostsUc::default()));
    let resp = g.claim_primes(Request::new(proto::ClaimPrimesRequest {
        guild_id: "g".into(), target_id: "t".into(),
        claimer_id: "c".into(), claimer_name: "Claimer".into(),
    })).await.unwrap();
    assert_eq!(resp.into_inner().value, 1500);
}

// ── buy_insurance ──

#[tokio::test]
async fn buy_insurance_returns_empty_on_success() {
    let uc = Arc::new(MockInventoryUc::default());
    *uc.buy_insurance_return.lock().unwrap() = true;
    let g = grpc(uc, Arc::new(MockProtectionsUc::default()), Arc::new(MockBoostsUc::default()));
    let _ = g.buy_insurance(Request::new(proto::BuyInsuranceRequest {
        guild_id: "g".into(), user_id: "u".into(),
        is_scam: false, duration_seconds: 86400, level: 0,
    })).await.unwrap();
}

#[tokio::test]
async fn buy_insurance_already_active_returns_already_exists() {
    let uc = Arc::new(MockInventoryUc::default());
    *uc.buy_insurance_return.lock().unwrap() = false;
    let g = grpc(uc, Arc::new(MockProtectionsUc::default()), Arc::new(MockBoostsUc::default()));
    let err = g.buy_insurance(Request::new(proto::BuyInsuranceRequest {
        guild_id: "g".into(), user_id: "u".into(),
        is_scam: false, duration_seconds: 86400, level: 0,
    })).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::AlreadyExists);
}

// ── get_active_insurance ──

#[tokio::test]
async fn get_active_insurance_none_when_absent() {
    let g = default_grpc();
    let resp = g.get_active_insurance(Request::new(proto::UserInGuildRequest {
        guild_id: "g".into(), user_id: "u".into(),
    })).await.unwrap();
    assert!(resp.into_inner().insurance.is_none());
}

#[tokio::test]
async fn get_active_insurance_some_when_present() {
    let uc = Arc::new(MockInventoryUc::default());
    *uc.active_insurance.lock().unwrap() = Some(Insurance {
        id: Uuid::new_v4(),
        is_scam: true,
        expires_at: Utc::now() + chrono::Duration::hours(1),
    });
    let g = grpc(uc, Arc::new(MockProtectionsUc::default()), Arc::new(MockBoostsUc::default()));
    let resp = g.get_active_insurance(Request::new(proto::UserInGuildRequest {
        guild_id: "g".into(), user_id: "u".into(),
    })).await.unwrap();
    let ins = resp.into_inner().insurance.unwrap();
    assert!(ins.is_scam);
}

// ── expire_insurance ──

#[tokio::test]
async fn expire_insurance_rejects_invalid_uuid() {
    let g = default_grpc();
    let err = g.expire_insurance(Request::new(proto::ExpireInsuranceRequest {
        insurance_id: "bad".into(),
    })).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn expire_insurance_valid_uuid_delegates() {
    let uc = Arc::new(MockInventoryUc::default());
    let id = Uuid::new_v4();
    let g = grpc(uc.clone(), Arc::new(MockProtectionsUc::default()), Arc::new(MockBoostsUc::default()));
    let _ = g.expire_insurance(Request::new(proto::ExpireInsuranceRequest {
        insurance_id: id.to_string(),
    })).await.unwrap();
    assert_eq!(uc.expire_calls.lock().unwrap()[0], id);
}

// ── steal protections ──

#[tokio::test]
async fn list_active_steal_protections_empty() {
    let g = default_grpc();
    let resp = g.list_active_steal_protections(Request::new(proto::UserInGuildRequest {
        guild_id: "g".into(), user_id: "u".into(),
    })).await.unwrap();
    assert!(resp.into_inner().protections.is_empty());
}

#[tokio::test]
async fn price_steal_protection_rejects_invalid_duration() {
    let g = default_grpc();
    let err = g.price_steal_protection(Request::new(proto::PriceStealProtectionRequest {
        item_key: "shield".into(),
        duration: proto::StealProtectionDurationKind::StealProtectionDurationUnspecified as i32,
    })).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn price_steal_protection_returns_value() {
    let p = Arc::new(MockProtectionsUc::default());
    *p.price_return.lock().unwrap() = 5000;
    let g = grpc(Arc::new(MockInventoryUc::default()), p.clone(), Arc::new(MockBoostsUc::default()));
    let resp = g.price_steal_protection(Request::new(proto::PriceStealProtectionRequest {
        item_key: "shield_3d".into(),
        duration: proto::StealProtectionDurationKind::StealProtectionDurationThreeDays as i32,
    })).await.unwrap();
    assert_eq!(resp.into_inner().value, 5000);
}

#[tokio::test]
async fn buy_steal_protection_invalid_duration_rejected() {
    let g = default_grpc();
    let err = g.buy_steal_protection(Request::new(proto::BuyStealProtectionRequest {
        guild_id: "g".into(), user_id: "u".into(),
        item_key: "shield".into(),
        duration: proto::StealProtectionDurationKind::StealProtectionDurationUnspecified as i32,
    })).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn buy_steal_protection_returns_expiry_and_cost() {
    let p = Arc::new(MockProtectionsUc::default());
    *p.price_return.lock().unwrap() = 1000;
    let future = Utc::now() + chrono::Duration::days(3);
    *p.subscribe_return.lock().unwrap() = Some(future);
    let g = grpc(Arc::new(MockInventoryUc::default()), p.clone(), Arc::new(MockBoostsUc::default()));
    let resp = g.buy_steal_protection(Request::new(proto::BuyStealProtectionRequest {
        guild_id: "g".into(), user_id: "u".into(),
        item_key: "shield_3d".into(),
        duration: proto::StealProtectionDurationKind::StealProtectionDurationThreeDays as i32,
    })).await.unwrap();
    let inner = resp.into_inner();
    assert_eq!(inner.cost, 1000);
    assert!(inner.expires_at.contains("T"));
    assert_eq!(p.subscribe_calls.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn try_trigger_steal_protection_none() {
    let g = default_grpc();
    let resp = g.try_trigger_steal_protection(Request::new(proto::UserInGuildRequest {
        guild_id: "g".into(), user_id: "u".into(),
    })).await.unwrap();
    assert!(resp.into_inner().trigger.is_none());
}

#[tokio::test]
async fn try_trigger_steal_protection_some() {
    let p = Arc::new(MockProtectionsUc::default());
    *p.trigger_return.lock().unwrap() = Some(StealProtectionTrigger {
        item_key: "shield_7d".into(),
        item_name: "Bouclier 7 jours".into(),
        rolled_value: 42,
        block_chance_percent: 80,
    });
    let g = grpc(Arc::new(MockInventoryUc::default()), p, Arc::new(MockBoostsUc::default()));
    let resp = g.try_trigger_steal_protection(Request::new(proto::UserInGuildRequest {
        guild_id: "g".into(), user_id: "u".into(),
    })).await.unwrap();
    let t = resp.into_inner().trigger.unwrap();
    assert_eq!(t.item_key, "shield_7d");
    assert_eq!(t.rolled_value, 42);
    assert_eq!(t.block_chance_percent, 80);
}

// ── steal boosts ──

#[tokio::test]
async fn list_active_steal_boosts_empty() {
    let g = default_grpc();
    let resp = g.list_active_steal_boosts(Request::new(proto::UserInGuildRequest {
        guild_id: "g".into(), user_id: "u".into(),
    })).await.unwrap();
    assert!(resp.into_inner().boosts.is_empty());
}

#[tokio::test]
async fn price_steal_boost_invalid_duration_rejected() {
    let g = default_grpc();
    let err = g.price_steal_boost(Request::new(proto::PriceStealBoostRequest {
        item_key: "boost".into(),
        duration: proto::StealProtectionDurationKind::StealProtectionDurationUnspecified as i32,
    })).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn price_steal_boost_returns_value() {
    let b = Arc::new(MockBoostsUc::default());
    *b.price_return.lock().unwrap() = 2500;
    let g = grpc(Arc::new(MockInventoryUc::default()), Arc::new(MockProtectionsUc::default()), b);
    let resp = g.price_steal_boost(Request::new(proto::PriceStealBoostRequest {
        item_key: "boost_5d".into(),
        duration: proto::StealProtectionDurationKind::StealProtectionDurationFiveDays as i32,
    })).await.unwrap();
    assert_eq!(resp.into_inner().value, 2500);
}

#[tokio::test]
async fn buy_steal_boost_invalid_duration_rejected() {
    let g = default_grpc();
    let err = g.buy_steal_boost(Request::new(proto::BuyStealBoostRequest {
        guild_id: "g".into(), user_id: "u".into(),
        item_key: "boost".into(),
        duration: proto::StealProtectionDurationKind::StealProtectionDurationUnspecified as i32,
    })).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn buy_steal_boost_returns_expiry_and_cost() {
    let b = Arc::new(MockBoostsUc::default());
    *b.price_return.lock().unwrap() = 3000;
    let g = grpc(Arc::new(MockInventoryUc::default()), Arc::new(MockProtectionsUc::default()), b);
    let resp = g.buy_steal_boost(Request::new(proto::BuyStealBoostRequest {
        guild_id: "g".into(), user_id: "u".into(),
        item_key: "boost_1d".into(),
        duration: proto::StealProtectionDurationKind::StealProtectionDurationOneDay as i32,
    })).await.unwrap();
    let inner = resp.into_inner();
    assert_eq!(inner.cost, 3000);
    assert!(inner.expires_at.contains("T"));
}

#[tokio::test]
async fn get_steal_boost_total_returns_bonus() {
    let b = Arc::new(MockBoostsUc::default());
    *b.total_return.lock().unwrap() = 25;
    let g = grpc(Arc::new(MockInventoryUc::default()), Arc::new(MockProtectionsUc::default()), b);
    let resp = g.get_steal_boost_total(Request::new(proto::UserInGuildRequest {
        guild_id: "g".into(), user_id: "u".into(),
    })).await.unwrap();
    assert_eq!(resp.into_inner().value, 25);
}
