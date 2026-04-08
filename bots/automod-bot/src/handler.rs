use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;
use serenity::async_trait;
use serenity::model::application::Interaction;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::{ChannelId, MessageId, UserId};
use serenity::prelude::*;
use tracing::{error, info, warn};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::heartbeat::{ApiClientKey, register_guilds};

use crate::adaptive_slowmode::SlowmodeTracker;
use crate::api_client::{Action, AnalyzeRequest, ApiClient, MessageMetadata};
use crate::commands;
use crate::detectors::{self, DetectorConfig};

use sentinel_shared::embeds::{warn_embed, moderate_embed, danger_embed, critical_embed};

/// Deduplication des messages deja traites (avec timestamp pour cleanup)
pub struct ProcessedMessagesKey;

impl TypeMapKey for ProcessedMessagesKey {
    type Value = Arc<DashMap<MessageId, Instant>>;
}

/// Flood tracker : (channel_id, user_id) -> liste de timestamps
pub struct FloodTrackerKey;

impl TypeMapKey for FloodTrackerKey {
    type Value = Arc<DashMap<(ChannelId, UserId), Vec<Instant>>>;
}

/// Couleurs des embeds lues depuis la config guild.
struct EmbedColors {
    warn: u32,
    delete: u32,
    mute: u32,
    ban: u32,
}

pub struct SlowmodeTrackerKey;
impl TypeMapKey for SlowmodeTrackerKey {
    type Value = SlowmodeTracker;
}

