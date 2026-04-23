use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::{CombatResolution, CoudeBalanceParams, CoudeCombat, NewCoudeCombat};
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_coude_combats::ManageCoudeCombatsUseCase;
use crate::ports::inbound::ManageCoudePlayersUseCase;
use crate::ports::outbound::{BotConfigRepository, CoudeCombatRepository};

pub struct ManageCoudeCombatsService {
    repo: Arc<dyn CoudeCombatRepository>,
    /// Optionnel : requis pour appliquer le gate `surprise_min_hp_percent`.
    players_uc: Option<Arc<dyn ManageCoudePlayersUseCase>>,
    /// Optionnel : requis pour lire `CoudeBalanceParams` depuis la config guild.
    bot_config_repo: Option<Arc<dyn BotConfigRepository>>,
}

impl ManageCoudeCombatsService {
    pub fn new(repo: Arc<dyn CoudeCombatRepository>) -> Self {
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

    async fn load_balance(&self, guild_id: &str) -> CoudeBalanceParams {
        let Some(repo) = self.bot_config_repo.as_ref() else {
            return CoudeBalanceParams::default();
        };
        match repo.get_config(guild_id, "coude-bot").await {
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
        if new.mise <= 0 {
            return Err(DomainError::ValidationError(
                "La mise doit etre strictement positive".into(),
            ));
        }
        if new.attacker_id == new.defender_id {
            return Err(DomainError::ValidationError(
                "Un joueur ne peut pas se defier lui-meme".into(),
            ));
        }

        // Gate 1 : verifier que les DEUX combattants ont au moins
        // `combat_min_hp_pct` % de HP. Empeche un joueur a 0 HP d etre
        // defie ou de defier (combat ferait 0 round et bug la resolution).
        if let (Some(players_uc), Some(_)) =
            (self.players_uc.as_ref(), self.bot_config_repo.as_ref())
        {
            let params = self.load_balance(&new.guild_id).await;
            let min_pct = params.combat_min_hp_pct;
            if min_pct > 0 {
                let attacker = players_uc.get(&new.guild_id, &new.attacker_id).await?;
                let defender = players_uc.get(&new.guild_id, &new.defender_id).await?;

                let check = |who: &str, hp_cur: i32, hp_max: i32| -> Result<(), DomainError> {
                    let hp_max_u = (hp_max.max(1)) as u64;
                    let hp_cur_u = (hp_cur.max(0)) as u64;
                    let cur_pct = hp_cur_u.saturating_mul(100) / hp_max_u;
                    if cur_pct < min_pct {
                        return Err(DomainError::ValidationError(format!(
                            "{who} n'a pas assez de PV pour combattre : {hp_cur_u}/{hp_max_u} ({cur_pct}%), minimum requis {min_pct}%. Utilise /repos pour te soigner."
                        )));
                    }
                    Ok(())
                };
                check("L'attaquant", attacker.hp_current, attacker.hp_max)?;
                check("Le defenseur", defender.hp_current, defender.hp_max)?;
            }

            // Gate 2 : pour l'attaque surprise, verification supplementaire
            // que l'attaquant a au moins `surprise_min_hp_percent` % de ses HP.
            if new.special_attack.as_deref() == Some("surprise") {
                let min_pct = params.surprise_min_hp_pct;
                if min_pct > 0 {
                    let attacker = players_uc.get(&new.guild_id, &new.attacker_id).await?;
                    let hp_max = attacker.hp_max.max(1) as u64;
                    let hp_cur = attacker.hp_current.max(0) as u64;
                    let cur_pct = hp_cur.saturating_mul(100) / hp_max;
                    if cur_pct < min_pct {
                        return Err(DomainError::ValidationError(format!(
                            "HP insuffisants pour une attaque surprise : {hp_cur}/{hp_max} ({cur_pct}%), minimum requis {min_pct}%."
                        )));
                    }
                }
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
}

#[cfg(test)]
#[path = "tests/manage_coude_combats.rs"]
mod tests;
