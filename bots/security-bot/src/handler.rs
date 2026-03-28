use serenity::async_trait;
use serenity::model::application::Interaction;
use serenity::model::channel::GuildChannel;
use serenity::model::gateway::Ready;
use serenity::model::guild::Member;
use serenity::model::id::{GuildId, RoleId};
use serenity::prelude::*;
use tracing::{error, info, warn};

use crate::account_checker::AccountChecker;
use crate::api_client::{ApiClient, SecurityEvent};
use crate::captcha;
use crate::config::Config;
use crate::quarantine::QuarantineManager;
use crate::raid_detector::RaidDetector;
use crate::slowmode::SlowmodeManager;

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

pub struct ConfigKey;
impl TypeMapKey for ConfigKey {
    type Value = Config;
}

pub struct QuarantineKey;
impl TypeMapKey for QuarantineKey {
    type Value = QuarantineManager;
}

pub struct SlowmodeKey;
impl TypeMapKey for SlowmodeKey {
    type Value = SlowmodeManager;
}

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, "Security bot connecté");

        let data = ctx.data.read().await;
        if let Some(api) = data.get::<ApiClientKey>() {
            api.send_bot_log("info", "Security bot demarre");
            for guild_status in &ready.guilds {
                let guild_id = guild_status.id;
                if let Ok(guild) = guild_id.to_partial_guild(&ctx.http).await {
                    let member_count = guild.approximate_member_count.unwrap_or(0) as i32;
                    if let Err(e) = api.register_guild(
                        &guild_id.to_string(),
                        &guild.name,
                        member_count,
                    ).await {
                        warn!(error = %e, guild = %guild.name, "Erreur enregistrement guild");
                    } else {
                        info!(guild = %guild.name, "Guild enregistree");
                    }
                }
            }
        }
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

        // Log l'arrivée dans le journal
        if let Some(api) = data.get::<ApiClientKey>() {
            api.send_log(
                "info",
                &guild_id.to_string(),
                &format!("Nouveau membre : {} ({})", user.name, user.id),
            );
        }
        let api = data.get::<ApiClientKey>().unwrap();
        let raid_detector = data.get::<RaidDetectorKey>().unwrap();
        let account_checker = data.get::<AccountCheckerKey>().unwrap();
        let env_config = data.get::<ConfigKey>().unwrap();
        let quarantine = data.get::<QuarantineKey>().unwrap();
        let slowmode = data.get::<SlowmodeKey>().unwrap();

        // Charger la config per-guild depuis l'API (fallback sur env vars)
        let guild_config = api.get_guild_config(&guild_id.to_string()).await.unwrap_or_default();
        let min_account_age = ApiClient::config_u64(&guild_config, "min_account_age_secs", env_config.min_account_age_secs);

        // Config quarantaine per-guild
        let quarantine_enabled = guild_config
            .get("quarantine_enabled")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(env_config.quarantine_enabled);
        let quarantine_role_id = guild_config
            .get("quarantine_role_id")
            .and_then(|v| v.parse::<u64>().ok())
            .or(env_config.quarantine_role_id);
        let captcha_enabled = guild_config
            .get("captcha_enabled")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(env_config.captcha_enabled);
        let slowmode_secs: u16 = guild_config
            .get("slowmode_seconds")
            .and_then(|v| v.parse().ok())
            .unwrap_or(env_config.slowmode_seconds);

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
                    "Raid détecté : {} joins en quelques secondes. Actions: lockdown{}{}",
                    join_count,
                    if slowmode_secs > 0 { ", slowmode auto" } else { "" },
                    if quarantine_enabled { ", quarantaine" } else { "" },
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

            // ── Slowmode auto ──
            if slowmode_secs > 0 {
                slowmode.activate(&ctx, guild_id, slowmode_secs).await;
            }

            // ── Quarantaine sur le membre qui a déclenché ──
            if quarantine_enabled {
                if let Some(role_id) = quarantine_role_id {
                    quarantine
                        .quarantine_user(&ctx, guild_id, user.id, RoleId::new(role_id))
                        .await;

                    if captcha_enabled {
                        let guild_name = guild_id
                            .to_partial_guild(&ctx.http)
                            .await
                            .map(|g| g.name.clone())
                            .unwrap_or_else(|_| "Serveur".to_string());

                        captcha::send_challenge(&ctx, user.id, &guild_name).await;
                    }
                }
            }

            // Envoyer une alerte dans le premier salon texte trouvé
            if let Ok(channels) = guild_id.channels(&ctx.http).await {
                if let Some(channel) = channels
                    .values()
                    .find(|c| c.kind == serenity::model::channel::ChannelType::Text)
                {
                    let mut alert = format!(
                        "**🚨 ALERTE SÉCURITÉ** — Raid détecté ({} joins rapides).\n\
                         Niveau de vérification augmenté automatiquement.",
                        join_count
                    );
                    if slowmode_secs > 0 {
                        alert.push_str(&format!(
                            "\n⏱️ Slowmode activé ({}s) sur tous les salons.",
                            slowmode_secs
                        ));
                    }
                    if quarantine_enabled {
                        alert.push_str("\n🔒 Nouveaux membres mis en quarantaine.");
                    }

                    channel
                        .send_message(
                            &ctx.http,
                            serenity::builder::CreateMessage::new().content(alert),
                        )
                        .await
                        .ok();
                }
            }

            raid_detector.reset(guild_id);
        }

        // ── 2. Vérification compte suspect ──
        let per_guild_checker = AccountChecker::new(min_account_age);
        let checker = if guild_config.contains_key("min_account_age_secs") {
            &per_guild_checker
        } else {
            account_checker
        };

        if checker.is_suspicious(user) {
            let age_h = checker.account_age_hours(user);

            warn!(
                guild_id = %guild_id,
                user = %user.name,
                account_age_hours = age_h,
                "Compte suspect détecté (trop récent)"
            );

            let mut description = format!(
                "Compte suspect : {} (créé il y a {}h)",
                user.name, age_h
            );

            // ── Quarantaine pour comptes suspects ──
            if quarantine_enabled {
                if let Some(role_id) = quarantine_role_id {
                    let quarantined = quarantine
                        .quarantine_user(&ctx, guild_id, user.id, RoleId::new(role_id))
                        .await;

                    if quarantined {
                        description.push_str(" — mis en quarantaine");

                        if captcha_enabled {
                            let guild_name = guild_id
                                .to_partial_guild(&ctx.http)
                                .await
                                .map(|g| g.name.clone())
                                .unwrap_or_else(|_| "Serveur".to_string());

                            captcha::send_challenge(&ctx, user.id, &guild_name).await;
                        }
                    }
                }
            }

            let event = SecurityEvent {
                guild_id: guild_id.to_string(),
                event_type: "suspicious_account".to_string(),
                severity: "warning".to_string(),
                description,
                user_ids: vec![user.id.to_string()],
            };

            if let Err(e) = api.report_event(&event).await {
                error!(error = %e, "Erreur envoi événement compte suspect");
            }
        }
    }

    /// Déclenché quand un membre quitte le serveur.
    async fn guild_member_removal(
        &self,
        ctx: Context,
        guild_id: GuildId,
        user: serenity::model::user::User,
        _member: Option<Member>,
    ) {
        info!(guild_id = %guild_id, user = %user.name, "Membre parti");

        let data = ctx.data.read().await;
        if let Some(api) = data.get::<ApiClientKey>() {
            api.send_log(
                "info",
                &guild_id.to_string(),
                &format!("Membre parti : {} ({})", user.name, user.id),
            );
        }
    }

    /// Déclenché quand un salon est créé.
    async fn channel_create(&self, ctx: Context, channel: GuildChannel) {
        let guild_id = channel.guild_id;
        let kind = match channel.kind {
            serenity::model::channel::ChannelType::Text => "texte",
            serenity::model::channel::ChannelType::Voice => "vocal",
            serenity::model::channel::ChannelType::Category => "categorie",
            serenity::model::channel::ChannelType::Stage => "stage",
            serenity::model::channel::ChannelType::Forum => "forum",
            _ => "autre",
        };

        info!(guild_id = %guild_id, channel = %channel.name, kind, "Salon créé");

        let data = ctx.data.read().await;
        if let Some(api) = data.get::<ApiClientKey>() {
            api.send_log(
                "info",
                &guild_id.to_string(),
                &format!("Salon {} créé : {} ({})", kind, channel.name, channel.id),
            );
        }
    }

    /// Déclenché quand un salon est supprimé.
    async fn channel_delete(&self, ctx: Context, channel: GuildChannel, _messages: Option<Vec<serenity::model::channel::Message>>) {
        let guild_id = channel.guild_id;
        let kind = match channel.kind {
            serenity::model::channel::ChannelType::Text => "texte",
            serenity::model::channel::ChannelType::Voice => "vocal",
            serenity::model::channel::ChannelType::Category => "categorie",
            _ => "autre",
        };

        info!(guild_id = %guild_id, channel = %channel.name, kind, "Salon supprimé");

        let data = ctx.data.read().await;
        if let Some(api) = data.get::<ApiClientKey>() {
            api.send_log(
                "warn",
                &guild_id.to_string(),
                &format!("Salon {} supprimé : {}", kind, channel.name),
            );
        }
    }

    /// Déclenché quand un membre est banni.
    async fn guild_ban_addition(&self, ctx: Context, guild_id: GuildId, banned_user: serenity::model::user::User) {
        info!(guild_id = %guild_id, user = %banned_user.name, "Membre banni");

        let data = ctx.data.read().await;
        if let Some(api) = data.get::<ApiClientKey>() {
            api.send_log(
                "warn",
                &guild_id.to_string(),
                &format!("Membre banni : {} ({})", banned_user.name, banned_user.id),
            );
        }
    }

    /// Déclenché quand un membre est débanni.
    async fn guild_ban_removal(&self, ctx: Context, guild_id: GuildId, unbanned_user: serenity::model::user::User) {
        info!(guild_id = %guild_id, user = %unbanned_user.name, "Membre débanni");

        let data = ctx.data.read().await;
        if let Some(api) = data.get::<ApiClientKey>() {
            api.send_log(
                "info",
                &guild_id.to_string(),
                &format!("Membre débanni : {} ({})", unbanned_user.name, unbanned_user.id),
            );
        }
    }

    /// Gère les interactions (bouton captcha).
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Component(component) = interaction {
            if component.data.custom_id != captcha::CAPTCHA_BUTTON_ID {
                return;
            }

            let user_id = component.user.id;

            let data = ctx.data.read().await;
            let quarantine = data.get::<QuarantineKey>().unwrap();
            let env_config = data.get::<ConfigKey>().unwrap();
            let api = data.get::<ApiClientKey>().unwrap();

            // Trouver dans quelle guild l'utilisateur est en quarantaine
            let mut released = false;

            {
                let cache = &ctx.cache;
                for guild_id in cache.guilds() {
                    if !quarantine.is_quarantined(guild_id, user_id) {
                        continue;
                    }

                    let guild_config = api
                        .get_guild_config(&guild_id.to_string())
                        .await
                        .unwrap_or_default();
                    let role_id = guild_config
                        .get("quarantine_role_id")
                        .and_then(|v| v.parse::<u64>().ok())
                        .or(env_config.quarantine_role_id);

                    if let Some(role_id) = role_id {
                        quarantine
                            .release_user(&ctx, guild_id, user_id, RoleId::new(role_id))
                            .await;

                        let event = SecurityEvent {
                            guild_id: guild_id.to_string(),
                            event_type: "captcha_verified".to_string(),
                            severity: "info".to_string(),
                            description: format!(
                                "Utilisateur {} a passé le captcha",
                                component.user.name
                            ),
                            user_ids: vec![user_id.to_string()],
                        };
                        api.report_event(&event).await.ok();

                        released = true;
                    }
                }
            }

            let content = if released {
                "✅ **Vérification réussie !** Vous avez maintenant accès au serveur."
            } else {
                "⚠️ Vous n'êtes pas en quarantaine ou la vérification a déjà été effectuée."
            };

            let response = serenity::builder::CreateInteractionResponse::Message(
                serenity::builder::CreateInteractionResponseMessage::new()
                    .content(content)
                    .ephemeral(true),
            );

            component.create_response(&ctx.http, response).await.ok();
        }
    }
}
