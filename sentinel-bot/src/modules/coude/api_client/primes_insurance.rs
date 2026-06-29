//! Methodes `ApiClient` des primes (bounty) et assurances.
//!
//! - Primes : pose une recompense sur la tete d'un joueur, le
//!   prochain qui le bat (combat ou vol) empoche.
//! - Assurance : abonnement temps-base qui reduit les pertes de
//!   combat de 50 % (ou les double si c'est une arnaque, configure
//!   par guild).

use sentinel_proto::coude::v1 as proto_coude;
use serde::Deserialize;

use super::{grpc_err_to_string, proto_prime_to_dto, ApiClient, Insurance, Prime};

#[derive(Debug, Deserialize, Clone)]
pub struct BuyInsuranceResolved {
    pub created: bool,
    pub is_scam: bool,
}

impl ApiClient {
    pub async fn create_prime(
        &self,
        guild_id: &str,
        target_id: &str,
        target_name: &str,
        placed_by_id: &str,
        placed_by_name: &str,
        amount: i64,
    ) -> Result<Prime, String> {
        let req = proto_coude::CreatePrimeRequest {
            guild_id: guild_id.to_string(),
            target_id: target_id.to_string(),
            target_name: target_name.to_string(),
            placed_by_id: placed_by_id.to_string(),
            placed_by_name: placed_by_name.to_string(),
            amount,
        };
        let mut client = self.grpc.coude_inventory();
        let p = self
            .grpc
            .guarded(|| async move { client.create_prime(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(proto_prime_to_dto(p))
    }

    pub async fn get_active_primes(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<Vec<Prime>, String> {
        let req = proto_coude::ListActivePrimesRequest {
            guild_id: guild_id.to_string(),
            target_id: target_id.to_string(),
        };
        let mut client = self.grpc.coude_inventory();
        let list = self
            .grpc
            .guarded(|| async move { client.list_active_primes(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(list.primes.into_iter().map(proto_prime_to_dto).collect())
    }

    pub async fn claim_primes(
        &self,
        guild_id: &str,
        target_id: &str,
        claimer_id: &str,
        claimer_name: &str,
    ) -> Result<i64, String> {
        let req = proto_coude::ClaimPrimesRequest {
            guild_id: guild_id.to_string(),
            target_id: target_id.to_string(),
            claimer_id: claimer_id.to_string(),
            claimer_name: claimer_name.to_string(),
        };
        let mut client = self.grpc.coude_inventory();
        let r = self
            .grpc
            .guarded(|| async move { client.claim_primes(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.value)
    }

    pub async fn buy_insurance(
        &self,
        guild_id: &str,
        user_id: &str,
        is_scam: bool,
        duration_seconds: i64,
        level: i32,
    ) -> Result<(), String> {
        let req = proto_coude::BuyInsuranceRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            is_scam,
            duration_seconds,
            level,
        };
        let mut client = self.grpc.coude_inventory();
        self.grpc
            .guarded(|| async move { client.buy_insurance(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    /// Phase 2 #3 audit : RNG scam decide cote API. Le bot envoie le taux
    /// + duree + level, l'API roule + persiste + retourne `is_scam`.
    pub async fn buy_insurance_with_roll(
        &self,
        guild_id: &str,
        user_id: &str,
        scam_rate_pct: u32,
        duration_seconds: i64,
        level: i32,
    ) -> Result<BuyInsuranceResolved, String> {
        #[derive(serde::Serialize)]
        struct Body<'a> {
            user_id: &'a str,
            scam_rate_pct: u32,
            duration_seconds: i64,
            level: i32,
        }
        let body = Body {
            user_id,
            scam_rate_pct,
            duration_seconds,
            level,
        };
        self.base
            .post_json(
                &format!("/api/coude/{guild_id}/insurance/buy-with-roll"),
                &body,
            )
            .await
    }

    pub async fn get_active_insurance(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<Insurance>, String> {
        let req = proto_coude::UserInGuildRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let mut client = self.grpc.coude_inventory();
        let r = self
            .grpc
            .guarded(|| async move {
                client
                    .get_active_insurance(req)
                    .await
                    .map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.insurance.map(|i| Insurance {
            id: i.id,
            is_scam: i.is_scam,
            expires_at: i.expires_at,
        }))
    }

    pub async fn expire_insurance(&self, id: &str) -> Result<(), String> {
        let req = proto_coude::ExpireInsuranceRequest {
            insurance_id: id.to_string(),
        };
        let mut client = self.grpc.coude_inventory();
        self.grpc
            .guarded(|| async move { client.expire_insurance(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }
}
