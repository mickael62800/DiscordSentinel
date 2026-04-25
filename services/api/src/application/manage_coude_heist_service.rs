//! Impl du systeme de braquage (Phase 10).
//!
//! Orchestre :
//! - cooldown hebdo (coude_heist_repo.last_attempt)
//! - prison (coude_heist_repo.get_prison / send_to_prison)
//! - inventaire d'outils (coude_inventory_uc.list_inventory)
//! - caisse communautaire (cashbox_repo.claim_all_for_redistribution
//!   adapte, ou lecture + deposit negative — on choisit un claim
//!   partiel custom ici)
//! - wallet du joueur (wallet_repo.credit/debit)
//! - consommation des outils (inventory_uc.use_item)

#[cfg(test)]
#[path = "tests/manage_coude_heist.rs"]
mod tests;

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration as ChronoDuration, Utc};
use rand::Rng;
use tracing::info;

use crate::domain::entities::{
    compute_success_chance, CoudeBalanceParams, HeistOutcome,
    HEIST_GAIN_MAX_PERCENT, HEIST_GAIN_MIN_PERCENT, HEIST_TOOLS,
};
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_coude_heist::{
    HeistCooldownStatus, ManageCoudeHeistUseCase, PrisonStatusInfo,
};
use crate::ports::inbound::ManageCoudeInventoryUseCase;
use crate::ports::outbound::{
    BotConfigRepository, CoudeCashboxRepository, CoudeHeistRepository, WalletRepository,
};

pub struct ManageCoudeHeistService {
    heist_repo: Arc<dyn CoudeHeistRepository>,
    cashbox_repo: Arc<dyn CoudeCashboxRepository>,
    inventory_uc: Arc<dyn ManageCoudeInventoryUseCase>,
    wallet_repo: Arc<dyn WalletRepository>,
    bot_config_repo: Arc<dyn BotConfigRepository>,
}

impl ManageCoudeHeistService {
    pub fn new(
        heist_repo: Arc<dyn CoudeHeistRepository>,
        cashbox_repo: Arc<dyn CoudeCashboxRepository>,
        inventory_uc: Arc<dyn ManageCoudeInventoryUseCase>,
        wallet_repo: Arc<dyn WalletRepository>,
        bot_config_repo: Arc<dyn BotConfigRepository>,
    ) -> Self {
        Self {
            heist_repo,
            cashbox_repo,
            inventory_uc,
            wallet_repo,
            bot_config_repo,
        }
    }

    async fn load_balance(&self, guild_id: &str) -> CoudeBalanceParams {
        match self.bot_config_repo.get_config(guild_id, "coude-bot").await {
            Ok(entries) => {
                let map: std::collections::HashMap<String, String> = entries
                    .into_iter()
                    .map(|e| (e.config_key, e.config_value))
                    .collect();
                CoudeBalanceParams::from_config(&map)
            }
            Err(_) => CoudeBalanceParams::default(),
        }
    }
}

