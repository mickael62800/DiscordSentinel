use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::adapters::inbound::ws::broadcaster::EventBroadcaster;
use crate::domain::entities::{ConductConfig, ConductPointsLog, Infraction, UserConductPoints};
use crate::domain::errors::DomainError;
use crate::domain::value_objects::{Action, DetectionFlags};
use crate::ports::inbound::{
    AddPointsCommand, DeductPointsCommand, ManageConductUseCase, SaveConductConfigCommand,
};
use crate::ports::outbound::{ConductRepository, InfractionRepository};

pub struct ManageConductService {
    repo: Arc<dyn ConductRepository>,
    infraction_repo: Arc<dyn InfractionRepository>,
    broadcaster: Arc<EventBroadcaster>,
    discord_bot_token: String,
    http_client: reqwest::Client,
}

impl ManageConductService {
    pub fn new(repo: Arc<dyn ConductRepository>, infraction_repo: Arc<dyn InfractionRepository>, broadcaster: Arc<EventBroadcaster>, discord_bot_token: String) -> Self {
        Self { repo, infraction_repo, broadcaster, discord_bot_token, http_client: reqwest::Client::new() }
    }

    /// Mute un utilisateur via l'API Discord (timeout 10 minutes)
    async fn mute_user(&self, guild_id: &str, user_id: &str) {
        if self.discord_bot_token.is_empty() {
            tracing::warn!("SENTINEL_DISCORD_TOKEN non configure, mute impossible");
            return;
        }

        let timeout_until = Utc::now() + chrono::Duration::minutes(10);
        let url = format!("https://discord.com/api/v10/guilds/{}/members/{}", guild_id, user_id);

        match self.http_client
            .patch(&url)
            .header("Authorization", format!("Bot {}", self.discord_bot_token))
            .json(&serde_json::json!({
                "communication_disabled_until": timeout_until.to_rfc3339(),
            }))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                tracing::info!(guild_id, user_id, "Utilisateur mute (0 points)");
            }
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                tracing::error!(guild_id, user_id, %status, %body, "Echec mute Discord");
            }
            Err(e) => {
                tracing::error!(guild_id, user_id, error = %e, "Erreur connexion Discord pour mute");
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
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
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
        let points_after = (points_before - penalty).max(0);

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
                channel_id: "system:conduct".to_string(),
                user_id: cmd.user_id.clone(),
                username: cmd.username.clone(),
                message_id: "system:zero-points".to_string(),
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
        let points_after = (points_before + cmd.amount).min(config.max_points);

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
                let points_after = (points_before + config.regen_amount).min(config.max_points);

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
}
