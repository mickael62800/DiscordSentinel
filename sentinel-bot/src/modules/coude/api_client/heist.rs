//! Methodes `ApiClient` du systeme de braquage (Phase 10).
//! Extrait du god-object `api_client.rs` (refactor 2026-04).

use sentinel_proto::coude::v1 as proto_coude;

use super::{grpc_err_to_string, ApiClient, HeistCooldown, HeistResult, PrisonStatus};

impl ApiClient {
    pub async fn attempt_heist(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<HeistResult, String> {
        let req = proto_coude::UserInGuildRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let mut client = self.grpc.coude_social();
        let r = self
            .grpc
            .guarded(|| async move { client.attempt_heist(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(HeistResult {
            success: r.success,
            chance_percent: r.chance_percent,
            cashbox_total_before: r.cashbox_total_before,
            amount_stolen: r.amount_stolen,
            tools_consumed: r.tools_consumed,
            prison_released_at: r.prison_released_at,
        })
    }

    pub async fn get_heist_cooldown(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<HeistCooldown, String> {
        let req = proto_coude::UserInGuildRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let mut client = self.grpc.coude_social();
        let r = self
            .grpc
            .guarded(|| async move { client.get_heist_cooldown(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(HeistCooldown {
            ready: r.ready,
            next_attempt_at: r.next_attempt_at,
            last_success: r.last_success,
        })
    }

    pub async fn get_prison_status(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<PrisonStatus, String> {
        let req = proto_coude::UserInGuildRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let mut client = self.grpc.coude_social();
        let r = self
            .grpc
            .guarded(|| async move { client.get_prison_status(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(PrisonStatus {
            in_prison: r.in_prison,
            released_at: r.released_at,
            reason: r.reason,
        })
    }
}