/// Defaults si l'API ne repond pas
const DEFAULT_FLOOD_MAX_MESSAGES: u64 = 5;
const DEFAULT_FLOOD_WINDOW_SECS: u64 = 10;
const DEFAULT_MUTE_DURATION_SECS: u64 = 600;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        // Ignorer les messages de bots
        if msg.author.bot {
            return;
        }

        // Deduplication : ignorer si deja traite
        {
            let data = ctx.data.read().await;
            if let Some(processed) = data.get::<ProcessedMessagesKey>() {
                let now = Instant::now();
                if processed.contains_key(&msg.id) {
                    return;
                }
                processed.insert(msg.id, now);
                // Cleanup : supprimer les entrees > 5 minutes
                if processed.len() > 2000 {
                    processed.retain(|_, ts| now.duration_since(*ts).as_secs() < 300);
                }
            }
        }

        // Charger la config depuis l'API pour ce guild (une seule fois par message)
        let guild_id = msg.guild_id.map(|id| id.to_string()).unwrap_or_default();
        let config = {
            let data = ctx.data.read().await;
            if let Some(api) = data.get::<ApiClientKey>() {
                match api.get_guild_config(&guild_id).await {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        warn!(guild_id = %guild_id, error = %e, "Impossible de charger la config guild, utilisation des valeurs par defaut");
                        std::collections::HashMap::new()
                    }
                }
            } else {
                std::collections::HashMap::new()
            }
        };

        if !BaseApiClient::config_bool(&config, "enabled", true) {
            return;
        }

        let flood_max_messages = BaseApiClient::config_u64(&config, "flood_max_messages", DEFAULT_FLOOD_MAX_MESSAGES) as usize;
        let flood_window_secs = BaseApiClient::config_u64(&config, "flood_window_secs", DEFAULT_FLOOD_WINDOW_SECS);
        let mute_duration_secs = BaseApiClient::config_u64(&config, "mute_duration_secs", DEFAULT_MUTE_DURATION_SECS);

        // Construire la config des detecteurs depuis la guild config
        let mut detector_config = build_detector_config(&config);

        // Night mode : seuils plus stricts pendant les heures configurees
        let night_mode_enabled = BaseApiClient::config_bool(&config, "night_mode_enabled", false);
        if night_mode_enabled {
            let start = BaseApiClient::config_u64(&config, "night_start_hour", 22) as u8;
            let end = BaseApiClient::config_u64(&config, "night_end_hour", 8) as u8;
            if is_night_mode(start, end) {
                apply_night_mode(&mut detector_config);
            }
        }

        // Construire les couleurs des embeds depuis la guild config
        let colors = build_embed_colors(&config);

        // Verifier les salons exclus
        let ignored_channels_str = BaseApiClient::config_or(&config, "ignored_channels", "");
        if !ignored_channels_str.is_empty() {
            let channel_id_str = msg.channel_id.get().to_string();
            let ignored: Vec<&str> = ignored_channels_str.split(',').map(|s| s.trim()).collect();
            if ignored.iter().any(|id| *id == channel_id_str) {
                return;
            }
        }

        // Verifier les roles ignores
        let ignored_roles_str = BaseApiClient::config_or(&config, "ignored_roles", "");
        if !ignored_roles_str.is_empty() {
            if let Some(member) = &msg.member {
                let ignored: Vec<&str> = ignored_roles_str.split(',').map(|s| s.trim()).collect();
                for role_id_str in &ignored {
                    if let Ok(role_id) = role_id_str.parse::<u64>() {
                        if member.roles.iter().any(|r| r.get() == role_id) {
                            return;
                        }
                    }
                }
            }
        }

        // 0. Detection pieces jointes suspectes
        if detector_config.suspicious_files_enabled && !msg.attachments.is_empty() {
            const DANGEROUS_EXTENSIONS: &[&str] = &[
                "exe", "bat", "cmd", "scr", "ps1", "vbs", "js",
                "jar", "com", "pif", "msi", "dll", "reg", "hta",
            ];

            let suspicious = msg.attachments.iter().find(|a| {
                let name_lower = a.filename.to_lowercase();
                let ext = name_lower.rsplit('.').next().unwrap_or("");
                DANGEROUS_EXTENSIONS.contains(&ext)
                    || detector_config.suspicious_file_extensions.iter().any(|e| e == ext)
            });

            if let Some(attachment) = suspicious {
                let embed = moderate_embed("🗑\u{fe0f} Fichier suspect supprime")
                    .color(colors.delete)
                    .field("📝 Raison", "Piece jointe avec extension dangereuse.", false)
                    .field("📎 Fichier", &attachment.filename, false)
                    .thumbnail(msg.author.face());
                let builder = serenity::builder::CreateMessage::new().embed(embed);
                if let Err(e) = msg.channel_id.send_message(&ctx.http, builder).await {
                    warn!(error = %e, "Echec envoi notification fichier suspect");
                }
                if let Err(e) = msg.delete(&ctx.http).await {
                    warn!(error = %e, message_id = %msg.id, "Echec suppression message fichier suspect");
                }

                let log_msg = format!("Fichier suspect supprime — {} : {}", msg.author.name, attachment.filename);
                let guild_id_str = guild_id.clone();
                let data = ctx.data.read().await;
                if let Some(base) = data.get::<ApiClientKey>() {
                    base.send_log("warn", &guild_id_str, &log_msg);
                }
                info!(user = %msg.author.name, filename = %attachment.filename, "Fichier suspect supprime");
                return;
            }
        }

        let content = &msg.content;

        // 1. Detection flood
        let is_flood = {
            let data = ctx.data.read().await;
            if let Some(tracker) = data.get::<FloodTrackerKey>() {
                let key = (msg.channel_id, msg.author.id);
                let now = Instant::now();
                let mut entry = tracker.entry(key).or_default();
                let timestamps = entry.value_mut();
                timestamps.retain(|t| now.duration_since(*t).as_secs() < flood_window_secs);
                timestamps.push(now);
                let flood = timestamps.len() >= flood_max_messages;
                // Cleanup periodique : supprimer les utilisateurs inactifs > 10 min
                if tracker.len() > 5000 {
                    tracker.retain(|_, ts| {
                        !ts.is_empty() && now.duration_since(*ts.last().unwrap()).as_secs() < 600
                    });
                }
                flood
            } else {
                false
            }
        };

        if is_flood {
            let data = ctx.data.read().await;
            if let Some(tracker) = data.get::<FloodTrackerKey>() {
                tracker.remove(&(msg.channel_id, msg.author.id));
            }
            drop(data);

            let embed = warn_embed("⚠\u{fe0f} Avertissement AutoMod")
                .color(colors.warn)
                .field("📝 Raison", "Merci de ne pas envoyer autant de messages aussi rapidement.", false)
                .thumbnail(msg.author.face());
            let builder = serenity::builder::CreateMessage::new().embed(embed);
            if let Err(e) = msg.channel_id.send_message(&ctx.http, builder).await {
                warn!(error = %e, "Echec envoi avertissement flood");
            }

            info!(user = %msg.author.name, "Flood detecte");

            let flags = detectors::DetectionFlags { spam: true, insult: false, link: false, phishing: false };
            let log_channel_id = BaseApiClient::config_u64(&config, "log_channel_id", 0);
            send_to_backend(&ctx, &msg, flags, mute_duration_secs, log_channel_id, &colors).await;
            return;
        }

        // 2. Detection caps (avertissement seulement, pas d'infraction)
        if detector_config.caps_enabled
            && detectors::spam::detect_caps(content, detector_config.caps_threshold_chars)
        {
            let embed = warn_embed("⚠\u{fe0f} Avertissement AutoMod")
                .color(colors.warn)
                .field("📝 Raison", "Merci d'ecrire normalement sans tout mettre en majuscules.", false)
                .thumbnail(msg.author.face());
            let builder = serenity::builder::CreateMessage::new().embed(embed);
            if let Err(e) = msg.channel_id.send_message(&ctx.http, builder).await {
                warn!(error = %e, "Echec envoi avertissement caps");
            }
            info!(user = %msg.author.name, "Caps detecte, avertissement envoye");
        }

        // 3. Slowmode adaptatif — tracker le volume de messages
        {
            let adaptive_enabled = BaseApiClient::config_bool(&config, "adaptive_slowmode_enabled", false);
            if adaptive_enabled {
                let threshold = BaseApiClient::config_u64(&config, "adaptive_slowmode_threshold", 15) as usize;
                let slowmode_secs = BaseApiClient::config_u64(&config, "adaptive_slowmode_seconds", 5) as u16;

                let data = ctx.data.read().await;
                if let Some(tracker) = data.get::<SlowmodeTrackerKey>() {
                    tracker.record_message(msg.channel_id);
                    if tracker.should_activate(msg.channel_id, threshold)
                        && tracker.try_start_activation(msg.channel_id)
                    {
                        let edit = serenity::builder::EditChannel::new().rate_limit_per_user(slowmode_secs);
                        if let Err(e) = msg.channel_id.edit(&ctx.http, edit).await {
                            warn!(error = %e, "Impossible d'activer le slowmode adaptatif");
                        } else {
                            info!(
                                channel_id = %msg.channel_id,
                                slowmode_secs,
                                "Slowmode adaptatif active ({}msg/30s)",
                                threshold
                            );
                            tracker.reset(msg.channel_id);
                        }
                        tracker.finish_activation(msg.channel_id);
                    }
                    // Cleanup periodique
                    if tracker.tracked_channels() > 500 {
                        tracker.cleanup();
                    }
                }
            }
        }

        // 4. Analyse locale (spam, insulte, lien, phishing, unicode)
        let flags = detectors::analyze(content, &detector_config);

        // Verifier si l'IA texte est activee pour cette guild
        let ia_text_enabled = BaseApiClient::config_bool(&config, "text_enabled", true);

        // Envoyer a l'API si flags locaux detectes OU si l'IA est activee
        let should_analyze = flags.spam || flags.insult || flags.link || flags.phishing || ia_text_enabled;

        if !should_analyze {
            return;
        }

        if flags.spam || flags.insult || flags.link || flags.phishing {
            info!(
                guild_id = ?msg.guild_id,
                user = %msg.author.name,
                flags.spam = flags.spam,
                flags.insult = flags.insult,
                flags.link = flags.link,
                flags.phishing = flags.phishing,
                "Message flagge localement"
            );
        }

        let log_channel_id = BaseApiClient::config_u64(&config, "log_channel_id", 0);

        // Spawn en background pour ne pas bloquer le bot
        let ctx_clone = ctx.clone();
        let msg_clone = msg.clone();
        tokio::spawn(async move {
            send_to_backend(&ctx_clone, &msg_clone, flags, mute_duration_secs, log_channel_id, &colors).await;
        });
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, "Automod bot connecte");

        // Definir le statut en ligne
        ctx.set_presence(
            Some(serenity::gateway::ActivityData::watching("les messages")),
            serenity::model::user::OnlineStatus::Online,
        );

        register_guilds(&ctx, &ready).await;

        if let Err(e) = serenity::model::application::Command::set_global_commands(
            &ctx.http,
            commands::all(),
        )
        .await
        {
            error!(error = %e, "Erreur enregistrement commandes");
        } else {
            info!("Slash commands enregistrees : automod");
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            if let Some(guild_id) = command.guild_id {
                let data = ctx.data.read().await;
                if let Some(api) = data.get::<ApiClientKey>() {
                    if !sentinel_shared::discord_helpers::is_bot_enabled(api, &guild_id.to_string()).await {
                        return;
                    }
                }
            }

            if command.data.name.as_str() == "automod" {
                commands::automod::handle(&ctx, &command).await;
            }
        }
    }
}

