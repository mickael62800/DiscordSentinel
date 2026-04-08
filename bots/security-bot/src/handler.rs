use chrono::DateTime;
use serenity::async_trait;
use serenity::model::application::Interaction;
use serenity::model::channel::GuildChannel;
use serenity::model::gateway::Ready;
use serenity::model::guild::Member;
use serenity::model::id::{GuildId, RoleId, UserId};
use serenity::prelude::*;
use tracing::{error, info, warn};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::embeds::{danger_embed, success_embed, warn_embed};
use sentinel_shared::heartbeat::{ApiClientKey, register_guilds};

use crate::api_client::{ApiClient, MemberPayload, SecurityEvent, SyncMembersPayload, UpdateMemberPayload};
use crate::commands;
use crate::config::Config;
use crate::security::account_checker::AccountChecker;
use crate::security::alt_detector::AltDetector;
use crate::security::captcha::{self, CaptchaPending};
use crate::security::lockdown::LockdownManager;
use crate::security::quarantine::QuarantineManager;
use crate::security::raid_analyzer::{self, JoinInfo, RecentJoinsTracker};
use crate::security::raid_detector::RaidDetector;
use crate::security::slowmode::SlowmodeManager;

// ── TypeMap keys ──

pub struct SecurityApiKey;
impl TypeMapKey for SecurityApiKey {
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

pub struct LockdownKey;
impl TypeMapKey for LockdownKey {
    type Value = LockdownManager;
}

pub struct RecentJoinsKey;
impl TypeMapKey for RecentJoinsKey {
    type Value = RecentJoinsTracker;
}

pub struct CaptchaPendingKey;
impl TypeMapKey for CaptchaPendingKey {
    type Value = CaptchaPending;
}

pub struct AltDetectorKey;
impl TypeMapKey for AltDetectorKey {
    type Value = AltDetector;
}

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, "Security bot connecte");
        register_guilds(&ctx, &ready).await;

        if let Err(e) = serenity::model::application::Command::set_global_commands(
            &ctx.http,
            commands::all(),
        )
        .await
        {
            error!(error = %e, "Erreur enregistrement commandes");
        } else {
            info!("Slash commands enregistrees : security");
        }

        // ── Sync des membres au demarrage ──
        let ctx_clone = ctx.clone();
        tokio::spawn(async move {
            let data = ctx_clone.data.read().await;
            let sec_api = match data.get::<SecurityApiKey>() {
                Some(a) => a,
                None => { error!("SecurityApiKey manquant pour sync membres"); return; }
            };

            for guild in &ready.guilds {
                let guild_id = guild.id;
                match guild_id.members(&ctx_clone.http, None, None).await {
                    Ok(members) => {
                        let payloads: Vec<MemberPayload> = members.iter().map(|m| {
                            let roles: Vec<String> = m.roles.iter().map(|r| r.to_string()).collect();
                            MemberPayload {
                                guild_id: guild_id.to_string(),
                                user_id: m.user.id.to_string(),
                                username: m.user.name.clone(),
                                display_name: m.nick.clone(),
                                avatar: m.user.avatar.as_ref().map(|a| a.to_string()),
                                roles: serde_json::json!(roles),
                                joined_at: m.joined_at.map(|t| DateTime::from_timestamp(t.unix_timestamp(), 0)).flatten(),
                                account_created: Some(DateTime::from_timestamp(m.user.created_at().unix_timestamp(), 0)).flatten(),
                                is_bot: m.user.bot,
                                last_seen_at: None,
                            }
                        }).collect();

                        let count = payloads.len();
                        let payload = SyncMembersPayload {
                            guild_id: guild_id.to_string(),
                            members: payloads,
                        };

                        match sec_api.sync_members(&payload).await {
                            Ok(()) => info!(guild_id = %guild_id, members = count, "Membres synchronises"),
                            Err(e) => error!(guild_id = %guild_id, error = %e, "Erreur sync membres"),
                        }
                    }
                    Err(e) => error!(guild_id = %guild_id, error = %e, "Impossible de recuperer les membres"),
                }
            }
        });
    }

    /// Declenche a chaque nouveau membre qui rejoint un serveur.
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

        // Enregistrer le membre dans la BDD
        if let Some(sec_api) = data.get::<SecurityApiKey>() {
            let roles: Vec<String> = new_member.roles.iter().map(|r| r.to_string()).collect();
            let member_payload = MemberPayload {
                guild_id: guild_id.to_string(),
                user_id: user.id.to_string(),
                username: user.name.clone(),
                display_name: new_member.nick.clone(),
                avatar: user.avatar.as_ref().map(|a| a.to_string()),
                roles: serde_json::json!(roles),
                joined_at: new_member.joined_at.map(|t| DateTime::from_timestamp(t.unix_timestamp(), 0)).flatten(),
                account_created: Some(DateTime::from_timestamp(user.created_at().unix_timestamp(), 0)).flatten(),
                is_bot: user.bot,
                last_seen_at: None,
            };
            if let Err(e) = sec_api.register_member(&member_payload).await {
                warn!(error = %e, "Erreur register_member");
            }
        }

        // Log l'arrivee dans le journal
        if let Some(base) = data.get::<ApiClientKey>() {
            base.send_log(
                "info",
                &guild_id.to_string(),
                &format!("Nouveau membre : {} ({})", user.name, user.id),
            );
        }
        let base = match data.get::<ApiClientKey>() {
            Some(a) => a,
            None => { error!(guild_id = %guild_id, "ApiClientKey manquant"); return; }
        };
        let sec_api = match data.get::<SecurityApiKey>() {
            Some(a) => a,
            None => { error!(guild_id = %guild_id, "SecurityApiKey manquant"); return; }
        };
        let raid_detector = match data.get::<RaidDetectorKey>() {
            Some(a) => a,
            None => { error!(guild_id = %guild_id, "RaidDetectorKey manquant"); return; }
        };
        let account_checker = match data.get::<AccountCheckerKey>() {
            Some(a) => a,
            None => { error!(guild_id = %guild_id, "AccountCheckerKey manquant"); return; }
        };
        let env_config = match data.get::<ConfigKey>() {
            Some(a) => a,
            None => { error!(guild_id = %guild_id, "ConfigKey manquant"); return; }
        };
        let quarantine = match data.get::<QuarantineKey>() {
            Some(a) => a,
            None => { error!(guild_id = %guild_id, "QuarantineKey manquant"); return; }
        };
        let slowmode = match data.get::<SlowmodeKey>() {
            Some(a) => a,
            None => { error!(guild_id = %guild_id, "SlowmodeKey manquant"); return; }
        };
        let lockdown = match data.get::<LockdownKey>() {
            Some(a) => a,
            None => { error!(guild_id = %guild_id, "LockdownKey manquant"); return; }
        };
        let recent_joins = match data.get::<RecentJoinsKey>() {
            Some(a) => a,
            None => { error!(guild_id = %guild_id, "RecentJoinsKey manquant"); return; }
        };
        let captcha_pending = match data.get::<CaptchaPendingKey>() {
            Some(a) => a,
            None => { error!(guild_id = %guild_id, "CaptchaPendingKey manquant"); return; }
        };
        let alt_detector = match data.get::<AltDetectorKey>() {
            Some(a) => a,
            None => { error!(guild_id = %guild_id, "AltDetectorKey manquant"); return; }
        };

        // Charger la config per-guild depuis l'API (fallback sur env vars)
        let guild_config = match base.get_guild_config(&guild_id.to_string()).await {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::warn!(error = %e, guild_id = %guild_id, "Echec chargement config guild");
                std::collections::HashMap::new()
            }
        };

        if !BaseApiClient::config_bool(&guild_config, "enabled", true) {
            return;
        }

        let min_account_age = BaseApiClient::config_u64(&guild_config, "min_account_age_secs", env_config.min_account_age_secs);

        // Config quarantaine per-guild
        let quarantine_enabled = guild_config
            .get("quarantine_enabled")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(env_config.quarantine_enabled);
        let quarantine_role_id = guild_config
            .get("quarantine_role_id")
            .and_then(|v| {
                v.parse::<u64>().map_err(|_| {
                    tracing::warn!(guild=%guild_id, value=%v, "quarantine_role_id invalide dans la config guild");
                }).ok()
            })
            .or(env_config.quarantine_role_id);
        let captcha_enabled = guild_config
            .get("captcha_enabled")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(env_config.captcha_enabled);
        let slowmode_secs: u16 = guild_config
            .get("slowmode_seconds")
            .and_then(|v| v.parse().ok())
            .unwrap_or(env_config.slowmode_seconds);
        let lockdown_enabled = guild_config
            .get("lockdown_enabled")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(env_config.lockdown_enabled);
        let captcha_type = guild_config
            .get("captcha_type")
            .cloned()
            .unwrap_or_else(|| env_config.captcha_type.clone());
        let alt_detection_enabled = guild_config
            .get("alt_detection_enabled")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(env_config.alt_detection_enabled);
        let raid_pattern_enabled = guild_config
            .get("raid_pattern_enabled")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(env_config.raid_pattern_enabled);
        let raid_pattern_score_threshold = guild_config
            .get("raid_pattern_score_threshold")
            .and_then(|v| v.parse().ok())
            .unwrap_or(env_config.raid_pattern_score_threshold);

        // ── 0. Enregistrer le join pour l'analyse de pattern ──
        let join_info = JoinInfo {
            username: user.name.clone(),
            has_avatar: user.avatar.is_some(),
            account_created_timestamp: user.created_at().unix_timestamp(),
        };
        recent_joins.record(guild_id, join_info);

        // ── 1. Detection anti-raid ──
        let is_raid = raid_detector.record_join(guild_id);

        // Analyse de pattern avancee (meme si le seuil de joins n'est pas atteint)
        let pattern_raid = if raid_pattern_enabled {
            let joins = recent_joins.recent(guild_id);
            if joins.len() >= 3 {
                let analysis = raid_analyzer::analyze_joins(&joins, 2, 3600);
                analysis.score >= raid_pattern_score_threshold
            } else {
                false
            }
        } else {
            false
        };

        let is_raid = is_raid || pattern_raid;

        if is_raid {
            let join_count = raid_detector.recent_joins(guild_id);

            warn!(
                guild_id = %guild_id,
                joins = join_count,
                "RAID DETECTE — activation lockdown"
            );

            // Signaler au backend
            let event = SecurityEvent {
                guild_id: guild_id.to_string(),
                event_type: "raid_detected".to_string(),
                severity: "critical".to_string(),
                description: format!(
                    "Raid detecte : {} joins en quelques secondes. Actions: lockdown{}{}",
                    join_count,
                    if slowmode_secs > 0 { ", slowmode auto" } else { "" },
                    if quarantine_enabled { ", quarantaine" } else { "" },
                ),
                user_ids: vec![user.id.to_string()],
            };

            if let Err(e) = sec_api.report_event(&event).await {
                error!(error = %e, "Erreur envoi evenement raid au backend");
            }

            // Activer le mode verification du serveur (highest)
            if let Ok(mut guild) = guild_id.to_partial_guild(&ctx.http).await {
                let edit = serenity::builder::EditGuild::new()
                    .verification_level(serenity::model::guild::VerificationLevel::Higher);

                if let Err(e) = guild.edit(&ctx.http, edit).await {
                    error!(error = %e, "Impossible d'activer le lockdown");
                } else {
                    info!(guild_id = %guild_id, "Lockdown active (verification: Highest)");
                }
            }

            // ── Slowmode auto ──
            if slowmode_secs > 0 {
                slowmode.activate(&ctx, guild_id, slowmode_secs).await;
            }

            // ── Lockdown auto ──
            if lockdown_enabled {
                lockdown.activate(&ctx, guild_id).await;
            }

            // ── Quarantaine sur le membre qui a declenche ──
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

                        send_captcha(&ctx, user.id, guild_id, &guild_name, &captcha_type, captcha_pending).await;
                    }
                }
            }

            // Envoyer une alerte dans le premier salon texte trouve
            match guild_id.channels(&ctx.http).await {
            Err(e) => tracing::warn!(guild=%guild_id, error=%e, "Impossible de charger les channels pour l'alerte raid"),
            Ok(channels) => {
                if let Some(channel) = channels
                    .values()
                    .find(|c| c.kind == serenity::model::channel::ChannelType::Text)
                {
                    let mut actions = String::from("Niveau de verification augmente automatiquement.");
                    if slowmode_secs > 0 {
                        actions.push_str(&format!(
                            "\nSlowmode active ({}s) sur tous les salons.",
                            slowmode_secs
                        ));
                    }
                    if lockdown_enabled {
                        actions.push_str("\nLockdown active — envoi de messages desactive.");
                    }
                    if quarantine_enabled {
                        actions.push_str("\nNouveaux membres mis en quarantaine.");
                    }

                    let embed = danger_embed("\u{1f6a8} ALERTE RAID DETECTE")
                        .field("\u{1f465} Joins rapides", join_count.to_string(), true)
                        .field("\u{26a1} Actions", actions, false);

                    if let Err(e) = channel
                        .send_message(
                            &ctx.http,
                            serenity::builder::CreateMessage::new().embed(embed),
                        )
                        .await
                    {
                        warn!(error = %e, "Failed to send raid alert embed");
                    }
                }
            }}

            raid_detector.reset(guild_id);
            recent_joins.reset(guild_id);
        }

        // ── 2. Verification compte suspect ──
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
                "Compte suspect detecte (trop recent)"
            );

            let mut description = format!(
                "Compte suspect : {} (cree il y a {}h)",
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

                            send_captcha(&ctx, user.id, guild_id, &guild_name, &captcha_type, captcha_pending).await;
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

            if let Err(e) = sec_api.report_event(&event).await {
                error!(error = %e, "Erreur envoi evenement compte suspect");
            }
        }

        // ── 3. Detection alt account ──
        if alt_detection_enabled {
            let alt_analysis = alt_detector.check_user(
                guild_id,
                &user.name,
                user.created_at().unix_timestamp(),
            );

            if alt_analysis.is_suspicious() {
                let mut description = format!("Alt account suspecte : {}", user.name);
                if let Some(ref banned) = alt_analysis.similar_to_banned {
                    description.push_str(&format!(" (nom similaire a {})", banned));
                }
                if let Some(ref banned) = alt_analysis.creation_near_banned {
                    description.push_str(&format!(" (creation proche de {})", banned));
                }

                warn!(
                    guild_id = %guild_id,
                    user = %user.name,
                    similar_to = ?alt_analysis.similar_to_banned,
                    creation_near = ?alt_analysis.creation_near_banned,
                    "Alt account suspecte"
                );

                // Quarantaine si active
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

                                send_captcha(&ctx, user.id, guild_id, &guild_name, &captcha_type, captcha_pending).await;
                            }
                        }
                    }
                }

                let event = SecurityEvent {
                    guild_id: guild_id.to_string(),
                    event_type: "alt_account_suspected".to_string(),
                    severity: "warning".to_string(),
                    description,
                    user_ids: vec![user.id.to_string()],
                };

                if let Err(e) = sec_api.report_event(&event).await {
                    error!(error = %e, "Erreur envoi evenement alt account");
                }
            }
        }
    }

    /// Declenche quand un membre quitte le serveur.
    async fn guild_member_removal(
        &self,
        ctx: Context,
        guild_id: GuildId,
        user: serenity::model::user::User,
        _member: Option<Member>,
    ) {
        info!(guild_id = %guild_id, user = %user.name, "Membre parti");

        let data = ctx.data.read().await;

        // Supprimer le membre de la BDD
        if let Some(sec_api) = data.get::<SecurityApiKey>() {
            if let Err(e) = sec_api.remove_member(&guild_id.to_string(), &user.id.to_string()).await {
                warn!(error = %e, "Erreur remove_member");
            }
        }

        if let Some(base) = data.get::<ApiClientKey>() {
            base.send_log(
                "info",
                &guild_id.to_string(),
                &format!("Membre parti : {} ({})", user.name, user.id),
            );
        }
    }

    /// Declenche quand un membre est mis a jour (pseudo, roles, avatar).
    async fn guild_member_update(&self, ctx: Context, _old: Option<Member>, new_member: Option<Member>, _event: serenity::model::event::GuildMemberUpdateEvent) {
        let member = match new_member {
            Some(m) => m,
            None => return,
        };
        let guild_id = member.guild_id;
        let user = &member.user;

        let data = ctx.data.read().await;
        if let Some(sec_api) = data.get::<SecurityApiKey>() {
            let roles: Vec<String> = member.roles.iter().map(|r| r.to_string()).collect();
            let payload = UpdateMemberPayload {
                username: Some(user.name.clone()),
                display_name: member.nick.clone(),
                avatar: user.avatar.as_ref().map(|a| a.to_string()),
                roles: Some(serde_json::json!(roles)),
            };
            if let Err(e) = sec_api.update_member(&guild_id.to_string(), &user.id.to_string(), &payload).await {
                warn!(error = %e, "Erreur update_member");
            }
        }
    }

    /// Declenche quand un salon est cree.
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

        info!(guild_id = %guild_id, channel = %channel.name, kind, "Salon cree");

        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            base.send_log(
                "info",
                &guild_id.to_string(),
                &format!("Salon {} cree : {} ({})", kind, channel.name, channel.id),
            );
        }
    }

    /// Declenche quand un salon est supprime.
    async fn channel_delete(&self, ctx: Context, channel: GuildChannel, _messages: Option<Vec<serenity::model::channel::Message>>) {
        let guild_id = channel.guild_id;
        let kind = match channel.kind {
            serenity::model::channel::ChannelType::Text => "texte",
            serenity::model::channel::ChannelType::Voice => "vocal",
            serenity::model::channel::ChannelType::Category => "categorie",
            _ => "autre",
        };

        info!(guild_id = %guild_id, channel = %channel.name, kind, "Salon supprime");

        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            base.send_log(
                "warn",
                &guild_id.to_string(),
                &format!("Salon {} supprime : {}", kind, channel.name),
            );
        }
    }

    /// Declenche quand un membre est banni.
    async fn guild_ban_addition(&self, ctx: Context, guild_id: GuildId, banned_user: serenity::model::user::User) {
        info!(guild_id = %guild_id, user = %banned_user.name, "Membre banni");

        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            base.send_log(
                "warn",
                &guild_id.to_string(),
                &format!("Membre banni : {} ({})", banned_user.name, banned_user.id),
            );
        }

        // Enregistrer le ban pour la detection d'alt accounts
        if let Some(alt_detector) = data.get::<AltDetectorKey>() {
            alt_detector.record_ban(
                guild_id,
                banned_user.name.clone(),
                banned_user.created_at().unix_timestamp(),
            );
        }
    }

    /// Declenche quand un membre est debanni.
    async fn guild_ban_removal(&self, ctx: Context, guild_id: GuildId, unbanned_user: serenity::model::user::User) {
        info!(guild_id = %guild_id, user = %unbanned_user.name, "Membre debanni");

        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            base.send_log(
                "info",
                &guild_id.to_string(),
                &format!("Membre debanni : {} ({})", unbanned_user.name, unbanned_user.id),
            );
        }
    }

    /// Gere les interactions (slash commands + bouton captcha + captcha math).
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(command) => {
                if let Some(guild_id) = command.guild_id {
                    let data = ctx.data.read().await;
                    if let Some(api) = data.get::<ApiClientKey>() {
                        if !sentinel_shared::discord_helpers::is_bot_enabled(api, &guild_id.to_string()).await {
                            return;
                        }
                    }
                }

                if command.data.name.as_str() == "security" {
                    commands::security::handle(&ctx, &command).await;
                }
                return;
            }
            Interaction::Component(component) => {
            let custom_id = &component.data.custom_id;
            let is_button_captcha = custom_id == captcha::CAPTCHA_BUTTON_ID;
            let is_math_captcha = custom_id.starts_with(captcha::CAPTCHA_MATH_PREFIX);

            if !is_button_captcha && !is_math_captcha {
                return;
            }

            let user_id = component.user.id;

            let data = ctx.data.read().await;
            let (quarantine, env_config, base, sec_api) =
                match (
                    data.get::<QuarantineKey>(),
                    data.get::<ConfigKey>(),
                    data.get::<ApiClientKey>(),
                    data.get::<SecurityApiKey>(),
                ) {
                    (Some(q), Some(c), Some(a), Some(s)) => (q, c, a, s),
                    _ => {
                        error!("TypeMap incomplete pour interaction captcha");
                        return;
                    }
                };

            // ── Captcha math : verifier la reponse ──
            if is_math_captcha {
                let captcha_pending = match data.get::<CaptchaPendingKey>() {
                    Some(p) => p,
                    None => {
                        error!("CaptchaPendingKey manquant");
                        return;
                    }
                };

                // Extraire et valider l'index du bouton presse
                let pressed_str = custom_id.strip_prefix(captcha::CAPTCHA_MATH_PREFIX).unwrap_or("");
                let pressed_index: usize = match pressed_str.parse::<usize>() {
                    Ok(i) if i < 4 => i,
                    _ => {
                        tracing::warn!(user=%user_id, index=%pressed_str, "Index captcha invalide");
                        return;
                    }
                };

                // Trouver le guild_id de l'utilisateur en quarantaine
                let mut target_guild = None;
                for gid in ctx.cache.guilds() {
                    if quarantine.is_quarantined(gid, user_id) {
                        target_guild = Some(gid);
                        break;
                    }
                }

                let guild_id = match target_guild {
                    Some(g) => g,
                    None => {
                        let embed = warn_embed("\u{26a0}\u{fe0f} Deja verifie")
                            .description("Vous n'etes pas en quarantaine.");
                        let response = serenity::builder::CreateInteractionResponse::Message(
                            serenity::builder::CreateInteractionResponseMessage::new()
                                .embed(embed)
                                .ephemeral(true),
                        );
                        if let Err(e) = component.create_response(&ctx.http, response).await {
                            warn!(error = %e, "Failed to send already-verified response");
                        }
                        return;
                    }
                };

                match captcha_pending.verify(guild_id, user_id, pressed_index) {
                    Some(true) => {
                        // Bonne reponse — liberer
                        captcha_pending.remove(guild_id, user_id);

                        let guild_config = match base.get_guild_config(&guild_id.to_string()).await {
                            Ok(cfg) => cfg,
                            Err(e) => {
                                tracing::warn!(error = %e, guild_id = %guild_id, "Echec chargement config guild");
                                std::collections::HashMap::new()
                            }
                        };
                        let role_id = guild_config
                            .get("quarantine_role_id")
                            .and_then(|v| v.parse::<u64>().ok())
                            .or(env_config.quarantine_role_id);

                        if let Some(role_id) = role_id {
                            quarantine.release_user(&ctx, guild_id, user_id, RoleId::new(role_id)).await;
                        }

                        let event = SecurityEvent {
                            guild_id: guild_id.to_string(),
                            event_type: "captcha_verified".to_string(),
                            severity: "info".to_string(),
                            description: format!("Utilisateur {} a passe le captcha math", component.user.name),
                            user_ids: vec![user_id.to_string()],
                        };
                        if let Err(e) = sec_api.report_event(&event).await {
                            warn!(error = %e, "Failed to report captcha_verified event");
                        }

                        let embed = success_embed("\u{2705} Verification reussie")
                            .description("Bonne reponse ! Vous avez maintenant acces au serveur.");
                        let response = serenity::builder::CreateInteractionResponse::Message(
                            serenity::builder::CreateInteractionResponseMessage::new()
                                .embed(embed)
                                .ephemeral(true),
                        );
                        if let Err(e) = component.create_response(&ctx.http, response).await {
                            warn!(error = %e, "Failed to send captcha success response");
                        }
                    }
                    Some(false) => {
                        // Mauvaise reponse — log pour detection brute-force
                        tracing::warn!(guild=%guild_id, user=%user_id, index=%pressed_index, "Echec captcha math");
                        let embed = danger_embed("\u{274c} Mauvaise reponse")
                            .description("Ce n'est pas la bonne reponse. Reessayez.");
                        let response = serenity::builder::CreateInteractionResponse::Message(
                            serenity::builder::CreateInteractionResponseMessage::new()
                                .embed(embed)
                                .ephemeral(true),
                        );
                        if let Err(e) = component.create_response(&ctx.http, response).await {
                            warn!(error = %e, "Failed to send captcha failure response");
                        }
                    }
                    None => {
                        let embed = warn_embed("\u{26a0}\u{fe0f} Captcha expire")
                            .description("Ce captcha n'est plus valide.");
                        let response = serenity::builder::CreateInteractionResponse::Message(
                            serenity::builder::CreateInteractionResponseMessage::new()
                                .embed(embed)
                                .ephemeral(true),
                        );
                        if let Err(e) = component.create_response(&ctx.http, response).await {
                            warn!(error = %e, "Failed to send captcha expired response");
                        }
                    }
                }
                return;
            }

            // ── Captcha bouton classique ──
            let mut released = false;

            {
                let cache = &ctx.cache;
                for guild_id in cache.guilds() {
                    if !quarantine.is_quarantined(guild_id, user_id) {
                        continue;
                    }

                    let guild_config = base
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
                                "Utilisateur {} a passe le captcha",
                                component.user.name
                            ),
                            user_ids: vec![user_id.to_string()],
                        };
                        if let Err(e) = sec_api.report_event(&event).await {
                            warn!(error = %e, "Failed to report captcha_verified event");
                        }

                        released = true;
                    }
                }
            }

            let embed = if released {
                success_embed("\u{2705} Verification reussie")
                    .description("Vous avez maintenant acces au serveur.")
            } else {
                warn_embed("\u{26a0}\u{fe0f} Deja verifie")
                    .description("Vous n'etes pas en quarantaine ou la verification a deja ete effectuee.")
            };

            let response = serenity::builder::CreateInteractionResponse::Message(
                serenity::builder::CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .ephemeral(true),
            );

            if let Err(e) = component.create_response(&ctx.http, response).await {
                warn!(error = %e, "Failed to send captcha final response");
            }
            }
            _ => {}
        }
    }
}

/// Envoie le captcha adapte selon le type configure.
async fn send_captcha(
    ctx: &Context,
    user_id: UserId,
    guild_id: GuildId,
    guild_name: &str,
    captcha_type: &str,
    captcha_pending: &CaptchaPending,
) {
    match captcha_type {
        "math" => {
            captcha::send_math_challenge(ctx, user_id, guild_id, guild_name, captcha_pending).await;
        }
        _ => {
            captcha::send_challenge(ctx, user_id, guild_name).await;
        }
    }
}
