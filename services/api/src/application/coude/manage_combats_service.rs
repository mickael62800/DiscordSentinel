use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::coude::combat_validation::check_min_hp_pct;
use crate::domain::entities::coude::combat_validation::check_surprise_hp_pct;
use crate::domain::entities::coude::combat_validation::validate_new_combat;
use crate::domain::entities::coude::combat::CombatResolution;
use crate::domain::entities::coude::balance::BalanceParams;
use crate::domain::entities::coude::combat::CoudeCombat;
use crate::domain::entities::coude::combat::NewCoudeCombat;
use crate::domain::errors::DomainError;
use crate::ports::inbound::coude::manage_combats::ManageCoudeCombatsUseCase;
use crate::ports::inbound::coude::manage_players::ManageCoudePlayersUseCase;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;
use crate::ports::outbound::coude::combat_repository::CombatRepository;
pub struct ManageCoudeCombatsService {
    repo: Arc<dyn CombatRepository>,
    /// Optionnel : requis pour appliquer le gate `surprise_min_hp_percent`.
    players_uc: Option<Arc<dyn ManageCoudePlayersUseCase>>,
    /// Optionnel : requis pour lire `BalanceParams` depuis la config guild.
    bot_config_repo: Option<Arc<dyn BotConfigRepository>>,
}

impl ManageCoudeCombatsService {
    pub fn new(repo: Arc<dyn CombatRepository>) -> Self {
        Self {
            repo,
            players_uc: None,
            bot_config_repo: None,
        }
    }

    /// Branche les dependances pour le gate `surprise_min_hp_percent`
    /// (Phase 132+). Sans ces deps, le gate est inactif (comportement
    /// historique).
    pub fn with_surprise_gate(
        mut self,
        players_uc: Arc<dyn ManageCoudePlayersUseCase>,
        bot_config_repo: Arc<dyn BotConfigRepository>,
    ) -> Self {
        self.players_uc = Some(players_uc);
        self.bot_config_repo = Some(bot_config_repo);
        self
    }

    async fn load_balance(&self, guild_id: &str) -> BalanceParams {
        let Some(repo) = self.bot_config_repo.as_ref() else {
            return BalanceParams::default();
        };
        crate::application::coude::guild_settings::load_balance_params(&**repo, guild_id).await
    }
}

#[async_trait]
impl ManageCoudeCombatsUseCase for ManageCoudeCombatsService {
    async fn list(
        &self,
        guild_id: &str,
        status: Option<&str>,
        limit: i64,
    ) -> Result<Vec<CoudeCombat>, DomainError> {
        let limit = limit.clamp(1, 200);
        // "all" est traité côté repo via Option::None.
        let status_filter = status.filter(|s| *s != "all");
        self.repo.list(guild_id, status_filter, limit).await
    }

    async fn get(&self, id: Uuid) -> Result<CoudeCombat, DomainError> {
        self.repo
            .get(id)
            .await?
            .ok_or_else(|| DomainError::NotFound("Combat introuvable".into()))
    }

    async fn get_pending_for_attacker(
        &self,
        guild_id: &str,
        attacker_id: &str,
    ) -> Result<Option<CoudeCombat>, DomainError> {
        self.repo.get_pending_for_attacker(guild_id, attacker_id).await
    }

    async fn get_pending_for_defender(
        &self,
        guild_id: &str,
        defender_id: &str,
    ) -> Result<Option<CoudeCombat>, DomainError> {
        self.repo.get_pending_for_defender(guild_id, defender_id).await
    }

    async fn list_expired_pending(&self) -> Result<Vec<CoudeCombat>, DomainError> {
        self.repo.list_expired_pending().await
    }

