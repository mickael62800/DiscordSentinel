use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::adapters::inbound::ws::broadcaster::EventBroadcaster;
use crate::adapters::outbound::discord_api::DiscordApi;
use crate::domain::entities::community::conduct::apply_conduct_penalty;
use crate::domain::entities::community::conduct::apply_conduct_regen;
use crate::domain::entities::community::conduct::ConductConfig;
use crate::domain::entities::community::conduct::ConductPointsLog;
use crate::domain::entities::moderation::infraction::Infraction;
use crate::domain::entities::community::conduct::UserConductPoints;
use crate::domain::entities::community::conduct::MUTE_AT_ZERO_POINTS_DURATION_MINS;
use crate::domain::errors::DomainError;
use crate::domain::enums::moderation::action::Action;
use crate::domain::entities::moderation::detection_flags::DetectionFlags;
use crate::ports::inbound::community::manage_conduct::AddPointsCommand;
use crate::ports::inbound::community::manage_conduct::DeductPointsCommand;
use crate::ports::inbound::community::manage_conduct::ManageConductUseCase;
use crate::ports::inbound::community::manage_conduct::SaveConductConfigCommand;
use crate::ports::outbound::community::conduct_repository::ConductRepository;
use crate::ports::outbound::moderation::infraction_repository::InfractionRepository;
#[cfg(test)]
#[path = "tests/manage_conduct.rs"]
mod tests;

pub struct ManageConductService {
    repo: Arc<dyn ConductRepository>,
    infraction_repo: Arc<dyn InfractionRepository>,
    broadcaster: Arc<EventBroadcaster>,
    discord_api: Arc<dyn DiscordApi>,
}

impl ManageConductService {
    pub fn new(
        repo: Arc<dyn ConductRepository>,
        infraction_repo: Arc<dyn InfractionRepository>,
        broadcaster: Arc<EventBroadcaster>,
        discord_api: Arc<dyn DiscordApi>,
    ) -> Self {
        Self { repo, infraction_repo, broadcaster, discord_api }
    }

    /// Mute un utilisateur via l'API Discord (timeout 10 minutes).
    /// Les erreurs Discord sont loggees mais n'interrompent pas le flow
    /// (l'infraction de ban est deja persistee).
    async fn mute_user(&self, guild_id: &str, user_id: &str) {
        let duration = (MUTE_AT_ZERO_POINTS_DURATION_MINS * 60) as u64;
        match self.discord_api.apply_timeout(guild_id, user_id, duration).await {
            Ok(()) => {
                tracing::info!(guild_id, user_id, "Utilisateur mute (0 points)");
            }
            Err(e) => {
                tracing::error!(guild_id, user_id, error = %e, "Echec mute Discord");
            }
        }
    }

    async fn get_or_create_points(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        max_points: i32,
    ) -> Result<UserConductPoints, DomainError> {
        if let Some(points) = self.repo.get_points(guild_id, user_id).await? {
            return Ok(points);
        }

        let now = Utc::now();
        let points = UserConductPoints {
            id: Uuid::new_v4(),
            guild_id: guild_id.to_string().into(),
            user_id: user_id.to_string().into(),
            username: username.to_string(),
            points: max_points,
            last_regen_at: now,
            created_at: now,
            updated_at: now,
        };

        self.repo.save_points(&points).await?;
        Ok(points)
    }
}

#[async_trait]
impl ManageConductUseCase for ManageConductService {
    async fn get_config(&self, guild_id: &str) -> Result<ConductConfig, DomainError> {
        Ok(self
            .repo
            .get_config(guild_id)
            .await?
            .unwrap_or_else(|| ConductConfig::default_for_guild(guild_id)))
    }

    async fn save_config(
        &self,
        cmd: SaveConductConfigCommand,
    ) -> Result<ConductConfig, DomainError> {
        let now = Utc::now();
        let config = ConductConfig {
            guild_id: cmd.guild_id,
            max_points: cmd.max_points,
            regen_amount: cmd.regen_amount,
            regen_interval: cmd.regen_interval,
            penalty_warn: cmd.penalty_warn,
            penalty_delete: cmd.penalty_delete,
            penalty_mute: cmd.penalty_mute,
            penalty_ban: cmd.penalty_ban,
            created_at: now,
            updated_at: now,
        };

        self.repo.save_config(&config).await?;
        Ok(config)
    }

