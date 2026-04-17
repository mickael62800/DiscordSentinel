//! Module security — anti-raid, verification utilisateurs, protection serveur.
//! Migre depuis security-bot.

pub mod api_client;
mod commands;
pub mod detectors;

use chrono::DateTime;
use serenity::all::{CommandInteraction, ComponentInteraction, Context, CreateCommand};
use serenity::model::guild::Member;
use serenity::model::id::{GuildId, RoleId, UserId};
use serenity::prelude::*;
use tracing::{error, info, warn};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::embeds::{danger_embed, success_embed, warn_embed};
use sentinel_shared::heartbeat::ApiClientKey;

use api_client::{ApiClient, MemberPayload, RecentJoinEntry, SecurityEvent, SyncMembersPayload, UpdateMemberPayload};
use detectors::account_checker::AccountChecker;
use detectors::alt_detector::AltDetector;
use detectors::captcha::{self, CaptchaPending};
use detectors::lockdown::LockdownManager;
use detectors::quarantine::QuarantineManager;
use detectors::raid_analyzer::{JoinInfo, RecentJoinsTracker};
use detectors::raid_detector::RaidDetector;
use detectors::slowmode::SlowmodeManager;

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

pub struct SecurityConfigKey;
impl TypeMapKey for SecurityConfigKey {
    type Value = SecurityConfig;
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

// ── Security config (loaded from env, stored in TypeMap) ──

use sentinel_shared::config::{load_env, load_env_bool, load_env_optional, load_env_string};

#[derive(Clone)]
pub struct SecurityConfig {
    pub raid_join_threshold: u64,
    pub raid_join_window_secs: u64,
    pub min_account_age_secs: u64,
    pub quarantine_role_id: Option<u64>,
    pub quarantine_enabled: bool,
    pub slowmode_seconds: u16,
    pub slowmode_duration_secs: u64,
    pub captcha_enabled: bool,
    pub captcha_timeout_secs: u64,
    pub captcha_type: String,
    pub lockdown_enabled: bool,
    pub lockdown_duration_secs: u64,
    pub alt_detection_enabled: bool,
    pub alt_retention_secs: u64,
    pub alt_name_distance: usize,
    pub raid_pattern_enabled: bool,
    pub raid_pattern_score_threshold: u32,
}

impl SecurityConfig {
    pub fn from_env() -> Self {
        Self {
            raid_join_threshold: load_env("RAID_JOIN_THRESHOLD", 10),
            raid_join_window_secs: load_env("RAID_JOIN_WINDOW_SECS", 10),
            min_account_age_secs: load_env("MIN_ACCOUNT_AGE_SECS", 86400),
            quarantine_role_id: load_env_optional("QUARANTINE_ROLE_ID"),
            quarantine_enabled: load_env_bool("QUARANTINE_ENABLED", false),
            slowmode_seconds: load_env("SLOWMODE_SECONDS", 10),
            slowmode_duration_secs: load_env("SLOWMODE_DURATION_SECS", 300),
            captcha_enabled: load_env_bool("CAPTCHA_ENABLED", false),
            captcha_timeout_secs: load_env("CAPTCHA_TIMEOUT_SECS", 300),
            captcha_type: {
                let ct = load_env_string("CAPTCHA_TYPE", "button");
                if ct != "button" && ct != "math" {
                    tracing::warn!(value=%ct, "CAPTCHA_TYPE invalide, utilisation de 'button' par defaut");
                    "button".to_string()
                } else {
                    ct
                }
            },
            lockdown_enabled: load_env_bool("LOCKDOWN_ENABLED", false),
            lockdown_duration_secs: load_env("LOCKDOWN_DURATION_SECS", 300),
            alt_detection_enabled: load_env_bool("ALT_DETECTION_ENABLED", false),
            alt_retention_secs: load_env("ALT_RETENTION_SECS", 604_800),
            alt_name_distance: load_env("ALT_NAME_DISTANCE", 2),
            raid_pattern_enabled: load_env_bool("RAID_PATTERN_ENABLED", true),
            raid_pattern_score_threshold: load_env("RAID_PATTERN_SCORE_THRESHOLD", 60),
        }
    }
}

// ── Slash commands ──

pub fn register_commands() -> Vec<CreateCommand> {
    vec![commands::register()]
}

pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    commands::handle(ctx, command).await;
}

