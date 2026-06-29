//! Methodes `ApiClient` des abonnements anti-vol (`/protection`).
//!
//! Abonnements temps-base 1/3/5/7 jours, invisibles aux voleurs.
//! Le catalogue et le calcul du prix vivent cote API ; le bot ne fait
//! que transmettre les appels gRPC et desserialiser les reponses.

use sentinel_proto::coude::v1 as proto_coude;

use super::{
    grpc_err_to_string, ApiClient, StealProtection, StealProtectionDuration, StealProtectionTrigger,
};

impl ApiClient {
    pub async fn list_active_steal_protections(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<StealProtection>, String> {
        let req = proto_coude::UserInGuildRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let mut client = self.grpc.coude_inventory();
        let r = self
            .grpc
            .guarded(|| async move {
                client
                    .list_active_steal_protections(req)
                    .await
                    .map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.protections
            .into_iter()
            .map(|p| StealProtection {
                item_key: p.item_key,
                expires_at: p.expires_at,
            })
            .collect())
    }

    pub async fn price_steal_protection(
        &self,
        item_key: &str,
        duration: StealProtectionDuration,
    ) -> Result<i64, String> {
        let req = proto_coude::PriceStealProtectionRequest {
            item_key: item_key.to_string(),
            duration: duration.as_proto() as i32,
        };
        let mut client = self.grpc.coude_inventory();
        let r = self
            .grpc
            .guarded(|| async move {
                client
                    .price_steal_protection(req)
                    .await
                    .map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.value)
    }

    /// Achete un abonnement de protection. Retourne (cost, expires_at).
    pub async fn buy_steal_protection(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
        duration: StealProtectionDuration,
    ) -> Result<(i64, String), String> {
        let req = proto_coude::BuyStealProtectionRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            item_key: item_key.to_string(),
            duration: duration.as_proto() as i32,
        };
        let mut client = self.grpc.coude_inventory();
        let r = self
            .grpc
            .guarded(|| async move {
                client
                    .buy_steal_protection(req)
                    .await
                    .map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok((r.cost, r.expires_at))
    }

    /// Interroge le serveur pour savoir si une protection a bloque un
    /// vol. L'API roll elle-meme les dés — le bot n'a aucun secret a
    /// garder (au contraire, le voleur ne voit meme pas le nom de
    /// l'item bloquant avant qu'il s'active).
    pub async fn try_trigger_steal_protection(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<Option<StealProtectionTrigger>, String> {
        let req = proto_coude::UserInGuildRequest {
            guild_id: guild_id.to_string(),
            user_id: target_id.to_string(),
        };
        let mut client = self.grpc.coude_inventory();
        let r = self
            .grpc
            .guarded(|| async move {
                client
                    .try_trigger_steal_protection(req)
                    .await
                    .map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.trigger.map(|t| StealProtectionTrigger {
            item_key: t.item_key,
            item_name: t.item_name,
            rolled_value: t.rolled_value,
            block_chance_percent: t.block_chance_percent,
        }))
    }
}
