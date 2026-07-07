//! Methodes `ApiClient` pour l'inventaire des items d'un joueur.
//!
//! Add / list / use / has. Utilise par le shop, les potions, les
//! items de combat (mindgame, rage, etc.) et les outils de braquage.
//! Les items "abonnements temps-base" (protections, boosts) ont leur
//! propre systeme en DB et ne passent pas par ces methodes.

use sentinel_proto::coude::v1 as proto_coude;

use super::{grpc_err_to_string, ApiClient, InventoryItem, PurchaseOutcome};

/// Resultat de l'usage d'une potion (bareme + heal resolus server-side).
#[derive(Debug, Clone)]
pub enum UsePotionOutcome {
    Healed {
        actually_healed: i32,
        new_hp: i32,
        hp_max: i32,
    },
    NotAPotion,
    AlreadyFull,
    Wasteful {
        hp_missing: i32,
        heal_amount: i32,
    },
    NoItem,
}

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

    /// Utilise une potion hors combat : le bareme (heal), la regle
    /// anti-gaspillage, le clamp au HP max et la consommation de l'item sont
    /// resolus server-side de facon atomique. Le bot ne fait que rendre.
    pub async fn use_potion(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<UsePotionOutcome, String> {
        use proto_coude::UsePotionOutcomeKind as K;
        let req = proto_coude::UseItemRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            item_key: item_key.to_string(),
        };
        let r = crate::grpc_call!(self.grpc, coude_inventory, use_potion, req)?;
        Ok(match K::try_from(r.kind).unwrap_or(K::NotAPotion) {
            K::Healed => UsePotionOutcome::Healed {
                actually_healed: r.actually_healed,
                new_hp: r.new_hp,
                hp_max: r.hp_max,
            },
            K::NotAPotion => UsePotionOutcome::NotAPotion,
            K::AlreadyFull => UsePotionOutcome::AlreadyFull,
            K::Wasteful => UsePotionOutcome::Wasteful {
                hp_missing: r.hp_missing,
                heal_amount: r.heal_amount,
            },
            K::NoItem => UsePotionOutcome::NoItem,
        })
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