/// Sanitise le contenu utilisateur pour l'affichage dans les embeds Discord.
/// Empeche l'injection de markdown, mentions, spoilers, etc.
fn sanitize_embed_content(content: &str, max_len: usize) -> String {
    let truncated: String = content.chars().take(max_len).collect();
    truncated
        .replace("```", "` ` `")
        .replace("||", "| |")
        .replace('@', "@\u{200b}") // zero-width space pour bloquer les mentions
}

/// Construit la config des detecteurs depuis la guild config.
fn build_detector_config(config: &std::collections::HashMap<String, String>) -> DetectorConfig {
    DetectorConfig {
        spam_enabled: BaseApiClient::config_bool(config, "spam_detection_enabled", true),
        spam_repeat_char_threshold: BaseApiClient::config_u64(config, "spam_repeat_char_threshold", 6) as usize,
        spam_repeat_word_threshold: BaseApiClient::config_u64(config, "spam_repeat_word_threshold", 5) as usize,
        caps_enabled: BaseApiClient::config_bool(config, "caps_warning_enabled", true),
        caps_threshold_chars: BaseApiClient::config_u64(config, "caps_threshold_chars", 8) as usize,
        insult_enabled: BaseApiClient::config_bool(config, "insult_detection_enabled", true),
        insult_custom_words: sentinel_shared::parsers::split_csv(&BaseApiClient::config_or(config, "insult_custom_words", "")),
        link_enabled: BaseApiClient::config_bool(config, "link_detection_enabled", true),
        allow_discord_invites: BaseApiClient::config_bool(config, "allow_discord_invites", false),
        allowed_domains: sentinel_shared::parsers::split_csv(&BaseApiClient::config_or(config, "allowed_domains", "")),
        phishing_enabled: BaseApiClient::config_bool(config, "phishing_detection_enabled", true),
        phishing_extra_whitelist: sentinel_shared::parsers::split_csv(&BaseApiClient::config_or(config, "phishing_extra_whitelist", "")),
        emoji_spam_enabled: BaseApiClient::config_bool(config, "emoji_spam_enabled", true),
        emoji_spam_max: BaseApiClient::config_u64(config, "emoji_spam_max", 10) as usize,
        mentions_enabled: BaseApiClient::config_bool(config, "mentions_enabled", true),
        mentions_max: BaseApiClient::config_u64(config, "mentions_max", 5) as usize,
        suspicious_files_enabled: BaseApiClient::config_bool(config, "suspicious_files_enabled", true),
        suspicious_file_extensions: sentinel_shared::parsers::split_csv(&BaseApiClient::config_or(config, "suspicious_file_extensions", "")),
        unicode_enabled: BaseApiClient::config_bool(config, "unicode_detection_enabled", true),
        unicode_max_combining: BaseApiClient::config_u64(config, "unicode_max_combining", 3) as usize,
        unicode_max_invisible: BaseApiClient::config_u64(config, "unicode_max_invisible", 5) as usize,
    }
}