    async fn get_betting_for_participant(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<CoudeCombat>, DomainError> {
        self.repo
            .get_betting_for_participant(guild_id, user_id)
            .await
    }

    async fn create(&self, new: NewCoudeCombat) -> Result<CoudeCombat, DomainError> {
        // 1. Validations pures (mise positive, pas de self-duel) — domain.
        validate_new_combat(&new)?;

        // 2. Gates HP (necessitent une lecture players + config). Inactifs si
        //    les deps optionnelles ne sont pas branchees.
        if let (Some(players_uc), Some(_)) =
            (self.players_uc.as_ref(), self.bot_config_repo.as_ref())
        {
            let params = self.load_balance(&new.guild_id).await;

            // Gate 1 : les DEUX combattants doivent avoir >= combat_min_hp_pct%.
            if params.combat_min_hp_pct > 0 {
                let attacker = players_uc.get(&new.guild_id, &new.attacker_id).await?;
                let defender = players_uc.get(&new.guild_id, &new.defender_id).await?;
                check_min_hp_pct(
                    "L'attaquant",
                    attacker.hp_current,
                    attacker.hp_max,
                    params.combat_min_hp_pct,
                )?;
                check_min_hp_pct(
                    "Le defenseur",
                    defender.hp_current,
                    defender.hp_max,
                    params.combat_min_hp_pct,
                )?;
            }

            // Gate 2 : attaque surprise -> seuil HP attaquant specifique.
            if new.special_attack.as_deref() == Some("surprise")
                && params.surprise_min_hp_pct > 0
            {
                let attacker = players_uc.get(&new.guild_id, &new.attacker_id).await?;
                check_surprise_hp_pct(
                    attacker.hp_current,
                    attacker.hp_max,
                    params.surprise_min_hp_pct,
                )?;
            }
        }

        self.repo.create(new).await
    }

    async fn cancel(&self, id: Uuid) -> Result<(), DomainError> {
        let cancelled = self.repo.cancel_pending(id).await?;
        if !cancelled {
            return Err(DomainError::NotFound(
                "Combat introuvable ou deja resolu".into(),
            ));
        }

        // Effet de bord : marquer les paris non résolus comme perdus.
        // Si ça échoue, on log mais on ne fait pas échouer la commande
        // (le combat est déjà annulé, mieux vaut un état partiellement
        // cohérent qu'une 500 qui laisse l'utilisateur dans le doute).
        if let Err(e) = self.repo.mark_unresolved_bets_lost(id).await {
            tracing::warn!(
                error = %e,
                combat_id = %id,
                "Echec remboursement paris apres annulation combat"
            );
        }

        Ok(())
    }

    async fn resolve(
        &self,
        id: Uuid,
        resolution: CombatResolution,
    ) -> Result<(), DomainError> {
        let resolved = self.repo.resolve(id, resolution).await?;
        if !resolved {
            return Err(DomainError::Conflict(
                "Combat deja resolu ou introuvable".into(),
            ));
        }
        Ok(())
    }

    async fn set_betting(&self, id: Uuid, message_id: &str) -> Result<bool, DomainError> {
        if message_id.is_empty() {
            return Err(DomainError::ValidationError(
                "message_id requis".into(),
            ));
        }
        self.repo.set_betting(id, message_id).await
    }

    async fn expire(&self, id: Uuid) -> Result<(), DomainError> {
        let expired = self.repo.expire(id).await?;
        if !expired {
            return Err(DomainError::NotFound("Combat introuvable".into()));
        }
        Ok(())
    }

    async fn set_defender_special(
        &self,
        id: Uuid,
        item_key: &str,
    ) -> Result<(), DomainError> {
        if item_key.is_empty() {
            return Err(DomainError::ValidationError("item_key requis".into()));
        }
        let updated = self.repo.set_defender_special(id, item_key).await?;
        if !updated {
            return Err(DomainError::NotFound("Combat introuvable".into()));
        }
        Ok(())
    }

    async fn get_guild_id(&self, id: Uuid) -> Result<Option<String>, DomainError> {
        Ok(self.repo.get(id).await?.map(|c| c.guild_id))
    }

    async fn purge_guild_subsystem(
        &self,
        guild_id: &str,
    ) -> Result<Vec<(String, u64)>, DomainError> {
        self.repo.purge_guild_subsystem(guild_id).await
    }
}

#[cfg(test)]
#[path = "tests/manage_combats.rs"]
mod tests;
