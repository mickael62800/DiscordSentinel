use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::entities::coude::player::CombatStat;
use crate::domain::entities::coude::player::CoudePlayer;
use crate::domain::entities::coude::player::XpProgress;
use crate::domain::errors::DomainError;
use crate::ports::inbound::coude::manage_players::ManageCoudePlayersUseCase;
use crate::ports::outbound::coude::player_repository::CoudePlayerRepository;

pub struct ManageCoudePlayersService {
    repo: Arc<dyn CoudePlayerRepository>,
}

impl ManageCoudePlayersService {
    pub fn new(repo: Arc<dyn CoudePlayerRepository>) -> Self {
        Self { repo }
    }

    async fn require_player(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<CoudePlayer, DomainError> {
        self.repo
            .get(guild_id, user_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("Joueur introuvable".into()))
    }
}

#[async_trait]
impl ManageCoudePlayersUseCase for ManageCoudePlayersService {
    async fn get_or_create(
        &self,
        guild_id: String,
        user_id: String,
        username: String,
    ) -> Result<CoudePlayer, DomainError> {
        self.repo.get_or_create(&guild_id, &user_id, &username).await
    }

    async fn get(&self, guild_id: &str, user_id: &str) -> Result<CoudePlayer, DomainError> {
        self.require_player(guild_id, user_id).await
    }

    async fn list(&self, guild_id: &str) -> Result<Vec<CoudePlayer>, DomainError> {
        // 200 = la limite historique du handler legacy.
        self.repo.list(guild_id, 200).await
    }

    async fn random_active(
        &self,
        guild_id: &str,
        count: i64,
    ) -> Result<Vec<CoudePlayer>, DomainError> {
        let count = count.clamp(1, 50);
        // 50 coins minimum = comportement historique (filtre les comptes "vides").
        self.repo.random_active(guild_id, count, 50).await
    }

    async fn list_guild_ids(&self) -> Result<Vec<String>, DomainError> {
        self.repo.list_guild_ids().await
    }

    async fn update_class(
        &self,
        guild_id: &str,
        user_id: &str,
        class: &str,
    ) -> Result<(), DomainError> {
        if class.trim().is_empty() {
            return Err(DomainError::ValidationError("Classe invalide".into()));
        }
        let updated = self.repo.update_class(guild_id, user_id, class).await?;
        if !updated {
            return Err(DomainError::NotFound("Joueur introuvable".into()));
        }
        Ok(())
    }

    async fn add_xp(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
    ) -> Result<XpProgress, DomainError> {
        self.repo
            .add_xp(guild_id, user_id, amount)
            .await?
            .ok_or_else(|| DomainError::NotFound("Joueur introuvable".into()))
    }

    async fn spend_stat_point(
        &self,
        guild_id: &str,
        user_id: &str,
        stat: CombatStat,
    ) -> Result<CoudePlayer, DomainError> {
        self.repo
            .spend_stat_point(guild_id, user_id, stat)
            .await?
            .ok_or_else(|| {
                DomainError::ValidationError(
                    "Joueur introuvable ou pas de stat_points disponibles".into(),
                )
            })
    }

    async fn reset_stats(
        &self,
        guild_id: &str,
        user_id: &str,
        cost: i64,
    ) -> Result<CoudePlayer, DomainError> {
        if cost < 0 {
            return Err(DomainError::ValidationError(
                "Le cout ne peut pas etre negatif".into(),
            ));
        }
        self.repo
            .reset_stats(guild_id, user_id, cost)
            .await?
            .ok_or_else(|| {
                DomainError::ValidationError(
                    "Reset impossible : joueur introuvable, coins insuffisants ou aucun point a reset"
                        .into(),
                )
            })
    }

    async fn record_win(
        &self,
        guild_id: &str,
        user_id: &str,
        earned: i64,
        stolen: i64,
    ) -> Result<(), DomainError> {
        if earned < 0 || stolen < 0 {
            return Err(DomainError::ValidationError(
                "Les montants ne peuvent pas etre negatifs".into(),
            ));
        }
        let updated = self.repo.record_win(guild_id, user_id, earned, stolen).await?;
        if !updated {
            return Err(DomainError::NotFound("Joueur introuvable".into()));
        }
        Ok(())
    }

    async fn record_loss(
        &self,
        guild_id: &str,
        user_id: &str,
        lost: i64,
    ) -> Result<(), DomainError> {
        let updated = self.repo.record_loss(guild_id, user_id, lost).await?;
        if !updated {
            return Err(DomainError::NotFound("Joueur introuvable".into()));
        }
        Ok(())
    }

    async fn record_draw(
        &self,
        guild_id: &str,
        user_id: &str,
        lost: i64,
    ) -> Result<(), DomainError> {
        let updated = self.repo.record_draw(guild_id, user_id, lost).await?;
        if !updated {
            return Err(DomainError::NotFound("Joueur introuvable".into()));
        }
        Ok(())
    }

    async fn increment_cowardice(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i32, DomainError> {
        self.repo
            .increment_cowardice(guild_id, user_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("Joueur introuvable".into()))
    }

    async fn increment_chaos(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
        let updated = self.repo.increment_chaos(guild_id, user_id).await?;
        if !updated {
            return Err(DomainError::NotFound("Joueur introuvable".into()));
        }
        Ok(())
    }

    async fn record_coins_earned(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
    ) -> Result<(), DomainError> {
        if amount <= 0 {
            return Err(DomainError::ValidationError(
                "Le montant doit etre positif".into(),
            ));
        }
        let updated = self
            .repo
            .record_coins_earned(guild_id, user_id, amount)
            .await?;
        if !updated {
            return Err(DomainError::NotFound("Joueur introuvable".into()));
        }
        Ok(())
    }

    async fn record_coins_lost(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
    ) -> Result<(), DomainError> {
        let updated = self
            .repo
            .record_coins_lost(guild_id, user_id, amount)
            .await?;
        if !updated {
            return Err(DomainError::NotFound("Joueur introuvable".into()));
        }
        Ok(())
    }

    async fn update_hp(
        &self,
        guild_id: &str,
        user_id: &str,
        hp_current: i32,
        hp_max: i32,
    ) -> Result<(), DomainError> {
        self.repo
            .update_hp(guild_id, user_id, hp_current, hp_max)
            .await
    }

    async fn full_heal(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
        self.repo.full_heal(guild_id, user_id).await
    }

    async fn regen_hp_tick(
        &self,
        rate_0_25: f64,
        rate_25_50: f64,
        rate_50_75: f64,
        rate_75_100: f64,
    ) -> Result<u64, DomainError> {
        self.repo
            .regen_hp_tick(rate_0_25, rate_25_50, rate_50_75, rate_75_100)
            .await
    }
}

#[cfg(test)]
#[path = "tests/manage_players.rs"]
mod tests;