#[async_trait]
impl ManageCoudeHeistUseCase for ManageCoudeHeistService {
    async fn get_cooldown_status(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<HeistCooldownStatus, DomainError> {
        let last = self.heist_repo.last_attempt(guild_id, user_id).await?;
        let Some(last) = last else {
            return Ok(HeistCooldownStatus {
                ready: true,
                next_attempt_at: None,
                last_success: None,
            });
        };
        let params = self.load_balance(guild_id).await;
        let cooldown_days = params.heist_cooldown_days.max(1) as i64;
        let next = last.attempted_at + ChronoDuration::days(cooldown_days);
        let ready = Utc::now() >= next;
        Ok(HeistCooldownStatus {
            ready,
            next_attempt_at: Some(next),
            last_success: Some(last.success),
        })
    }

    async fn get_prison_status(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<PrisonStatusInfo, DomainError> {
        let state = self.heist_repo.get_prison(guild_id, user_id).await?;
        let Some(state) = state else {
            return Ok(PrisonStatusInfo {
                in_prison: false,
                released_at: None,
                reason: None,
            });
        };
        let in_prison = state.released_at > Utc::now();
        Ok(PrisonStatusInfo {
            in_prison,
            released_at: Some(state.released_at),
            reason: Some(state.reason),
        })
    }

    async fn attempt_heist(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<HeistOutcome, DomainError> {
        // 1. Check prison
        let prison = self.get_prison_status(guild_id, user_id).await?;
        if prison.in_prison {
            return Err(DomainError::Forbidden(
                "Tu es en prison ! Impossible de braquer avant liberation.".into(),
            ));
        }

        // 2. Check cooldown configurable (default 7 jours, cf. CoudeBalanceParams)
        let params_early = self.load_balance(guild_id).await;
        let cooldown = self.get_cooldown_status(guild_id, user_id).await?;
        if !cooldown.ready {
            return Err(DomainError::Forbidden(format!(
                "Cooldown {} jours non ecoule.",
                params_early.heist_cooldown_days
            )));
        }

        // 3. Charger la caisse : on veut juste lire le solde pour savoir
        //    s'il vaut le coup de tenter (sinon abort). On ne vide pas
        //    tout : meme sur succes, on ne prend qu'un % aleatoire.
        let cashbox = self.cashbox_repo.get_or_create(guild_id).await?;
        if cashbox.balance <= 0 {
            return Err(DomainError::Forbidden(
                "La caisse est vide, inutile de tenter.".into(),
            ));
        }

        // 4. Lister les outils de braquage actifs dans l'inventaire.
        //    Chaque tool key present (quantity > 0) compte une fois.
        let inventory = self
            .inventory_uc
            .list_inventory(guild_id, user_id)
            .await?;
        let tool_keys: Vec<String> = inventory
            .iter()
            .filter(|i| i.quantity > 0)
            .map(|i| i.item_key.clone())
            .filter(|k| HEIST_TOOLS.iter().any(|t| t.key == k.as_str()))
            .collect();

        // 5. Calcule la chance effective (domain pur).
        let chance = compute_success_chance(&tool_keys);

        // 6. Roll + decide. On scope ThreadRng pour rester Send.
        let (success, gain_percent) = {
            let mut rng = rand::thread_rng();
            let roll: u32 = rng.gen_range(1..=100);
            let success = roll <= chance;
            let gain: u32 = rng.gen_range(HEIST_GAIN_MIN_PERCENT..=HEIST_GAIN_MAX_PERCENT);
            (success, gain)
        };

        // 7. Consomme une fraction aleatoire des outils utilises (Phase 132) :
        //    * succes → `braquage_tools_consumed_success_pct` (%)
        //    * echec  → `braquage_tools_consumed_fail_pct`    (%)
        //    Selection aleatoire parmi les outils actifs. Si l'erreur
        //    survient en cours de consommation, on arrete et on remonte
        //    l'erreur (meme semantique qu'avant).
        let balance = self.load_balance(guild_id).await;
        let consumed_pct = if success {
            balance.braquage_tools_consumed_success_pct
        } else {
            balance.braquage_tools_consumed_fail_pct
        };
        let tools_to_consume: Vec<String> = {
            use rand::seq::SliceRandom;
            let total = tool_keys.len();
            let count = ((total as u64 * consumed_pct) / 100) as usize;
            let count = count.min(total);
            let mut rng = rand::thread_rng();
            let mut shuffled: Vec<String> = tool_keys.clone();
            shuffled.shuffle(&mut rng);
            shuffled.into_iter().take(count).collect()
        };
        for key in &tools_to_consume {
            self.inventory_uc.use_item(guild_id, user_id, key).await?;
        }

        // 8. Calcule le montant vole (capture instantane, la caisse peut
        //    bouger entre get_or_create et le prelevement — acceptable :
        //    on prend gain % du solde courant).
        let amount_stolen: i64 = if success {
            let refreshed = self.cashbox_repo.get_or_create(guild_id).await?;
            let balance = refreshed.balance.max(0);
            // Arithmetique i128 pour eviter perte de precision f64 sur
            // gros soldes (> 2^53) et tout risque d'overflow i64 sur
            // balance * gain_percent.
            ((balance as i128) * (gain_percent as i128) / 100) as i64
        } else {
            0
        };

        if success && amount_stolen > 0 {
            // Prelever de la caisse : on utilise un deposit negatif ? Non,
            // la signature refuse amount <= 0. On passe par une methode
            // dedie ? Pour rester minimal, on fait directement une UPDATE
            // via le repo : mais on n'a pas de `withdraw` methode. On
            // recourre donc a un hack temporaire : on fait un claim_all
            // puis on re-deposit la difference. C'est atomique cote DB
            // mais moche. Alternative propre : ajouter une methode
            // withdraw au CoudeCashboxRepository. Faisons-le.
            self.cashbox_repo
                .withdraw(guild_id, amount_stolen)
                .await?;

            // Credit le wallet du joueur
            self.wallet_repo
                .credit(
                    guild_id,
                    user_id,
                    amount_stolen,
                    "coude_heist_success",
                    "Braquage de la caisse reussi",
                )
                .await?;
        }

        // 9. Log la tentative (pour le cooldown)
        let _ = self
            .heist_repo
            .record_attempt(
                guild_id,
                user_id,
                success,
                amount_stolen,
                chance as i32,
                &tool_keys,
            )
            .await?;

        // 10. Si echec → prison (duree configurable, default 24h).
        let prison_released_at = if !success {
            let prison_hours = params_early.heist_prison_hours.max(1) as i64;
            let released = Utc::now() + ChronoDuration::hours(prison_hours);
            self.heist_repo
                .send_to_prison(guild_id, user_id, released, "heist_failed")
                .await?;
            Some(released)
        } else {
            None
        };

        info!(
            guild_id,
            user_id,
            success,
            chance,
            amount_stolen,
            tools = tool_keys.len(),
            "Heist attempt resolved"
        );

        Ok(HeistOutcome {
            success,
            chance_percent: chance,
            cashbox_total_before: cashbox.balance,
            amount_stolen,
            tools_consumed: tools_to_consume,
            prison_released_at,
        })
    }
}
