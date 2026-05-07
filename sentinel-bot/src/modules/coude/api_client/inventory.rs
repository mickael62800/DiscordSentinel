//! Methodes `ApiClient` pour l'inventaire des items d'un joueur.
//!
//! Add / list / use / has. Utilise par le shop, les potions, les
//! items de combat (mindgame, rage, etc.) et les outils de braquage.
//! Les items "abonnements temps-base" (protections, boosts) ont leur
//! propre systeme en DB et ne passent pas par ces methodes.

use sentinel_proto::coude::v1 as proto_coude;

use super::{grpc_err_to_string, ApiClient, InventoryItem};

impl ApiClient {
    pub async fn add_item(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<(), String> {
        let req = proto_coude::AddItemRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            item_key: item_key.to_string(),
        };
        let mut client = self.grpc.coude_inventory();
        self.grpc
            .guarded(|| async move { client.add_item(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    pub async fn get_inventory(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<InventoryItem>, String> {
        let req = proto_coude::UserInGuildRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let mut client = self.grpc.coude_inventory();
        let list = self
            .grpc
            .guarded(|| async move {
                client.list_inventory(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(list
            .items
            .into_iter()
            .map(|i| InventoryItem {
                guild_id: i.guild_id,
                user_id: i.user_id,
                item_key: i.item_key,
                quantity: i.quantity,
            })
            .collect())
    }

    pub async fn use_item(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<bool, String> {
        let req = proto_coude::UseItemRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            item_key: item_key.to_string(),
        };
        let mut client = self.grpc.coude_inventory();
        let r = self
            .grpc
            .guarded(|| async move { client.use_item(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.consumed)
    }

    pub async fn has_item(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<bool, String> {
        let req = proto_coude::HasItemRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            item_key: item_key.to_string(),
        };
        let mut client = self.grpc.coude_inventory();
        let r = self
            .grpc
            .guarded(|| async move { client.has_item(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.value)
    }
}