/// Construit les couleurs d'embeds depuis la guild config.
fn build_embed_colors(config: &std::collections::HashMap<String, String>) -> EmbedColors {
    EmbedColors {
        warn:   parse_color(&BaseApiClient::config_or(config, "color_warn",   "f59e0b"), 0xf59e0b),
        delete: parse_color(&BaseApiClient::config_or(config, "color_delete", "f97316"), 0xf97316),
        mute:   parse_color(&BaseApiClient::config_or(config, "color_mute",   "ef4444"), 0xef4444),
        ban:    parse_color(&BaseApiClient::config_or(config, "color_ban",    "dc2626"), 0xdc2626),
    }
}

/// Parse une couleur hex (avec ou sans #) vers u32. Retourne `default` si invalide.
fn parse_color(hex: &str, default: u32) -> u32 {
    u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(default)
}

/// Verifie si l'heure actuelle est dans la plage de nuit.
fn is_night_mode(start: u8, end: u8) -> bool {
    let hour = time::OffsetDateTime::now_utc().hour();
    if start > end {
        // Passage par minuit (ex: 22h-8h)
        hour >= start || hour < end
    } else {
        hour >= start && hour < end
    }
}

/// Reduit les seuils de detection pour le mode nuit (seuils divises par ~2).
fn apply_night_mode(config: &mut DetectorConfig) {
    config.spam_repeat_char_threshold = (config.spam_repeat_char_threshold / 2).max(2);
    config.spam_repeat_word_threshold = (config.spam_repeat_word_threshold / 2).max(2);
    config.caps_threshold_chars = (config.caps_threshold_chars / 2).max(4);
    config.emoji_spam_max = (config.emoji_spam_max / 2).max(3);
    config.mentions_max = (config.mentions_max / 2).max(2);
}


