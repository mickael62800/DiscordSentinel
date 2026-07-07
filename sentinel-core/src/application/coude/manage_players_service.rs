use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::entities::coude::player::CombatStat;
use crate::domain::entities::coude::player::Player;
use crate::domain::entities::coude::player::XpProgress;
use crate::domain::entities::system::discord_ids::GuildId;
use crate::domain::entities::system::discord_ids::UserId;
use crate::domain::errors::DomainError;
use crate::ports::inbound::coude::manage_players::ManageCoudePlayersUseCase;
use crate::ports::inbound::coude::manage_players::MilestoneView;
use crate::ports::inbound::coude::manage_players::PlayerProgression;
use crate::ports::outbound::coude::player_repository::PlayerRepository;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;

/// Cle de config du cooldown /repos (defaut 12h). Mirror de la lecture bot.
const REPOS_COOLDOWN_KEY: &str = "repos_cooldown_hours";
const REPOS_COOLDOWN_DEFAULT: i64 = 12;

pub struct ManageCoudePlayersService {
    repo: Arc<dyn PlayerRepository>,
    bot_config_repo: Option<Arc<dyn BotConfigRepository>>,
}

impl ManageCoudePlayersService {
    pub fn new(repo: Arc<dyn PlayerRepository>) -> Self {
        Self {
            repo,
            bot_config_repo: None,
        }
    }

    /// Branche le repo de config bot : lecture server-side du cooldown /repos
    /// configure par guild (progression / cooldown effectif).
    pub fn with_bot_config_repo(mut self, repo: Arc<dyn BotConfigRepository>) -> Self {
        self.bot_config_repo = Some(repo);
        self
    }

    /// Cooldown /repos configure par la guild (defaut 12h). Sans repo de
    /// config : valeur par defaut historique.
    async fn base_repos_cooldown_hours(&self, guild_id: &str) -> i64 {
        match &self.bot_config_repo {
            Some(repo) => {
                crate::application::coude::guild_settings::GuildSettings::load(&**repo, guild_id)
                    .await
                    .get_i64(REPOS_COOLDOWN_KEY, REPOS_COOLDOWN_DEFAULT)
            }
            None => REPOS_COOLDOWN_DEFAULT,
        }
    }

    async fn require_player(&self, guild_id: &str, user_id: &str) -> Result<Player, DomainError> {
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
        guild_id: GuildId,
        user_id: UserId,
        username: String,
    ) -> Result<Player, DomainError> {
        self.repo
            .get_or_create(&guild_id, &user_id, &username)
            .await
    }

    async fn get(&self, guild_id: &str, user_id: &str) -> Result<Player, DomainError> {
        self.require_player(guild_id, user_id).await
    }

    async fn list(&self, guild_id: &str) -> Result<Vec<Player>, DomainError> {
        // 200 = la limite historique du handler legacy.
        self.repo.list(guild_id, 200).await
    }

    async fn random_active(&self, guild_id: &str, count: i64) -> Result<Vec<Player>, DomainError> {
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
    ) -> Result<Player, DomainError> {
        self.repo
            .spend_stat_point(guild_id, user_id, stat)
            .await?
            .ok_or_else(|| {
                DomainError::ValidationError(
                    "Joueur introuvable ou pas de stat_points disponibles".into(),
                )
            })
    }

    async fn reset_stats(&self, guild_id: &str, user_id: &str) -> Result<Player, DomainError> {
        // Cout lu server-side (config guild `reset_stats_cost`, defaut 300 —
        // meme cle et defaut que l'ancien bot).
        let cost = match &self.bot_config_repo {
            Some(repo) => {
                crate::application::coude::guild_settings::GuildSettings::load(&**repo, guild_id)
                    .await
                    .get_i64("reset_stats_cost", 300)
            }
            None => 300,
        };
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
        let updated = self
            .repo
            .record_win(guild_id, user_id, earned, stolen)
            .await?;
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

    async fn increment_cowardice(&self, guild_id: &str, user_id: &str) -> Result<i32, DomainError> {
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
        crate::application::validation::validate_positive(amount, "Le montant")?;
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

    async fn effective_repos_cooldown_hours(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, DomainError> {
        let player = self.require_player(guild_id, user_id).await?;
        let base = self.base_repos_cooldown_hours(guild_id).await;
        Ok(
            crate::domain::services::coude::milestones::effective_repos_cooldown_hours(
                base,
                player.level,
            ),
        )
    }

    async fn get_progression(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<PlayerProgression, DomainError> {
        use crate::domain::services::coude::{achievements, milestones};

        let player = self.require_player(guild_id, user_id).await?;
        let base = self.base_repos_cooldown_hours(guild_id).await;

        let unlocked_achievements = achievements::unlocked_keys(&player)
            .into_iter()
            .map(str::to_string)
            .collect();

        let to_view = |m: &milestones::Milestone| MilestoneView {
            level: m.level,
            key: m.key.to_string(),
            label: m.label.to_string(),
            emoji: m.emoji.to_string(),
            description: m.description.to_string(),
            unlocked: milestones::is_unlocked(m, player.level),
        };
        let milestone_views = milestones::MILESTONES.iter().map(to_view).collect();
        let next_milestone = milestones::next_for(player.level).map(to_view);

        Ok(PlayerProgression {
            unlocked_achievements,
            total_achievements: achievements::total_achievements() as i32,
            milestones: milestone_views,
            next_milestone,
            effective_repos_cooldown_hours: milestones::effective_repos_cooldown_hours(
                base,
                player.level,
            ),
        })
    }
}

#[cfg(test)]
#[path = "tests/manage_players.rs"]
mod tests;
