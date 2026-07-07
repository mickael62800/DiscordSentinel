//! Handler gRPC du service `CoudeInventoryService`.
//!
//! Inclut :
//! - CRUD inventaire items (list, add, use, has)
//! - Primes et assurances
//! - Phase 9 Part B/C : abonnements anti-vol + boosts voleur
//!   (list_active, price, buy, try_trigger, total)
//!
//! Les helpers de mapping (inventory_item_to_proto, prime_to_proto,
//! insurance_to_proto, steal_boost_to_proto, steal_protection_to_proto,
//! proto_steal_duration_to_domain) sont locaux a ce fichier.

use std::sync::Arc;

use sentinel_proto::coude::v1 as proto;
use sentinel_proto::coude::v1::coude_inventory_service_server::CoudeInventoryService;
use tonic::Request;
use tonic::Response;
use tonic::Status;

use crate::adapters::inbound::grpc::errors::domain_to_status;
use crate::ports::inbound::coude::manage_inventory::ManageCoudeInventoryUseCase;
use sentinel_core::domain::entities::coude::inventory::Insurance;
use sentinel_core::domain::entities::coude::inventory::InventoryItem;
use sentinel_core::domain::entities::coude::inventory::NewCoudePrime;
use sentinel_core::domain::entities::coude::inventory::Prime;

use super::parse_uuid;

pub struct InventoryGrpc {
    pub uc: Arc<dyn ManageCoudeInventoryUseCase>,
    pub purchase_uc: Arc<dyn crate::ports::inbound::coude::purchase_item::PurchaseItemUseCase>,
    pub steal_protections_uc: Arc<dyn crate::ports::inbound::coude::manage_steal_protections::ManageCoudeStealProtectionsUseCase>,
    pub steal_boosts_uc: Arc<dyn crate::ports::inbound::coude::manage_steal_boosts::ManageCoudeStealBoostsUseCase>,
}

pub(super) fn inventory_item_to_proto(i: InventoryItem) -> proto::InventoryItem {
    proto::InventoryItem {
        guild_id: i.guild_id.into(),
        user_id: i.user_id.into(),
        item_key: i.item_key,
        quantity: i.quantity,
    }
}

pub(super) fn prime_to_proto(p: Prime) -> proto::Prime {
    proto::Prime {
        id: p.id.to_string(),
        guild_id: p.guild_id.into(),
        target_id: p.target_id,
        target_name: p.target_name,
        placed_by_id: p.placed_by_id,
        placed_by_name: p.placed_by_name,
        amount: p.amount,
        claimed: p.claimed,
        claimed_by_id: p.claimed_by_id,
        claimed_by_name: p.claimed_by_name,
        claimed_at: p.claimed_at.map(|d| d.to_rfc3339()),
        created_at: p.created_at.to_rfc3339(),
    }
}

pub(super) fn insurance_to_proto(i: Insurance) -> proto::Insurance {
    proto::Insurance {
        id: i.id.to_string(),
        is_scam: i.is_scam,
        expires_at: i.expires_at.to_rfc3339(),
    }
}