/// Envoie le message au backend pour analyse et execute l'action.
async fn send_to_backend(
    ctx: &Context,
    msg: &Message,
    flags: detectors::DetectionFlags,
    mute_duration_secs: u64,
    log_channel_id: u64,
    colors: &EmbedColors,
) {
    let request = AnalyzeRequest {
        guild_id: msg.guild_id.map(|id| id.to_string()).unwrap_or_default(),
        channel_id: msg.channel_id.to_string(),
        user_id: msg.author.id.to_string(),
        username: msg.author.name.clone(),
        content: msg.content.clone(),
        flags,
        metadata: MessageMetadata {
            message_id: msg.id.to_string(),
            timestamp: msg.timestamp.to_string(),
        },
    };

    let data = ctx.data.read().await;
    let base = match data.get::<ApiClientKey>() {
        Some(client) => Arc::clone(client),
        None => {
            error!("BaseApiClient introuvable dans le contexte");
            return;
        }
    };
    drop(data);

    let api_client = ApiClient::new(Arc::clone(&base));

    match api_client.analyze(&request).await {
        Ok(response) => {
            info!(action = ?response.action, reason = ?response.reason, "Reponse du backend");

            if response.action != Action::None {
                let guild_id = msg.guild_id.map(|id| id.to_string()).unwrap_or_default();
                let action_label = match &response.action {
                    Action::Warn => "Avertissement",
                    Action::Delete => "Suppression",
                    Action::Mute => "Mute",
                    Action::Ban => "Proposition de ban",
                    Action::None => "",
                };
                let log_message = format!(
                    "{} — {} : {}",
                    action_label,
                    msg.author.name,
                    response.reason.as_deref().unwrap_or("Automod"),
                );

                base.send_log(
                    if matches!(response.action, Action::Ban) { "error" } else { "warn" },
                    &guild_id,
                    &log_message,
                );

                if log_channel_id != 0 {
                    send_discord_log(
                        ctx, msg, &response.action, action_label,
                        response.reason.as_deref().unwrap_or("Automod"),
                        &request.flags, log_channel_id, colors,
                    ).await;
                }
            }

            if let Err(e) = execute_action(ctx, msg, &response.action, response.reason.as_deref(), mute_duration_secs, colors).await {
                error!(error = %e, "Erreur lors de l'execution de l'action");
            }
        }
        Err(e) => {
            error!(error = %e, "Backend injoignable — action locale par defaut");
            // En mode fallback, supprimer les messages flagges (phishing, insulte, spam, lien)
            let reason = if request.flags.phishing {
                Some("Lien suspect detecte.")
            } else if request.flags.insult {
                Some("Langage inapproprie.")
            } else if request.flags.spam {
                Some("Spam detecte.")
            } else if request.flags.link {
                Some("Lien non autorise.")
            } else {
                None
            };

            if let Some(reason_text) = reason {
                let embed = moderate_embed("🗑\u{fe0f} Message supprime (mode hors-ligne)")
                    .color(colors.delete)
                    .field("📝 Raison", reason_text, false)
                    .thumbnail(msg.author.face());
                let builder = serenity::builder::CreateMessage::new().embed(embed);
                if let Err(e) = msg.channel_id.send_message(&ctx.http, builder).await {
                    warn!(error = %e, "Echec envoi notification mode hors-ligne");
                }
                if let Err(e) = msg.delete(&ctx.http).await {
                    warn!(error = %e, message_id = %msg.id, "Echec suppression message mode hors-ligne");
                }
            }
        }
    }
}

