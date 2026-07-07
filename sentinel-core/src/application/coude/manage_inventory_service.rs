use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::application::coude::guild_settings::GuildSettings;
use crate::domain::entities::coude::inventory::Insurance;
use crate::domain::entities::coude::inventory::InventoryItem;
use crate::domain::entities::coude::inventory::NewCoudePrime;
use crate::domain::entities::coude::inventory::Prime;
use crate::domain::errors::DomainError;
use crate::ports::inbound::coude::manage_inventory::ManageCoudeInventoryUseCase;
use crate::ports::inbound::coude::manage_inventory::UsePotionResult;
use crate::ports::outbound::coude::inventory_repository::InventoryRepository;
use crate::ports::outbound::coude::inventory_repository::UsePotionTxOutcome;
use crate::ports::outbound::coude::player_repository::PlayerRepository;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;
pub struct ManageCoudeInventoryService {
    repo: Arc<dyn InventoryRepository>,
    bot_config_repo: Option<Arc<dyn BotConfigRepository>>,
    player_repo: Option<Arc<dyn PlayerRepository>>,
}

impl ManageCoudeInventoryService {
    pub fn new(repo: Arc<dyn InventoryRepository>) -> Self {
        Self {
            repo,
            bot_config_repo: None,
            player_repo: None,
        }
    }

    pub fn with_bot_config_repo(mut self, repo: Arc<dyn BotConfigRepository>) -> Self {
        self.bot_config_repo = Some(repo);
        self
    }

    /// Branche le repo player pour l'usage de potion hors combat (lecture des
    /// HP courants pour la regle anti-gaspillage).
    pub fn with_player_repo(mut self, repo: Arc<dyn PlayerRepository>) -> Self {
        self.player_repo = Some(repo);
        self
    }
}

#[async_trait]
impl ManageCoudeInventoryUseCase for ManageCoudeInventoryService {
    async fn list_inventory(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<InventoryItem>, DomainError> {
        self.repo.list_inventory(guild_id, user_id).await
    }

    async fn add_item(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<(), DomainError> {
        crate::application::validation::validate_non_empty(item_key, "item_key")?;
        self.repo.add_item(guild_id, user_id, item_key).await
    }

    async fn use_item(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<bool, DomainError> {
        self.repo.use_item(guild_id, user_id, item_key).await
    }

    async fn has_item(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<bool, DomainError> {
        self.repo.has_item(guild_id, user_id, item_key).await
    }

    async fn use_potion(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<UsePotionResult, DomainError> {
        use crate::domain::services::coude::potion::{evaluate, PotionEvaluation};

        let player_repo = self.player_repo.as_ref().ok_or_else(|| {
            DomainError::Internal("player_repo non branche pour use_potion".into())
        })?;

        let player = player_repo
            .get(guild_id, user_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("Joueur introuvable".into()))?;

        // Bareme + regle anti-gaspillage + clamp : domain pur.
        match evaluate(item_key, player.hp_current, player.hp_max) {
            PotionEvaluation::NotAPotion => Ok(UsePotionResult::NotAPotion),
            PotionEvaluation::AlreadyFull => Ok(UsePotionResult::AlreadyFull),
            PotionEvaluation::Wasteful {
                hp_missing,
                heal_amount,
            } => Ok(UsePotionResult::Wasteful {
                hp_missing,
                heal_amount,
            }),
            PotionEvaluation::Ok { heal_amount, .. } => {
                // Consommation item + heal (clamp re-applique) en UNE tx.
                match self
                    .repo
                    .use_potion_atomic(guild_id, user_id, item_key, heal_amount)
                    .await?
                {
                    UsePotionTxOutcome::Healed {
                        actually_healed,
                        new_hp,
                        hp_max,
                    } => Ok(UsePotionResult::Healed {
                        actually_healed,
                        new_hp,
                        hp_max,
                    }),
                    UsePotionTxOutcome::NoItem => Ok(UsePotionResult::NoItem),
                    UsePotionTxOutcome::AlreadyFull => Ok(UsePotionResult::AlreadyFull),
                }
            }
        }
    }

    async fn create_prime(&self, new: NewCoudePrime) -> Result<Prime, DomainError> {
        crate::application::validation::validate_positive(new.amount, "Le montant d'une prime")?;
        if new.target_id == new.placed_by_id {
            return Err(DomainError::ValidationError(
                "Impossible de placer une prime sur soi-meme".into(),
            ));
        }
        self.repo.create_prime(new).await
    }

    async fn list_active_primes(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<Vec<Prime>, DomainError> {
        self.repo.list_active_primes(guild_id, target_id).await
    }

    async fn claim_primes(
        &self,
        guild_id: &str,
        target_id: &str,
        claimer_id: &str,
        claimer_name: &str,
    ) -> Result<i64, DomainError> {
        self.repo
            .claim_primes(guild_id, target_id, claimer_id, claimer_name)
            .await
    }

    async fn buy_insurance(
        &self,
        guild_id: &str,
        user_id: &str,
        is_scam: bool,
        duration_seconds: i64,
    ) -> Result<bool, DomainError> {
        self.repo
            .buy_insurance(guild_id, user_id, is_scam, duration_seconds)
            .await
    }

    async fn buy_insurance_for_level(
        &self,
        guild_id: &str,
        user_id: &str,
        is_scam: bool,
        duration_seconds: i64,
        level: i32,
    ) -> Result<bool, DomainError> {
        // Cf. COUPE_AMELIORATIONS 3.2 palier niveau 5 (configurable via
        // `assurance_extra_slot_level`, default 5).
        let unlock_level = match &self.bot_config_repo {
            Some(repo) => GuildSettings::load(&**repo, guild_id)
                .await
                .get_i32("assurance_extra_slot_level", 5),
            None => 5,
        };
        let max_slots = if level >= unlock_level { 2 } else { 1 };
        self.repo
            .buy_insurance_with_max_slots(guild_id, user_id, is_scam, duration_seconds, max_slots)
            .await
    }

    async fn buy_insurance_with_scam_roll(
        &self,
        guild_id: &str,
        user_id: &str,
        scam_rate_pct: u32,
        duration_seconds: i64,
        level: i32,
    ) -> Result<(bool, bool), DomainError> {
        // Roll RNG cote serveur. Phase 2 #3 audit : le bot ne decide plus.
        let is_scam = {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            rng.gen_range(1..=100) <= scam_rate_pct.min(100)
        };
        let unlock_level = match &self.bot_config_repo {
            Some(repo) => GuildSettings::load(&**repo, guild_id)
                .await
                .get_i32("assurance_extra_slot_level", 5),
            None => 5,
        };
        let max_slots = if level >= unlock_level { 2 } else { 1 };
        let created = self
            .repo
            .buy_insurance_with_max_slots(guild_id, user_id, is_scam, duration_seconds, max_slots)
            .await?;
        Ok((created, is_scam))
    }

    async fn get_active_insurance(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<Insurance>, DomainError> {
        self.repo.get_active_insurance(guild_id, user_id).await
    }

    async fn expire_insurance(&self, insurance_id: Uuid) -> Result<(), DomainError> {
        let expired = self.repo.expire_insurance(insurance_id).await?;
        if !expired {
            return Err(DomainError::NotFound("Assurance introuvable".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/manage_inventory.rs"]
mod tests;