#[tonic::async_trait]
impl CoudeInventoryService for InventoryGrpc {
    async fn list_inventory(
        &self,
        request: Request<proto::UserInGuildRequest>,
    ) -> Result<Response<proto::InventoryList>, Status> {
        let req = request.into_inner();
        let items = self
            .uc
            .list_inventory(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::InventoryList {
            items: items.into_iter().map(inventory_item_to_proto).collect(),
        }))
    }

    async fn add_item(
        &self,
        request: Request<proto::AddItemRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.uc
            .add_item(&req.guild_id, &req.user_id, &req.item_key)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn use_item(
        &self,
        request: Request<proto::UseItemRequest>,
    ) -> Result<Response<proto::UseItemResponse>, Status> {
        let req = request.into_inner();
        let consumed = self
            .uc
            .use_item(&req.guild_id, &req.user_id, &req.item_key)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::UseItemResponse { consumed }))
    }

    async fn has_item(
        &self,
        request: Request<proto::HasItemRequest>,
    ) -> Result<Response<proto::BoolValue>, Status> {
        let req = request.into_inner();
        let v = self
            .uc
            .has_item(&req.guild_id, &req.user_id, &req.item_key)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::BoolValue { value: v }))
    }

    async fn purchase_item(
        &self,
        request: Request<proto::PurchaseItemRequest>,
    ) -> Result<Response<proto::PurchaseItemResponse>, Status> {
        use crate::ports::inbound::coude::purchase_item::PurchaseResult;
        let req = request.into_inner();
        let result = self
            .purchase_uc
            .purchase_item(&req.guild_id, &req.user_id, &req.item_key)
            .await
            .map_err(domain_to_status)?;
        let resp = match result {
            PurchaseResult::Success { price, new_balance } => proto::PurchaseItemResponse {
                success: true,
                price,
                balance: new_balance,
            },
            PurchaseResult::InsufficientFunds { price, balance } => proto::PurchaseItemResponse {
                success: false,
                price,
                balance,
            },
        };
        Ok(Response::new(resp))
    }

    async fn create_prime(
        &self,
        request: Request<proto::CreatePrimeRequest>,
    ) -> Result<Response<proto::Prime>, Status> {
        let req = request.into_inner();
        let prime = self
            .uc
            .create_prime(NewCoudePrime {
                guild_id: req.guild_id.into(),
                target_id: req.target_id,
                target_name: req.target_name,
                placed_by_id: req.placed_by_id,
                placed_by_name: req.placed_by_name,
                amount: req.amount,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(prime_to_proto(prime)))
    }

    async fn list_active_primes(
        &self,
        request: Request<proto::ListActivePrimesRequest>,
    ) -> Result<Response<proto::PrimeList>, Status> {
        let req = request.into_inner();
        let primes = self
            .uc
            .list_active_primes(&req.guild_id, &req.target_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::PrimeList {
            primes: primes.into_iter().map(prime_to_proto).collect(),
        }))
    }

    async fn claim_primes(
        &self,
        request: Request<proto::ClaimPrimesRequest>,
    ) -> Result<Response<proto::Int64Value>, Status> {
        let req = request.into_inner();
        let total = self
            .uc
            .claim_primes(
                &req.guild_id,
                &req.target_id,
                &req.claimer_id,
                &req.claimer_name,
            )
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Int64Value { value: total }))
    }

    async fn buy_insurance(
        &self,
        request: Request<proto::BuyInsuranceRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        let inserted = self
            .uc
            .buy_insurance_for_level(
                &req.guild_id,
                &req.user_id,
                req.is_scam,
                req.duration_seconds,
                req.level,
            )
            .await
            .map_err(domain_to_status)?;
        if !inserted {
            return Err(Status::already_exists(
                "Une assurance active existe deja pour ce joueur",
            ));
        }
        Ok(Response::new(proto::Empty {}))
    }

    async fn get_active_insurance(
        &self,
        request: Request<proto::UserInGuildRequest>,
    ) -> Result<Response<proto::MaybeInsurance>, Status> {
        let req = request.into_inner();
        let ins = self
            .uc
            .get_active_insurance(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::MaybeInsurance {
            insurance: ins.map(insurance_to_proto),
        }))
    }

    async fn expire_insurance(
        &self,
        request: Request<proto::ExpireInsuranceRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let insurance_id = parse_uuid(&request.into_inner().insurance_id)?;
        self.uc
            .expire_insurance(insurance_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    // ── Phase 9 Part B : abonnements anti-vol ──

    async fn list_active_steal_protections(
        &self,
        request: Request<proto::UserInGuildRequest>,
    ) -> Result<Response<proto::StealProtectionList>, Status> {
        let req = request.into_inner();
        let list = self
            .steal_protections_uc
            .list_active(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::StealProtectionList {
            protections: list.into_iter().map(steal_protection_to_proto).collect(),
        }))
    }

    async fn price_steal_protection(
        &self,
        request: Request<proto::PriceStealProtectionRequest>,
    ) -> Result<Response<proto::Int64Value>, Status> {
        let req = request.into_inner();
        let duration = proto_steal_duration_to_domain(req.duration)
            .ok_or_else(|| Status::invalid_argument("duree invalide"))?;
        let price = self
            .steal_protections_uc
            .price_for(&req.item_key, duration)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Int64Value { value: price }))
    }

    async fn buy_steal_protection(
        &self,
        request: Request<proto::BuyStealProtectionRequest>,
    ) -> Result<Response<proto::BuyStealProtectionResponse>, Status> {
        let req = request.into_inner();
        let duration = proto_steal_duration_to_domain(req.duration)
            .ok_or_else(|| Status::invalid_argument("duree invalide"))?;
        let cost = self
            .steal_protections_uc
            .price_for(&req.item_key, duration)
            .await
            .map_err(domain_to_status)?;
        let expires_at = self
            .steal_protections_uc
            .subscribe(&req.guild_id, &req.user_id, &req.item_key, duration)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::BuyStealProtectionResponse {
            expires_at: expires_at.to_rfc3339(),
            cost,
        }))
    }

    async fn try_trigger_steal_protection(
        &self,
        request: Request<proto::UserInGuildRequest>,
    ) -> Result<Response<proto::MaybeStealProtectionTrigger>, Status> {
        let req = request.into_inner();
        let trigger = self
            .steal_protections_uc
            .try_trigger(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::MaybeStealProtectionTrigger {
            trigger: trigger.map(|t| proto::StealProtectionTrigger {
                item_key: t.item_key,
                item_name: t.item_name,
                rolled_value: t.rolled_value,
                block_chance_percent: t.block_chance_percent,
            }),
        }))
    }

    // ── Phase 9 Part C : boost voleur ──

    async fn list_active_steal_boosts(
        &self,
        request: Request<proto::UserInGuildRequest>,
    ) -> Result<Response<proto::StealBoostList>, Status> {
        let req = request.into_inner();
        let list = self
            .steal_boosts_uc
            .list_active(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::StealBoostList {
            boosts: list.into_iter().map(steal_boost_to_proto).collect(),
        }))
    }

    async fn price_steal_boost(
        &self,
        request: Request<proto::PriceStealBoostRequest>,
    ) -> Result<Response<proto::Int64Value>, Status> {
        let req = request.into_inner();
        let duration = proto_steal_duration_to_domain(req.duration)
            .ok_or_else(|| Status::invalid_argument("duree invalide"))?;
        let price = self
            .steal_boosts_uc
            .price_for(&req.item_key, duration)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Int64Value { value: price }))
    }

    async fn buy_steal_boost(
        &self,
        request: Request<proto::BuyStealBoostRequest>,
    ) -> Result<Response<proto::BuyStealBoostResponse>, Status> {
        let req = request.into_inner();
        let duration = proto_steal_duration_to_domain(req.duration)
            .ok_or_else(|| Status::invalid_argument("duree invalide"))?;
        let cost = self
            .steal_boosts_uc
            .price_for(&req.item_key, duration)
            .await
            .map_err(domain_to_status)?;
        let expires_at = self
            .steal_boosts_uc
            .subscribe(&req.guild_id, &req.user_id, &req.item_key, duration)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::BuyStealBoostResponse {
            expires_at: expires_at.to_rfc3339(),
            cost,
        }))
    }

    async fn get_steal_boost_total(
        &self,
        request: Request<proto::UserInGuildRequest>,
    ) -> Result<Response<proto::Int64Value>, Status> {
        let req = request.into_inner();
        let total = self
            .steal_boosts_uc
            .total_bonus(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Int64Value {
            value: total as i64,
        }))
    }
}

