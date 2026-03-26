use serenity::async_trait;
use serenity::model::gateway::Ready;
use serenity::model::guild::Member;
use serenity::prelude::*;
use tracing::{error, info, warn};

use crate::account_checker::AccountChecker;
use crate::api_client::{ApiClient, SecurityEvent};
use crate::raid_detector::RaidDetector;

// ── TypeMap keys ──

pub struct ApiClientKey;
impl TypeMapKey for ApiClientKey {
    type Value = ApiClient;
}

pub struct RaidDetectorKey;
impl TypeMapKey for RaidDetectorKey {
    type Value = RaidDetector;
}

pub struct AccountCheckerKey;
impl TypeMapKey for AccountCheckerKey {
    type Value = AccountChecker;
}

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, "Security bot connecté");
    }

    /// Déclenché à chaque nouveau membre qui rejoint un serveur.
    async fn guild_member_addition(&self, ctx: Context, new_member: Member) {
        let guild_id = new_member.guild_id;
        let user = &new_member.user;

        info!(
            guild_id = %guild_id,
            user = %user.name,
            user_id = %user.id,
            "Nouveau membre"
        );

        let data = ctx.data.read().await;
        let api = data.get::<ApiClientKey>().unwrap();
        let raid_detector = data.get::<RaidDetectorKey>().unwrap();
        let account_checker = data.get::<AccountCheckerKey>().unwrap();

        // ── 1. Détection anti-raid ──
        let is_raid = raid_detector.record_join(guild_id);

        if is_raid {
            let join_count = raid_detector.recent_joins(guild_id);

            warn!(
                guild_id = %guild_id,
                joins = join_count,
                "RAID DÉTECTÉ — activation lockdown"
            );

            // Signaler au backend
            let event = SecurityEvent {
                guild_id: guild_id.to_string(),
                event_type: "raid_detected".to_string(),
                severity: "critical".to_string(),
                description: format!(
                    "Raid détecté : {} joins en quelques secondes",
                    join_count
                ),
                user_ids: vec![user.id.to_string()],
            };

            if let Err(e) = api.report_event(&event).await {
                error!(error = %e, "Erreur envoi événement raid au backend");
            }

            // Activer le mode vérification du serveur (highest)
            if let Ok(mut guild) = guild_id.to_partial_guild(&ctx.http).await {
                let edit = serenity::builder::EditGuild::new()
                    .verification_level(serenity::model::guild::VerificationLevel::Higher);

                if let Err(e) = guild.edit(&ctx.http, edit).await {
                    error!(error = %e, "Impossible d'activer le lockdown");
                } else {
                    info!(guild_id = %guild_id, "Lockdown activé (verification: Highest)");
                }
            }

            // Envoyer une alerte dans le premier salon texte trouvé
            if let Ok(channels) = guild_id.channels(&ctx.http).await {
                if let Some(channel) = channels
                    .values()
                    .find(|c| c.kind == serenity::model::channel::ChannelType::Text)
                {
                    channel
                        .send_message(
                            &ctx.http,
                            serenity::builder::CreateMessage::new().content(format!(
                                "**ALERTE SÉCURITÉ** — Raid détecté ({} joins rapides). \
                                 Niveau de vérification augmenté automatiquement.",
                                join_count
                            )),
                        )
                        .await
                        .ok();
                }
            }

            raid_detector.reset(guild_id);
        }

        // ── 2. Vérification compte suspect ──
        if account_checker.is_suspicious(user) {
            let age_h = account_checker.account_age_hours(user);

            warn!(
                guild_id = %guild_id,
                user = %user.name,
                account_age_hours = age_h,
                "Compte suspect détecté (trop récent)"
            );

            let event = SecurityEvent {
                guild_id: guild_id.to_string(),
                event_type: "suspicious_account".to_string(),
                severity: "warning".to_string(),
                description: format!(
                    "Compte suspect : {} (créé il y a {}h)",
                    user.name, age_h
                ),
                user_ids: vec![user.id.to_string()],
            };

            if let Err(e) = api.report_event(&event).await {
                error!(error = %e, "Erreur envoi événement compte suspect");
            }
        }
    }
}
