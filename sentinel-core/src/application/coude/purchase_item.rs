//! Use case `PurchaseItemUseCase` : achat boutique atomique server-side.
//!
//! Remplace l'orchestration NON-atomique qui vivait dans le bot
//! (`shop_cmd.rs` : lecture prix bot -> debit -> add_item -> rollback manuel
//! -> deposit cashbox). Ici, tout est server-side :
//! 1. le prix vient du catalogue SERVEUR (`shop::SHOP_ITEMS`), reglable par
//!    guild via la config `shop_<item>_price` ;
//! 2. la mutation (debit wallet + ajout item + cashbox) est deleguee au
//!    `PurchaseRepository` qui l'execute dans UNE transaction atomique.
//!
//! Plus aucun rollback cote client : le bug de perte de coins disparait.

use std::sync::Arc;

use async_trait::async_trait;

use crate::application::coude::guild_settings::GuildSettings;
use crate::domain::errors::DomainError;
use crate::domain::services::coude::coude_combat_engine::shop;
use crate::ports::inbound::coude::purchase_item::PurchaseItemUseCase;
use crate::ports::inbound::coude::purchase_item::PurchaseResult;
use crate::ports::outbound::coude::purchase_repository::PurchaseRepository;
use crate::ports::outbound::coude::purchase_repository::PurchaseTxOutcome;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;

pub struct PurchaseItemService {
    repo: Arc<dyn PurchaseRepository>,
    bot_config_repo: Arc<dyn BotConfigRepository>,
}

impl PurchaseItemService {
    pub fn new(
        repo: Arc<dyn PurchaseRepository>,
        bot_config_repo: Arc<dyn BotConfigRepository>,
    ) -> Self {
        Self {
            repo,
            bot_config_repo,
        }
    }

    /// Prix serveur : override guild `shop_<item>_price` s'il existe, sinon
    /// prix du catalogue domain. Mirror exact de la logique historique du bot
    /// (`guild_config.rs::shop_price`), mais desormais cote serveur.
    async fn resolve_price(&self, guild_id: &str, item_key: &str, default: i64) -> i64 {
        let config_key = format!("shop_{item_key}_price");
        GuildSettings::load(&*self.bot_config_repo, guild_id)
            .await
            .get_i64(&config_key, default)
    }
}

#[async_trait]
impl PurchaseItemUseCase for PurchaseItemService {
    async fn purchase_item(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<PurchaseResult, DomainError> {
        crate::application::validation::validate_non_empty(item_key, "item_key")?;

        // Le prix vient du catalogue SERVEUR (source de verite), pas du bot.
        let item = shop::get_item(item_key)
            .ok_or_else(|| DomainError::NotFound(format!("Objet inconnu : {item_key}")))?;
        let price = self.resolve_price(guild_id, item_key, item.price).await;

        match self
            .repo
            .purchase_item_atomic(guild_id, user_id, item_key, price)
            .await?
        {
            PurchaseTxOutcome::Purchased { new_balance } => {
                Ok(PurchaseResult::Success { price, new_balance })
            }
            PurchaseTxOutcome::InsufficientFunds { balance } => {
                Ok(PurchaseResult::InsufficientFunds { price, balance })
            }
        }
    }
}