// ── Component interaction routing ──

/// Retourne true si ce custom_id est gere par le module security.
pub fn handles_component(cid: &str) -> bool {
    cid == captcha::CAPTCHA_BUTTON_ID || cid.starts_with(captcha::CAPTCHA_MATH_PREFIX)
}

// ── Event handlers (appelees depuis handler.rs) ──

/// Spawn les background tasks security : captcha timeout / slowmode revert / lockdown revert.
pub fn spawn_background(ctx: Context) {
    // 1. Captcha timeout + quarantine kick (30s loop)
    let ctx_q = ctx.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;

            let data = ctx_q.data.read().await;
            let Some(quarantine) = data.get::<QuarantineKey>() else { continue };
            let captcha_timeout = data
                .get::<SecurityConfigKey>()
                .map(|c| c.captcha_timeout_secs)
                .unwrap_or(300);

            if let Some(cp) = data.get::<CaptchaPendingKey>() {
                cp.cleanup_expired();
            }

            let expired = quarantine.expired_users(captcha_timeout);
            for (guild_id, user_id) in expired {
                if let Err(e) = guild_id.kick(&ctx_q.http, user_id).await {
                    tracing::warn!(
                        error = %e,
                        guild_id = %guild_id,
                        user_id = %user_id,
                        "Impossible de kick l'utilisateur (captcha timeout)"
                    );
                } else {
                    tracing::info!(
                        guild_id = %guild_id,
                        user_id = %user_id,
                        "Utilisateur kick (captcha timeout)"
                    );
                }
                quarantine.remove_tracking(guild_id, user_id);
                if let Some(cp) = data.get::<CaptchaPendingKey>() {
                    cp.remove(guild_id, user_id);
                }
            }
        }
    });

    // 2. Slowmode revert (15s loop)
    let ctx_s = ctx.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;

            let data = ctx_s.data.read().await;
            let Some(slowmode) = data.get::<SlowmodeKey>() else { continue };
            let duration = data
                .get::<SecurityConfigKey>()
                .map(|c| c.slowmode_duration_secs)
                .unwrap_or(300);

            let expired = slowmode.expired_guilds(duration);
            for guild_id in expired {
                slowmode.deactivate_with_http(&ctx_s.http, guild_id).await;
            }
        }
    });

    // 3. Lockdown revert (15s loop)
    let ctx_l = ctx;
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;

            let data = ctx_l.data.read().await;
            let Some(lockdown) = data.get::<LockdownKey>() else { continue };
            let duration = data
                .get::<SecurityConfigKey>()
                .map(|c| c.lockdown_duration_secs)
                .unwrap_or(600);

            let expired = lockdown.expired_guilds(duration);
            for guild_id in expired {
                lockdown.deactivate_with_http(&ctx_l.http, guild_id).await;
            }
        }
    });
}