/// Execute l'action decidee par le backend.
async fn execute_action(
    ctx: &Context,
    msg: &Message,
    action: &Action,
    reason: Option<&str>,
    mute_duration_secs: u64,
    colors: &EmbedColors,
) -> Result<(), serenity::Error> {
    let reason_text = reason.unwrap_or("Automod");

    match action {
        Action::None => {}
        Action::Warn => {
            let embed = warn_embed("⚠\u{fe0f} Avertissement AutoMod")
                .color(colors.warn)
                .field("📝 Raison", reason_text, false)
                .thumbnail(msg.author.face());
            let builder = serenity::builder::CreateMessage::new().embed(embed);
            msg.channel_id.send_message(&ctx.http, builder).await?;
            info!(user = %msg.author.name, "Avertissement envoye");
        }
        Action::Delete => {
            let content_preview = sanitize_embed_content(&msg.content, 200);
            let embed = moderate_embed("🗑\u{fe0f} Message supprime")
                .color(colors.delete)
                .field("📝 Raison", reason_text, false)
                .field("📄 Contenu original", format!("```{}```", content_preview), false)
                .thumbnail(msg.author.face());
            let builder = serenity::builder::CreateMessage::new().embed(embed);
            if let Err(e) = msg.channel_id.send_message(&ctx.http, builder).await {
                warn!(error = %e, "Echec envoi notification suppression");
            }
            msg.delete(&ctx.http).await?;
            info!(message_id = %msg.id, "Message supprime");
        }
        Action::Mute => {
            let mute_minutes = mute_duration_secs / 60;
            let embed = danger_embed("🔇 Mute automatique")
                .color(colors.mute)
                .field("📝 Raison", reason_text, false)
                .field("⏱ Duree", format!("{} minutes", mute_minutes), false)
                .thumbnail(msg.author.face());
            let builder = serenity::builder::CreateMessage::new().embed(embed);
            if let Err(e) = msg.channel_id.send_message(&ctx.http, builder).await {
                warn!(error = %e, "Echec envoi notification mute");
            }
            msg.delete(&ctx.http).await?;
            if let (Some(guild_id), Ok(member)) = (msg.guild_id, msg.member(&ctx.http).await) {
                let mut member = guild_id.member(&ctx.http, member.user.id).await?;
                let secs = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64
                    + mute_duration_secs as i64;
                let datetime = match time::OffsetDateTime::from_unix_timestamp(secs) {
                    Ok(dt) => dt,
                    Err(e) => {
                        error!(error = %e, "Timestamp invalide pour le mute");
                        return Ok(());
                    }
                };
                let timeout = serenity::model::Timestamp::from(datetime);
                member
                    .disable_communication_until_datetime(&ctx.http, timeout)
                    .await?;
                info!(user = %msg.author.name, duration_secs = mute_duration_secs, "Utilisateur mute");
            }
        }
        Action::Ban => {
            if let Some(_guild_id) = msg.guild_id {
                let embed = critical_embed("🔨 Signalement pour bannissement")
                    .color(colors.ban)
                    .field("📝 Raison", reason_text, false)
                    .thumbnail(msg.author.face());
                let builder = serenity::builder::CreateMessage::new().embed(embed);
                if let Err(e) = msg.channel_id.send_message(&ctx.http, builder).await {
                    warn!(error = %e, "Echec envoi notification ban");
                }
                msg.delete(&ctx.http).await?;
                info!(user = %msg.author.name, "Proposition de ban enregistree (ban non execute)");
            }
        }
    }

    Ok(())
}

