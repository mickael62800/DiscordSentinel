//! Methodes `ApiClient` pour l'inventaire des items d'un joueur.
//!
//! Add / list / use / has. Utilise par le shop, les potions, les
//! items de combat (mindgame, rage, etc.) et les outils de braquage.
//! Les items "abonnements temps-base" (protections, boosts) ont leur
//! propre systeme en DB et ne passent pas par ces methodes.

use sentinel_proto::coude::v1 as proto_coude;

use super::{grpc_err_to_string, ApiClient, InventoryItem, PurchaseOutcome};

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
        crate::grpc_call!(@unit self.grpc, coude_inventory, add_item, req)
    }

    /// Achat boutique atomique server-side. L'API valide le prix (config
    /// serveur), debite le wallet, ajoute l'item et alimente la cashbox dans
    /// UNE transaction. Le bot n'a plus qu'a rendre le resultat.
    ///
    /// Retourne `PurchaseOutcome` : succes (avec prix paye + solde restant)
    /// ou solde insuffisant (avec prix requis + solde courant).
    pub async fn purchase_item(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<PurchaseOutcome, String> {
        let req = proto_coude::PurchaseItemRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            item_key: item_key.to_string(),
        };
        let r = crate::grpc_call!(self.grpc, coude_inventory, purchase_item, req)?;
        Ok(if r.success {
            PurchaseOutcome::Success {
                price: r.price,
                new_balance: r.balance,
            }
        } else {
            PurchaseOutcome::InsufficientFunds {
                price: r.price,
                balance: r.balance,
            }
        })
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
        let list = crate::grpc_call!(self.grpc, coude_inventory, list_inventory, req)?;
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
        let r = crate::grpc_call!(self.grpc, coude_inventory, use_item, req)?;
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
        let r = crate::grpc_call!(self.grpc, coude_inventory, has_item, req)?;
        Ok(r.value)
    }
}