#[cfg(test)]
#[path = "tests/inventory.rs"]
mod tests;

pub(super) fn steal_boost_to_proto(
    b: sentinel_core::domain::entities::coude::steal::boost::StealBoost,
) -> proto::StealBoost {
    proto::StealBoost {
        id: b.id.to_string(),
        guild_id: b.guild_id.into(),
        user_id: b.user_id.into(),
        item_key: b.item_key,
        expires_at: b.expires_at.to_rfc3339(),
        created_at: b.created_at.to_rfc3339(),
    }
}

pub(super) fn steal_protection_to_proto(
    p: sentinel_core::domain::entities::coude::steal::protection::StealProtection,
) -> proto::StealProtection {
    proto::StealProtection {
        id: p.id.to_string(),
        guild_id: p.guild_id.into(),
        user_id: p.user_id.into(),
        item_key: p.item_key,
        expires_at: p.expires_at.to_rfc3339(),
        created_at: p.created_at.to_rfc3339(),
    }
}

pub(super) fn proto_steal_duration_to_domain(
    v: i32,
) -> Option<sentinel_core::domain::entities::coude::steal::protection::StealProtectionDuration> {
    use proto::StealProtectionDurationKind as P;
    use sentinel_core::domain::entities::coude::steal::protection::StealProtectionDuration as D;
    match P::try_from(v).ok()? {
        P::StealProtectionDurationUnspecified => None,
        P::StealProtectionDurationOneDay => Some(D::OneDay),
        P::StealProtectionDurationThreeDays => Some(D::ThreeDays),
        P::StealProtectionDurationFiveDays => Some(D::FiveDays),
        P::StealProtectionDurationSevenDays => Some(D::SevenDays),
    }
}