/// Sync tous les membres au demarrage (appelee depuis ready).
pub async fn on_ready_sync(ctx: &Context, guilds: &[serenity::model::guild::UnavailableGuild]) {
    let data = ctx.data.read().await;
    let sec_api = match data.get::<SecurityApiKey>() {
        Some(a) => a,
        None => {
            error!("SecurityApiKey manquant pour sync membres");
            return;
        }
    };

    for guild in guilds {
        let guild_id = guild.id;
        match guild_id.members(&ctx.http, None, None).await {
            Ok(members) => {
                let payloads: Vec<MemberPayload> = members
                    .iter()
                    .map(|m| {
                        let roles: Vec<String> =
                            m.roles.iter().map(|r| r.to_string()).collect();
                        MemberPayload {
                            guild_id: guild_id.to_string(),
                            user_id: m.user.id.to_string(),
                            username: m.user.name.clone(),
                            display_name: m.nick.clone(),
                            avatar: m.user.avatar.as_ref().map(|a| a.to_string()),
                            roles: serde_json::json!(roles),
                            joined_at: m
                                .joined_at
                                .and_then(|t| DateTime::from_timestamp(t.unix_timestamp(), 0)),
                            account_created: Some(DateTime::from_timestamp(
                                m.user.created_at().unix_timestamp(),
                                0,
                            ))
                            .flatten(),
                            is_bot: m.user.bot,
                            last_seen_at: None,
                        }
                    })
                    .collect();

                let count = payloads.len();
                let payload = SyncMembersPayload {
                    guild_id: guild_id.to_string(),
                    members: payloads,
                };

                match sec_api.sync_members(&payload).await {
                    Ok(()) => info!(guild_id = %guild_id, members = count, "Membres synchronises"),
                    Err(e) => {
                        error!(guild_id = %guild_id, error = %e, "Erreur sync membres")
                    }
                }
            }
            Err(e) => {
                error!(guild_id = %guild_id, error = %e, "Impossible de recuperer les membres")
            }
        }
    }
}