    async fn get_points(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<UserConductPoints, DomainError> {
        let config = self.get_config(guild_id).await?;
        self.get_or_create_points(guild_id, user_id, "", config.max_points)
            .await
    }

    async fn deduct_points(
        &self,
        cmd: DeductPointsCommand,
    ) -> Result<UserConductPoints, DomainError> {
        let config = self.get_config(&cmd.guild_id).await?;
        let penalty = config.penalty_for_action(&cmd.action);

        if penalty == 0 {
            return self
                .get_or_create_points(&cmd.guild_id, &cmd.user_id, &cmd.username, config.max_points)
                .await;
        }

        let mut user_points = self
            .get_or_create_points(&cmd.guild_id, &cmd.user_id, &cmd.username, config.max_points)
            .await?;

        let points_before = user_points.points;
        let points_after = apply_conduct_penalty(points_before, penalty);

        self.repo
            .update_points(&cmd.guild_id, &cmd.user_id, points_after)
            .await?;

        // Log le mouvement
        let log = ConductPointsLog {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id.clone(),
            user_id: cmd.user_id.clone(),
            delta: -penalty,
            reason: format!("Infraction: {} (-{} points)", cmd.action, penalty),
            points_before,
            points_after,
            created_at: Utc::now(),
        };
        self.repo.save_log(&log).await?;

        user_points.points = points_after;

        // Mute + proposition de ban si 0 points
        if points_after == 0 {
            self.mute_user(&cmd.guild_id, &cmd.user_id).await;
            let reason = format!("Points de conduite tombes a 0 (derniere infraction: {})", cmd.action);
            tracing::warn!(
                guild_id = %cmd.guild_id,
                user_id = %cmd.user_id,
                username = %cmd.username,
                last_action = %cmd.action,
                "Utilisateur a 0 points de conduite — proposition de ban"
            );
            let infraction = Infraction {
                id: Uuid::new_v4(),
                guild_id: cmd.guild_id.clone(),
                channel_id: "system:conduct".to_string().into(),
                user_id: cmd.user_id.clone(),
                username: cmd.username.clone(),
                display_name: None,
                message_id: "system:zero-points".into(),
                content: format!("[Systeme] {} a atteint 0 points de conduite", cmd.username),
                flags: DetectionFlags { spam: false, insult: false, link: false, phishing: false },
                score: 0.0,
                action: Action::Ban,
                reason,
                duration: None,
                created_at: Utc::now(),
            };
            if let Err(e) = self.infraction_repo.save(&infraction).await {
                tracing::error!(error = %e, guild_id = %cmd.guild_id, user_id = %cmd.user_id, "CRITIQUE: Echec sauvegarde infraction ban (0 points)");
            }

            self.broadcaster.broadcast(
                "user_zero_points",
                serde_json::json!({
                    "guild_id": &cmd.guild_id,
                    "user_id": &cmd.user_id,
                    "username": &cmd.username,
                    "action": &cmd.action,
                }),
            );
        }

        Ok(user_points)
    }

    async fn add_points(&self, cmd: AddPointsCommand) -> Result<UserConductPoints, DomainError> {
        let config = self.get_config(&cmd.guild_id).await?;
        let mut user_points = self
            .get_or_create_points(&cmd.guild_id, &cmd.user_id, "", config.max_points)
            .await?;

        let points_before = user_points.points;
        let points_after = apply_conduct_regen(points_before, cmd.amount, config.max_points);

        self.repo
            .update_points(&cmd.guild_id, &cmd.user_id, points_after)
            .await?;

        let log = ConductPointsLog {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id.clone(),
            user_id: cmd.user_id.clone(),
            delta: cmd.amount,
            reason: cmd.reason,
            points_before,
            points_after,
            created_at: Utc::now(),
        };
        self.repo.save_log(&log).await?;

        user_points.points = points_after;
        Ok(user_points)
    }

    async fn get_leaderboard(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<UserConductPoints>, DomainError> {
        self.repo.get_leaderboard(guild_id, limit).await
    }

    async fn get_points_log(
        &self,
        guild_id: &str,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<ConductPointsLog>, DomainError> {
        self.repo.get_log(guild_id, user_id, limit).await
    }

    async fn run_regen(&self) -> Result<u64, DomainError> {
        // Recuperer toutes les configs
        // Pour chaque config, trouver les users dont last_regen_at + interval est passe
        let mut total = 0u64;

        for interval in &["weekly", "monthly"] {
            let users = self.repo.find_users_needing_regen(interval).await?;

            for user in users {
                // Recuperer la config du guild
                let config = self.get_config(&user.guild_id).await?;
                if config.regen_interval != *interval {
                    continue;
                }

                if user.points >= config.max_points {
                    // Deja au max → supprimer de la table (utilisateur "propre")
                    self.repo
                        .delete_points(&user.guild_id, &user.user_id)
                        .await?;
                    total += 1;
                    continue;
                }

                let points_before = user.points;
                let points_after = apply_conduct_regen(points_before, config.regen_amount, config.max_points);

                if points_after >= config.max_points {
                    // Revenu au max → supprimer de la table
                    let log = ConductPointsLog {
                        id: Uuid::new_v4(),
                        guild_id: user.guild_id.clone(),
                        user_id: user.user_id.clone(),
                        delta: config.regen_amount,
                        reason: format!("Regeneration {} (+{} points) — retour au max, supprime", interval, config.regen_amount),
                        points_before,
                        points_after: config.max_points,
                        created_at: Utc::now(),
                    };
                    self.repo.save_log(&log).await?;
                    self.repo
                        .delete_points(&user.guild_id, &user.user_id)
                        .await?;
                } else {
                    // Pas encore au max → mettre a jour
                    self.repo
                        .update_points(&user.guild_id, &user.user_id, points_after)
                        .await?;
                    self.repo
                        .update_regen_timestamp(&user.guild_id, &user.user_id)
                        .await?;

                    let log = ConductPointsLog {
                        id: Uuid::new_v4(),
                        guild_id: user.guild_id.clone(),
                        user_id: user.user_id.clone(),
                        delta: config.regen_amount,
                        reason: format!("Regeneration {} (+{} points)", interval, config.regen_amount),
                        points_before,
                        points_after,
                        created_at: Utc::now(),
                    };
                    self.repo.save_log(&log).await?;
                }

                total += 1;
            }
        }

        Ok(total)
    }

    async fn sync_ban_proposals(&self) -> Result<u64, DomainError> {
        // Reason prefix utilise pour deduplication (matching `LIKE 'Points de conduite%'`)
        const REASON_PREFIX: &str = "Points de conduite";
        const REASON_FULL: &str = "Points de conduite tombes a 0";

        let users = self
            .repo
            .find_zero_points_users_without_ban_proposal(REASON_PREFIX)
            .await?;

        let mut count = 0u64;
        for user in users {
            let infraction = Infraction {
                id: Uuid::new_v4(),
                guild_id: user.guild_id.clone(),
                channel_id: String::new().into(),
                user_id: user.user_id.clone(),
                username: user.username.clone(),
                display_name: None,
                message_id: String::new().into(),
                content: String::new(),
                flags: DetectionFlags { spam: false, insult: false, link: false, phishing: false },
                score: 0.0,
                action: Action::Ban,
                reason: REASON_FULL.into(),
                duration: None,
                created_at: Utc::now(),
            };
            if let Err(e) = self.infraction_repo.save(&infraction).await {
                tracing::warn!(error = %e, guild_id = %user.guild_id, user_id = %user.user_id, "sync_ban_proposals: save infraction echoue");
                continue;
            }
            count += 1;
        }

        Ok(count)
    }
}