/// Envoie un log soigne dans le salon Discord de logs (embed riche).
async fn send_discord_log(
    ctx: &Context,
    msg: &Message,
    action: &Action,
    action_label: &str,
    reason: &str,
    flags: &detectors::DetectionFlags,
    log_channel_id: u64,
    colors: &EmbedColors,
) {
    let channel = serenity::model::id::ChannelId::new(log_channel_id);

    let (icon, color) = match action {
        Action::Warn    => ("\u{26a0}\u{fe0f}", colors.warn),
        Action::Delete  => ("\u{1f5d1}\u{fe0f}", colors.delete),
        Action::Mute    => ("\u{1f507}", colors.mute),
        Action::Ban     => ("\u{1f6ab}", colors.ban),
        Action::None    => ("\u{2705}", 0x22c55e),
    };

    let mut detections = Vec::new();
    if flags.spam     { detections.push("\u{1f4e8} Spam"); }
    if flags.insult   { detections.push("\u{1f92c} Insulte"); }
    if flags.link     { detections.push("\u{1f517} Lien"); }
    if flags.phishing { detections.push("\u{1f3a3} Phishing"); }
    let detections_text = if detections.is_empty() {
        "Aucune".to_string()
    } else {
        detections.join(" | ")
    };

    let content_preview = sanitize_embed_content(&msg.content, 300);

    let embed = serenity::builder::CreateEmbed::new()
        .author(serenity::builder::CreateEmbedAuthor::new(
            format!("{} {}", icon, action_label),
        ))
        .title("AutoMod - Detection automatique")
        .color(color)
        .field(
            "\u{1f464} Utilisateur",
            format!("<@{}> (`{}`)", msg.author.id, msg.author.name),
            true,
        )
        .field(
            "\u{1f4ac} Salon",
            format!("<#{}>", msg.channel_id),
            true,
        )
        .field(
            "\u{2699}\u{fe0f} Action",
            action_label,
            true,
        )
        .field(
            "\u{1f50d} Detections",
            detections_text,
            false,
        )
        .field(
            "\u{1f4dd} Raison",
            reason,
            false,
        )
        .field(
            "\u{1f4e9} Message original",
            format!("```{}```", content_preview),
            false,
        )
        .thumbnail(msg.author.face())
        .footer(serenity::builder::CreateEmbedFooter::new(
            format!("ID: {} | Auteur: {}", msg.id, msg.author.id),
        ))
        .timestamp(serenity::model::Timestamp::now());

    let builder = serenity::builder::CreateMessage::new().embed(embed);
    if let Err(e) = channel.send_message(&ctx.http, builder).await {
        tracing::warn!(error = %e, "Impossible d'envoyer le log Discord");
    }
}