/// Declenche a chaque nouveau membre qui rejoint un serveur.
pub async fn on_member_add(ctx: &Context, new_member: &Member) {
    let guild_id = new_member.guild_id;
    let user = &new_member.user;

    info!(
        guild_id = %guild_id,
        user = %user.name,
        user_id = %user.id,
        "Nouveau membre (security)"
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
            joined_at: new_member
                .joined_at
                .and_then(|t| DateTime::from_timestamp(t.unix_timestamp(), 0)),
            account_created: Some(DateTime::from_timestamp(
                user.created_at().unix_timestamp(),
                0,
            ))
            .flatten(),
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
        None => {
            error!(guild_id = %guild_id, "ApiClientKey manquant");
            return;
        }
    };
    let sec_api = match data.get::<SecurityApiKey>() {
        Some(a) => a,
        None => {
            error!(guild_id = %guild_id, "SecurityApiKey manquant");
            return;
        }
    };
    let raid_detector = match data.get::<RaidDetectorKey>() {
        Some(a) => a,
        None => {
            error!(guild_id = %guild_id, "RaidDetectorKey manquant");
            return;
        }
    };
    let _account_checker = match data.get::<AccountCheckerKey>() {
        Some(a) => a,
        None => {
            error!(guild_id = %guild_id, "AccountCheckerKey manquant");
            return;
        }
    };
    let env_config = match data.get::<SecurityConfigKey>() {
        Some(a) => a,
        None => {
            error!(guild_id = %guild_id, "SecurityConfigKey manquant");
            return;
        }
    };
    let quarantine = match data.get::<QuarantineKey>() {
        Some(a) => a,
        None => {
            error!(guild_id = %guild_id, "QuarantineKey manquant");
            return;
        }
    };
    let slowmode = match data.get::<SlowmodeKey>() {
        Some(a) => a,
        None => {
            error!(guild_id = %guild_id, "SlowmodeKey manquant");
            return;
        }
    };
    let lockdown = match data.get::<LockdownKey>() {
        Some(a) => a,
        None => {
            error!(guild_id = %guild_id, "LockdownKey manquant");
            return;
        }
    };
    let recent_joins = match data.get::<RecentJoinsKey>() {
        Some(a) => a,
        None => {
            error!(guild_id = %guild_id, "RecentJoinsKey manquant");
            return;
        }
    };
    let captcha_pending = match data.get::<CaptchaPendingKey>() {
        Some(a) => a,
        None => {
            error!(guild_id = %guild_id, "CaptchaPendingKey manquant");
            return;
        }
    };
    let _alt_detector = match data.get::<AltDetectorKey>() {
        Some(a) => a,
        None => {
            error!(guild_id = %guild_id, "AltDetectorKey manquant");
            return;
        }
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

    let _min_account_age =
        BaseApiClient::config_u64(&guild_config, "min_account_age_secs", env_config.min_account_age_secs);

    // Config quarantaine per-guild
    let _quarantine_enabled = guild_config
        .get("quarantine_enabled")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(env_config.quarantine_enabled);
    let quarantine_role_id = guild_config
        .get("quarantine_role_id")
        .and_then(|v| {
            v.parse::<u64>()
                .map_err(|_| {
                    tracing::warn!(guild=%guild_id, value=%v, "quarantine_role_id invalide dans la config guild");
                })
                .ok()
        })
        .or(env_config.quarantine_role_id);
    let _captcha_enabled = guild_config
        .get("captcha_enabled")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(env_config.captcha_enabled);
    let _slowmode_secs: u16 = guild_config
        .get("slowmode_seconds")
        .and_then(|v| v.parse().ok())
        .unwrap_or(env_config.slowmode_seconds);
    let _lockdown_enabled = guild_config
        .get("lockdown_enabled")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(env_config.lockdown_enabled);
    let captcha_type = guild_config
        .get("captcha_type")
        .cloned()
        .unwrap_or_else(|| env_config.captcha_type.clone());
    let _alt_detection_enabled = guild_config
        .get("alt_detection_enabled")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(env_config.alt_detection_enabled);
    let _raid_pattern_enabled = guild_config
        .get("raid_pattern_enabled")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(env_config.raid_pattern_enabled);
    let _raid_pattern_score_threshold = guild_config
        .get("raid_pattern_score_threshold")
        .and_then(|v| v.parse().ok())
        .unwrap_or(env_config.raid_pattern_score_threshold);

    // ── 0. Buffer temporel des joins (le bot garde le timing, pas le metier) ──
    let join_info = JoinInfo {
        username: user.name.clone(),
        has_avatar: user.avatar.is_some(),
        account_created_timestamp: user.created_at().unix_timestamp(),
    };
    recent_joins.record(guild_id, join_info);

    // Simple detection seuil de joins rapides (buffer local).
    let simple_raid = raid_detector.record_join(guild_id);

    // ── 1. Appel API : l'API decide de tout ──
    let recent = recent_joins.recent(guild_id);
    let recent_entries: Vec<RecentJoinEntry> = recent
        .iter()
        .map(|j| RecentJoinEntry {
            username: j.username.clone(),
            has_avatar: j.has_avatar,
            account_created_timestamp: j.account_created_timestamp,
        })
        .collect();

    let decision = match sec_api
        .analyze_new_member(
            &guild_id.to_string(),
            &user.id.to_string(),
            &user.name,
            user.avatar.is_some(),
            user.created_at().unix_timestamp(),
            user.bot,
            recent_entries,
        )
        .await
    {
        Ok(d) => d,
        Err(e) => {
            error!(error = %e, "Erreur API analyze_new_member");
            return;
        }
    };

    let is_raid = simple_raid || decision.is_raid;

    // ── 2. Executer les decisions de l'API ──

    if is_raid {
        warn!(guild_id = %guild_id, score = decision.raid_score, "RAID DETECTE");

        if decision.activate_lockdown {
            if let Ok(mut guild) = guild_id.to_partial_guild(&ctx.http).await {
                let edit = serenity::builder::EditGuild::new()
                    .verification_level(serenity::model::guild::VerificationLevel::Higher);
                if let Err(e) = guild.edit(&ctx.http, edit).await {
                    error!(error = %e, "Impossible d'activer le lockdown");
                }
            }
            lockdown.activate(ctx, guild_id).await;
        }

        if decision.slowmode_secs > 0 {
            slowmode
                .activate(ctx, guild_id, decision.slowmode_secs as u16)
                .await;
        }

        raid_detector.reset(guild_id);
        recent_joins.reset(guild_id);
    }

    // Quarantaine + captcha (decision API).
    if decision.quarantine {
        if let Some(role_id) = quarantine_role_id {
            quarantine
                .quarantine_user(ctx, guild_id, user.id, RoleId::new(role_id))
                .await;

            if decision.send_captcha {
                let guild_name = guild_id
                    .to_partial_guild(&ctx.http)
                    .await
                    .map(|g| g.name.clone())
                    .unwrap_or_else(|_| "Serveur".to_string());
                send_captcha(
                    ctx,
                    user.id,
                    guild_id,
                    &guild_name,
                    &captcha_type,
                    captcha_pending,
                )
                .await;
            }
        }
    }

    // Log si event detecte.
    if !decision.event_type.is_empty() {
        info!(
            guild_id = %guild_id,
            event = %decision.event_type,
            desc = %decision.event_description,
            raid = decision.is_raid,
            suspicious = decision.is_suspicious_account,
            alt = decision.is_alt_account,
            "Security decision appliquee"
        );
    }
}

/// Declenche quand un membre quitte le serveur.
pub async fn on_member_remove(
    ctx: &Context,
    guild_id: GuildId,
    user: &serenity::model::user::User,
) {
    info!(guild_id = %guild_id, user = %user.name, "Membre parti (security)");

    let data = ctx.data.read().await;

    // Supprimer le membre de la BDD
    if let Some(sec_api) = data.get::<SecurityApiKey>() {
        if let Err(e) = sec_api
            .remove_member(&guild_id.to_string(), &user.id.to_string())
            .await
        {
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
pub async fn on_member_update(ctx: &Context, member: &Member) {
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
        if let Err(e) = sec_api
            .update_member(&guild_id.to_string(), &user.id.to_string(), &payload)
            .await
        {
            warn!(error = %e, "Erreur update_member");
        }
    }
}

/// Declenche quand un membre est banni.
pub async fn on_ban_add(
    ctx: &Context,
    guild_id: GuildId,
    banned_user: &serenity::model::user::User,
) {
    info!(guild_id = %guild_id, user = %banned_user.name, "Membre banni (security)");

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
pub async fn on_ban_remove(
    ctx: &Context,
    guild_id: GuildId,
    unbanned_user: &serenity::model::user::User,
) {
    info!(guild_id = %guild_id, user = %unbanned_user.name, "Membre debanni (security)");

    let data = ctx.data.read().await;
    if let Some(base) = data.get::<ApiClientKey>() {
        base.send_log(
            "info",
            &guild_id.to_string(),
            &format!(
                "Membre debanni : {} ({})",
                unbanned_user.name, unbanned_user.id
            ),
        );
    }
}

/// Gere les interactions captcha (bouton + math).
pub async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    let custom_id = &component.data.custom_id;
    let is_button_captcha = custom_id == captcha::CAPTCHA_BUTTON_ID;
    let is_math_captcha = custom_id.starts_with(captcha::CAPTCHA_MATH_PREFIX);

    if !is_button_captcha && !is_math_captcha {
        return;
    }

    let user_id = component.user.id;

    let data = ctx.data.read().await;
    let (quarantine, env_config, base, sec_api) = match (
        data.get::<QuarantineKey>(),
        data.get::<SecurityConfigKey>(),
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
        let pressed_str = custom_id
            .strip_prefix(captcha::CAPTCHA_MATH_PREFIX)
            .unwrap_or("");
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
                let embed =
                    warn_embed("\u{26a0}\u{fe0f} Deja verifie").description("Vous n'etes pas en quarantaine.");
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
                    quarantine
                        .release_user(ctx, guild_id, user_id, RoleId::new(role_id))
                        .await;
                }

                let event = SecurityEvent {
                    guild_id: guild_id.to_string(),
                    event_type: "captcha_verified".to_string(),
                    severity: "info".to_string(),
                    description: format!(
                        "Utilisateur {} a passe le captcha math",
                        component.user.name
                    ),
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

            let guild_config = match base.get_guild_config(&guild_id.to_string()).await {
                Ok(cfg) => cfg,
                Err(e) => {
                    tracing::warn!(error = %e, guild_id = %guild_id, "Echec chargement config guild (captcha bouton)");
                    std::collections::HashMap::new()
                }
            };
            let role_id = guild_config
                .get("quarantine_role_id")
                .and_then(|v| v.parse::<u64>().ok())
                .or(env_config.quarantine_role_id);

            if let Some(role_id) = role_id {
                quarantine
                    .release_user(ctx, guild_id, user_id, RoleId::new(role_id))
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
