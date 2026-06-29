//! Methodes `ApiClient` des boosts voleur (`/boost-voleur`).
//!
//! Abonnements temps-base qui ajoutent un bonus flat au roll d20 du
//! voleur. 5 items cumulables (Crochet/Passe-partout/Deguisement/
//! Fumigene/Marteau). Meme grille de duree que les protections.

use sentinel_proto::coude::v1 as proto_coude;

use super::{grpc_err_to_string, ApiClient, StealProtectionDuration};

impl ApiClient {
    pub async fn price_steal_boost(
        &self,
        item_key: &str,
        duration: StealProtectionDuration,
    ) -> Result<i64, String> {
        let req = proto_coude::PriceStealBoostRequest {
            item_key: item_key.to_string(),
            duration: duration.as_proto() as i32,
        };
        let mut client = self.grpc.coude_inventory();
        let r = self
            .grpc
            .guarded(|| async move { client.price_steal_boost(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.value)
    }

    pub async fn buy_steal_boost(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
        duration: StealProtectionDuration,
    ) -> Result<(i64, String), String> {
        let req = proto_coude::BuyStealBoostRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            item_key: item_key.to_string(),
            duration: duration.as_proto() as i32,
        };
        let mut client = self.grpc.coude_inventory();
        let r = self
            .grpc
            .guarded(|| async move { client.buy_steal_boost(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok((r.cost, r.expires_at))
    }

    /// Retourne la somme des roll bonuses des items de boost actifs du
    /// voleur. 0 si aucun item actif. Appele avant un /voler pour
    /// ajouter au thief_total.
    pub async fn get_steal_boost_total(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i32, String> {
        let req = proto_coude::UserInGuildRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let mut client = self.grpc.coude_inventory();
        let r = self
            .grpc
            .guarded(|| async move {
                client
                    .get_steal_boost_total(req)
                    .await
                    .map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.value as i32)
    }
}
