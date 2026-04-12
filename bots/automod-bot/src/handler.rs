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
                if processed.len() > 2000 {
                    processed.retain(|_, ts| now.duration_since(*ts).as_secs() < 300);
                }
            }
        }

        // Charger la config depuis l'API pour ce guild
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

        let mut detector_config = build_detector_config(&config);

        let night_mode_enabled = BaseApiClient::config_bool(&config, "night_mode_enabled", false);
        if night_mode_enabled {
            let start = BaseApiClient::config_u64(&config, "night_start_hour", 22) as u8;
            let end = BaseApiClient::config_u64(&config, "night_end_hour", 8) as u8;
            if is_night_mode(start, end) {
                apply_night_mode(&mut detector_config);
            }
        }

        let colors = build_embed_colors(&config);
        let log_channel_id = BaseApiClient::config_u64(&config, "log_channel_id", 0);

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

        let content = &msg.content;

        // Detection pieces jointes suspectes
        let files_review = BaseApiClient::config_bool(&config, "files_review_mode", true);
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
                info!(user = %msg.author.name, filename = %attachment.filename, "Fichier suspect detecte");
                let reason = format!("Piece jointe suspecte : {}", attachment.filename);

                if files_review && log_channel_id != 0 {
                    let flags = detectors::DetectionFlags { spam: false, insult: false, link: false, phishing: false };
                    send_review_card(&ctx, &msg, &Action::Delete, &reason, 1.0, &flags, log_channel_id, &colors).await;
                } else {
                    let embed = moderate_embed("🗑\u{fe0f} Fichier suspect supprime")
                        .color(colors.delete)
                        .field("📝 Raison", &reason, false)
                        .field("📎 Fichier", &attachment.filename, false)
                        .thumbnail(msg.author.face());
                    let builder = serenity::builder::CreateMessage::new().embed(embed);
                    if let Err(e) = msg.channel_id.send_message(&ctx.http, builder).await {
                        warn!(error = %e, "Echec envoi notification fichier suspect");
                    }
                    if let Err(e) = msg.delete(&ctx.http).await {
                        warn!(error = %e, message_id = %msg.id, "Echec suppression message fichier suspect");
                    }
                }

                let log_msg = format!("Fichier suspect — {} : {}", msg.author.name, attachment.filename);
                let data = ctx.data.read().await;
                if let Some(base) = data.get::<ApiClientKey>() {
                    base.send_log("warn", &guild_id, &log_msg);
                }
                return;
            }
        }

        // Detection flood (clone le tracker pour eviter deadlock sur le RwLock)
        {
            let flood_tracker = {
                let data = ctx.data.read().await;
                data.get::<FloodTrackerKey>().cloned()
            };
            // Le lock ctx.data est libere ici

            let is_flood = if let Some(tracker) = &flood_tracker {
                let key = (msg.channel_id, msg.author.id);
                let now = Instant::now();
                let mut entry = tracker.entry(key).or_default();
                let timestamps = entry.value_mut();
                timestamps.retain(|t| now.duration_since(*t).as_secs() < flood_window_secs);
                timestamps.push(now);
                let flood = timestamps.len() >= flood_max_messages;
                // Drop le entry pour eviter le deadlock avec retain
                drop(entry);
                if tracker.len() > 5000 {
                    tracker.retain(|_, ts| {
                        ts.last()
                            .map(|t| now.duration_since(*t).as_secs() < 600)
                            .unwrap_or(false)
                    });
                }
                flood
            } else {
                false
            };

            if is_flood {
                info!(user = %msg.author.name, "Flood detecte");
                if let Some(tracker) = &flood_tracker {
                    tracker.remove(&(msg.channel_id, msg.author.id));
                }

                let flood_review = BaseApiClient::config_bool(&config, "flood_review_mode", true);
                if flood_review && log_channel_id != 0 {
                    let flags = detectors::DetectionFlags { spam: true, insult: false, link: false, phishing: false };
                    send_review_card(&ctx, &msg, &Action::Warn, "Flood detecte — messages envoyes trop rapidement.", 0.9, &flags, log_channel_id, &colors).await;
                } else {
                    let embed = warn_embed("⚠\u{fe0f} Avertissement AutoMod")
                        .color(colors.warn)
                        .field("📝 Raison", "Merci de ne pas envoyer autant de messages aussi rapidement.", false)
                        .thumbnail(msg.author.face());
                    let builder = serenity::builder::CreateMessage::new().embed(embed);
                    if let Err(e) = msg.channel_id.send_message(&ctx.http, builder).await {
                        warn!(error = %e, "Echec envoi avertissement flood");
                    }

                    let flags = detectors::DetectionFlags { spam: true, insult: false, link: false, phishing: false };
                    let ctx_max_msgs = BaseApiClient::config_u64(&config, "context_max_messages", 3) as u8;
                    let ctx_max_chars = BaseApiClient::config_u64(&config, "context_max_chars", 200) as usize;
                    let ctx_clone = ctx.clone();
                    let msg_clone = msg.clone();
                    tokio::spawn(async move {
                        let ai_review = true; // flood passe par le backend IA en review
                        send_to_backend(&ctx_clone, &msg_clone, flags, mute_duration_secs, log_channel_id, ai_review, &colors, ctx_max_msgs, ctx_max_chars).await;
                    });
                }
                return;
            }
        }

        // Detection caps
        if detector_config.caps_enabled
            && detectors::spam::detect_caps(content, detector_config.caps_threshold_chars)
        {
            info!(user = %msg.author.name, "Caps detecte");
            let caps_review = BaseApiClient::config_bool(&config, "caps_review_mode", true);
            if caps_review && log_channel_id != 0 {
                let flags = detectors::DetectionFlags { spam: true, insult: false, link: false, phishing: false };
                send_review_card(&ctx, &msg, &Action::Warn, "Abus de majuscules detecte.", 0.8, &flags, log_channel_id, &colors).await;
            } else {
                let embed = warn_embed("⚠\u{fe0f} Avertissement AutoMod")
                    .color(colors.warn)
                    .field("📝 Raison", "Merci d'ecrire normalement sans tout mettre en majuscules.", false)
                    .thumbnail(msg.author.face());
                let builder = serenity::builder::CreateMessage::new().embed(embed);
                if let Err(e) = msg.channel_id.send_message(&ctx.http, builder).await {
                    warn!(error = %e, "Echec envoi avertissement caps");
                }
            }
        }

        // Slowmode adaptatif
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
                            info!(channel_id = %msg.channel_id, slowmode_secs, "Slowmode adaptatif active");
                            tracker.reset(msg.channel_id);
                        }
                        tracker.finish_activation(msg.channel_id);
                    }
                    if tracker.tracked_channels() > 500 {
                        tracker.cleanup();
                    }
                }
            }
        }

        // Analyse locale (spam, insulte, lien, phishing)
        let flags = detectors::analyze(content, &detector_config);

        if flags.spam || flags.insult || flags.link || flags.phishing {
            info!(
                user = %msg.author.name,
                spam = flags.spam, insult = flags.insult, link = flags.link, phishing = flags.phishing,
                "Message flagge localement"
            );
        }

        let ia_text_enabled = BaseApiClient::config_bool(&config, "text_enabled", true);
        let should_analyze = flags.spam || flags.insult || flags.link || flags.phishing || ia_text_enabled;

        if !should_analyze {
            return;
        }

        let context_max_messages = BaseApiClient::config_u64(&config, "context_max_messages", 3) as u8;
        let context_max_chars = BaseApiClient::config_u64(&config, "context_max_chars", 200) as usize;

        let ctx_clone = ctx.clone();
        let msg_clone = msg.clone();
        tokio::spawn(async move {
            let ai_review = BaseApiClient::config_bool(&config, "ai_review_mode", true);
            send_to_backend(&ctx_clone, &msg_clone, flags, mute_duration_secs, log_channel_id, ai_review, &colors, context_max_messages, context_max_chars).await;
        });
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, "Automod bot connecte");

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

        // Background task : desactiver le slowmode adaptatif quand l'activite retombe
        let ctx_clone = ctx.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                let data = ctx_clone.data.read().await;
                if let Some(tracker) = data.get::<SlowmodeTrackerKey>() {
                    let to_deactivate = tracker.channels_to_deactivate(15);
                    drop(data);
                    for channel_id in to_deactivate {
                        let edit = serenity::builder::EditChannel::new().rate_limit_per_user(0);
                        if let Err(e) = channel_id.edit(&ctx_clone.http, edit).await {
                            warn!(error = %e, channel_id = %channel_id, "Echec desactivation slowmode adaptatif");
                        } else {
                            info!(channel_id = %channel_id, "Slowmode adaptatif desactive (activite retombee)");
                        }
                    }
                }
            }
        });

        // O1/H6 — Background cleanup : purge des caches processed + flood
        // tracker toutes les 5 minutes. Evite d'attendre une burst > 2000/5000
        // items avant de nettoyer (fuite memoire lente sinon).
        let ctx_clean = ctx.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(300)).await;
                let data = ctx_clean.data.read().await;
                let now = Instant::now();

                if let Some(processed) = data.get::<ProcessedMessagesKey>() {
                    let before = processed.len();
                    processed.retain(|_, ts| now.duration_since(*ts).as_secs() < 300);
                    let removed = before.saturating_sub(processed.len());
                    if removed > 0 {
                        info!(removed, remaining = processed.len(), "Purge background processed cache");
                    }
                }

                if let Some(tracker) = data.get::<FloodTrackerKey>() {
                    let before = tracker.len();
                    tracker.retain(|_, ts| {
                        ts.last()
                            .map(|t| now.duration_since(*t).as_secs() < 600)
                            .unwrap_or(false)
                    });
                    let removed = before.saturating_sub(tracker.len());
                    if removed > 0 {
                        info!(removed, remaining = tracker.len(), "Purge background flood tracker");
                    }
                }
            }
        });
    }

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
                if command.data.name.as_str() == "automod" {
                    commands::automod::handle(&ctx, &command).await;
                }
            }
            Interaction::Component(component) => {
                if component.data.custom_id.starts_with("am_") {
                    handle_review_button(&ctx, &component).await;
                }
            }
            _ => {}
        }
    }
}

// ══════════════════════════════════════════════════════════════════════
// Phase 8 — Mode review : carte de proposition + handler de boutons
// ══════════════════════════════════════════════════════════════════════

/// Custom ID format : `am_{action}:{guild_id}:{channel_id}:{message_id}:{user_id}`
/// action = w (warn) | d (delete) | m (mute) | b (ban) | i (ignore)
const AM_PREFIX: &str = "am_";

/// Envoie une carte de review dans le salon de logs au lieu d'appliquer
/// l'action directement. Les moderateurs cliquent sur un bouton pour
/// valider ou ajuster la severite.
async fn send_review_card(
    ctx: &Context,
    msg: &Message,
    suggested_action: &Action,
    reason: &str,
    score: f64,
    flags: &crate::detectors::DetectionFlags,
    review_channel_id: u64,
    colors: &EmbedColors,
) {
    let guild_id = msg.guild_id.map(|g| g.to_string()).unwrap_or_default();
    let channel_id = msg.channel_id.to_string();
    let message_id = msg.id.to_string();
    let user_id = msg.author.id.to_string();
    let content_preview = sanitize_embed_content(&msg.content, 500);

    let action_label = match suggested_action {
        Action::Warn => "⚠️ Avertissement",
        Action::Delete => "🗑️ Suppression",
        Action::Mute => "🔇 Mute",
        Action::Ban => "🔨 Bannissement",
        Action::None => return,
    };

    let action_color = match suggested_action {
        Action::Warn => colors.warn,
        Action::Delete => colors.delete,
        Action::Mute => colors.mute,
        Action::Ban => colors.ban,
        Action::None => 0x95a5a6,
    };

    let mut flag_parts = Vec::new();
    if flags.spam { flag_parts.push("Spam"); }
    if flags.insult { flag_parts.push("Insulte"); }
    if flags.link { flag_parts.push("Lien"); }
    if flags.phishing { flag_parts.push("Phishing"); }
    let flags_str = if flag_parts.is_empty() { "Aucun".to_string() } else { flag_parts.join(", ") };

    let embed = serenity::builder::CreateEmbed::new()
        .title(format!("🛡️ AutoMod — Action suggeree : {}", action_label))
        .color(action_color)
        .field("👤 Utilisateur", format!("<@{}> (`{}`)", user_id, msg.author.name), true)
        .field("💬 Salon", format!("<#{}>", channel_id), true)
        .field("🎯 Score IA", format!("{:.0}%", score * 100.0), true)
        .field("📝 Message original", format!("```{}```", content_preview), false)
        .field("🤖 Raison IA", reason, false)
        .field("🚩 Flags detectes", &flags_str, true)
        .thumbnail(msg.author.face())
        .footer(serenity::builder::CreateEmbedFooter::new(
            "AutoMod Review | Cliquez pour valider ou ajuster",
        ))
        .timestamp(serenity::model::Timestamp::now());

    // Suffixe commun pour les custom_id
    let id_suffix = format!("{}:{}:{}:{}", guild_id, channel_id, message_id, user_id);

    // Bouton principal (action suggeree) + ajustements + ignorer.
    // Les boutons de la rangee 2 excluent l'action suggeree pour eviter
    // un custom_id duplique (Discord rejette le formulaire sinon).
    let suggested_char = action_char(suggested_action);

    let btn_apply = serenity::builder::CreateButton::new(format!("am_{}:{}", suggested_char, id_suffix))
        .label(format!("✅ Appliquer ({})", action_label))
        .style(serenity::all::ButtonStyle::Success);

    let btn_ignore = serenity::builder::CreateButton::new(format!("am_i:{}", id_suffix))
        .label("❌ Ignorer")
        .style(serenity::all::ButtonStyle::Secondary);

    // Rangee 2 : ajustements de severite (sans doublon avec le bouton principal)
    let mut adjust_buttons = Vec::new();
    if suggested_char != 'w' {
        adjust_buttons.push(
            serenity::builder::CreateButton::new(format!("am_w:{}", id_suffix))
                .label("⚠️ Warn")
                .style(serenity::all::ButtonStyle::Secondary),
        );
    }
    if suggested_char != 'd' {
        adjust_buttons.push(
            serenity::builder::CreateButton::new(format!("am_d:{}", id_suffix))
                .label("🗑️ Delete")
                .style(serenity::all::ButtonStyle::Secondary),
        );
    }
    if suggested_char != 'm' {
        adjust_buttons.push(
            serenity::builder::CreateButton::new(format!("am_m:{}", id_suffix))
                .label("🔇 Mute")
                .style(serenity::all::ButtonStyle::Danger),
        );
    }

    let row1 = serenity::builder::CreateActionRow::Buttons(vec![btn_apply, btn_ignore]);
    let row2 = serenity::builder::CreateActionRow::Buttons(adjust_buttons);

    let builder = serenity::builder::CreateMessage::new()
        .embed(embed)
        .components(vec![row1, row2]);

    match serenity::model::id::ChannelId::new(review_channel_id)
        .send_message(&ctx.http, builder)
        .await
    {
        Ok(_) => info!(
            user = %msg.author.name,
            channel = %msg.channel_id,
            action = %action_label,
            review_channel = review_channel_id,
            "Carte de review envoyee"
        ),
        Err(e) => error!(
            error = %e,
            review_channel = review_channel_id,
            "Echec envoi carte de review automod — verifier que le bot a acces au salon"
        ),
    }
}

fn action_char(action: &Action) -> char {
    match action {
        Action::Warn => 'w',
        Action::Delete => 'd',
        Action::Mute => 'm',
        Action::Ban => 'b',
        Action::None => 'i',
    }
}

fn char_to_action(c: char) -> Action {
    match c {
        'w' => Action::Warn,
        'd' => Action::Delete,
        'm' => Action::Mute,
        'b' => Action::Ban,
        _ => Action::None,
    }
}

/// Handler des boutons de review. Parse le custom_id, execute l'action
/// choisie par le moderateur, et met a jour la carte.
async fn handle_review_button(ctx: &Context, component: &serenity::model::application::ComponentInteraction) {
    // H1 — Gate permission : seuls les moderateurs peuvent valider une action.
    // Sans ce check, n'importe quel user avec le custom_id peut declencher
    // mute/ban/delete via un POST d'interaction crafted.
    // Les interactions Discord incluent `member.permissions` pre-calcule
    // dans le payload (cf. PartialMember.permissions). On l'utilise
    // directement au lieu de Member::permissions(cache) qui est deprecie
    // et ne considere pas les overwrites de channel.
    let has_perm = component
        .member
        .as_ref()
        .and_then(|m| m.permissions)
        .map(|p| {
            p.contains(serenity::all::Permissions::MODERATE_MEMBERS)
                || p.contains(serenity::all::Permissions::MANAGE_MESSAGES)
                || p.contains(serenity::all::Permissions::ADMINISTRATOR)
        })
        .unwrap_or(false);

    if !has_perm {
        let _ = component
            .create_response(
                &ctx.http,
                serenity::builder::CreateInteractionResponse::Message(
                    serenity::builder::CreateInteractionResponseMessage::new()
                        .content("❌ Seul un moderateur peut valider une action automod.")
                        .ephemeral(true),
                ),
            )
            .await;
        warn!(
            user = %component.user.name,
            user_id = %component.user.id,
            "Tentative d'action review sans permission"
        );
        return;
    }

    let custom_id = &component.data.custom_id;
    // Format : am_{action}:{guild_id}:{channel_id}:{message_id}:{user_id}
    let parts: Vec<&str> = custom_id.split(':').collect();
    if parts.len() != 5 {
        warn!(custom_id = %custom_id, "custom_id review malforme (nombre de parts incorrect)");
        return;
    }

    // C1 — Validation stricte du prefix + action char (plus de fallback "i").
    let action_str = match parts[0].strip_prefix(AM_PREFIX) {
        Some(s) => s,
        None => {
            warn!(custom_id = %custom_id, "custom_id review sans prefix am_");
            return;
        }
    };
    let action_char = match action_str.chars().next() {
        Some(c) if matches!(c, 'w' | 'd' | 'm' | 'b' | 'i') => c,
        _ => {
            warn!(custom_id = %custom_id, "custom_id review action char invalide");
            return;
        }
    };
    let action = char_to_action(action_char);
    let _guild_id_str = parts[1];
    let channel_id_str = parts[2];
    let message_id_str = parts[3];
    let user_id_str = parts[4];

    let moderator_name = &component.user.name;

    // Charger la config guild pour mute_duration
    let guild_id = component.guild_id.map(|g| g.to_string()).unwrap_or_default();
    let data = ctx.data.read().await;
    let config = if let Some(api) = data.get::<ApiClientKey>() {
        api.get_guild_config(&guild_id).await.unwrap_or_default()
    } else {
        std::collections::HashMap::new()
    };
    drop(data);

    let mute_duration_secs = BaseApiClient::config_u64(&config, "mute_duration_secs", DEFAULT_MUTE_DURATION_SECS);
    let colors = build_embed_colors(&config);

    if action == Action::None {
        // Ignorer — mettre a jour la carte
        info!(target = %user_id_str, moderator = %moderator_name, "Detection ignoree via review");
        let ignored_embed = serenity::builder::CreateEmbed::new()
            .title("🛡️ AutoMod — ❌ Ignore par un moderateur")
            .description(format!("Moderateur : **{}**\nAucune action appliquee.", moderator_name))
            .color(0x95a5a6)
            .timestamp(serenity::model::Timestamp::now());

        if let Err(e) = component
            .create_response(
                &ctx.http,
                serenity::builder::CreateInteractionResponse::UpdateMessage(
                    serenity::builder::CreateInteractionResponseMessage::new()
                        .embed(ignored_embed)
                        .components(vec![]),
                ),
            )
            .await
        {
            error!(error = %e, "Echec update carte review (ignore)");
        }
        return;
    }

    // Executer l'action sur le message original
    let action_label = match &action {
        Action::Warn => "⚠️ Avertissement",
        Action::Delete => "🗑️ Suppression",
        Action::Mute => "🔇 Mute",
        Action::Ban => "🔨 Bannissement",
        Action::None => "Aucune",
    };

    let channel_id = match channel_id_str.parse::<u64>() {
        Ok(id) => serenity::model::id::ChannelId::new(id),
        Err(_) => return,
    };

    // Execute l'action
    match action {
        Action::Warn => {
            info!(target = %user_id_str, channel = %channel_id_str, moderator = %moderator_name, "Warn valide via review");
            let embed = warn_embed("⚠️ Avertissement AutoMod")
                .color(colors.warn)
                .field("📝 Raison", "Contenu inapproprie detecte par l'IA", false)
                .field("👮 Valide par", moderator_name, true);
            if let Err(e) = channel_id.send_message(&ctx.http, serenity::builder::CreateMessage::new().embed(embed)).await {
                error!(error = %e, "Echec envoi embed warn dans le salon");
            }
        }
        Action::Delete => {
            if let Ok(msg_id) = message_id_str.parse::<u64>() {
                match channel_id
                    .delete_message(&ctx.http, serenity::model::id::MessageId::new(msg_id))
                    .await
                {
                    Ok(_) => info!(message_id = %msg_id, "Message supprime via review"),
                    Err(e) => warn!(error = %e, message_id = %msg_id, "Echec suppression message (peut-etre deja supprime)"),
                }
            }
            let embed = moderate_embed("🗑️ Message supprime par un moderateur")
                .color(colors.delete)
                .field("👮 Valide par", moderator_name, true);
            if let Err(e) = channel_id.send_message(&ctx.http, serenity::builder::CreateMessage::new().embed(embed)).await {
                error!(error = %e, "Echec envoi embed delete dans le salon");
            }
        }
        Action::Mute => {
            // H2 — Ordre inverse : MUTE d'abord, puis delete.
            // Si delete echoue (permissions message), au moins le user est mute.
            // L'inverse masquait l'echec : le modo pensait avoir agit alors
            // que l'utilisateur pouvait continuer a ecrire.
            let mut mute_applied = false;
            if let (Some(guild_id), Ok(uid)) = (component.guild_id, user_id_str.parse::<u64>()) {
                match guild_id.member(&ctx.http, serenity::model::id::UserId::new(uid)).await {
                    Ok(mut member) => {
                        let secs = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs() as i64 + mute_duration_secs as i64)
                            .unwrap_or(0);
                        match time::OffsetDateTime::from_unix_timestamp(secs) {
                            Ok(dt) => {
                                let timeout = serenity::model::Timestamp::from(dt);
                                match member.disable_communication_until_datetime(&ctx.http, timeout).await {
                                    Ok(_) => {
                                        info!(user_id = %uid, duration = mute_duration_secs, "Mute applique via review");
                                        mute_applied = true;
                                    }
                                    Err(e) => error!(error = %e, user_id = %uid, "Echec Discord disable_communication — le bot a-t-il la permission MODERATE_MEMBERS ?"),
                                }
                            }
                            Err(e) => error!(error = %e, "Timestamp invalide pour mute"),
                        }
                    }
                    Err(e) => warn!(error = %e, user_id = %uid, "Membre introuvable pour mute"),
                }
            } else {
                warn!(guild_id = ?component.guild_id, user_id = %user_id_str, "guild_id ou user_id invalide pour mute");
            }

            // Supprimer le message original APRES le mute (best-effort).
            if let Ok(msg_id) = message_id_str.parse::<u64>() {
                if let Err(e) = channel_id
                    .delete_message(&ctx.http, serenity::model::id::MessageId::new(msg_id))
                    .await
                {
                    warn!(error = %e, "Echec suppression message apres mute review");
                }
            }
            let mute_min = mute_duration_secs / 60;
            let status_text = if mute_applied { "applique" } else { "ECHOUE (voir logs)" };
            let embed = danger_embed(&format!("🔇 Mute {} par un moderateur", status_text))
                .color(colors.mute)
                .field("⏱ Duree", format!("{} minutes", mute_min), true)
                .field("👮 Valide par", moderator_name, true);
            let _ = channel_id.send_message(&ctx.http, serenity::builder::CreateMessage::new().embed(embed)).await;
        }
        Action::Ban => {
            info!(target = %user_id_str, channel = %channel_id_str, moderator = %moderator_name, "Ban signale via review");
            let embed = critical_embed("🔨 Signalement pour bannissement (valide)")
                .color(colors.ban)
                .field("👮 Valide par", moderator_name, true);
            if let Err(e) = channel_id.send_message(&ctx.http, serenity::builder::CreateMessage::new().embed(embed)).await {
                error!(error = %e, "Echec envoi embed ban dans le salon");
            }
            // Note : le ban reel reste a la main du modérateur pour l'instant
            // (pas d'auto-ban meme en review mode). On pourrait ajouter un
            // guild.ban_member() ici si le proprio de la guild le souhaite.
        }
        Action::None => {}
    }

    // Mettre a jour la carte de review (retirer les boutons, afficher le resultat)
    let result_embed = serenity::builder::CreateEmbed::new()
        .title(format!("🛡️ AutoMod — {} applique", action_label))
        .description(format!(
            "Moderateur : **{}**\nCible : <@{}>\nSalon : <#{}>",
            moderator_name, user_id_str, channel_id_str
        ))
        .color(match action {
            Action::Warn => colors.warn,
            Action::Delete => colors.delete,
            Action::Mute => colors.mute,
            Action::Ban => colors.ban,
            Action::None => 0x95a5a6,
        })
        .footer(serenity::builder::CreateEmbedFooter::new("AutoMod Review | Action executee"))
        .timestamp(serenity::model::Timestamp::now());

    let _ = component
        .create_response(
            &ctx.http,
            serenity::builder::CreateInteractionResponse::UpdateMessage(
                serenity::builder::CreateInteractionResponseMessage::new()
                    .embed(result_embed)
                    .components(vec![]),
            ),
        )
        .await;

    info!(
        moderator = %moderator_name,
        action = %action_label,
        target_user = %user_id_str,
        "Action automod validee par un moderateur"
    );
}

/// Sanitise le contenu utilisateur pour l'affichage dans les embeds Discord.
/// Empeche l'injection de markdown, mentions, spoilers, etc.
///
/// B4 — on remplace `@role`/`@everyone`/`@here` par des formes neutralisees
/// au lieu d'injecter un zero-width space (qui apparait bizarrement sur
/// certains clients Discord mobile). Les autres `@` sont laisses tels quels.
fn sanitize_embed_content(content: &str, max_len: usize) -> String {
    let truncated: String = content.chars().take(max_len).collect();
    truncated
        .replace("```", "` ` `")
        .replace("||", "| |")
        .replace("@everyone", "@-everyone")
        .replace("@here", "@-here")
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
/// M8 — lower bounds realistes : "aa" ne doit pas etre flagué meme en night mode.
fn apply_night_mode(config: &mut DetectorConfig) {
    config.spam_repeat_char_threshold = (config.spam_repeat_char_threshold / 2).max(4);
    config.spam_repeat_word_threshold = (config.spam_repeat_word_threshold / 2).max(3);
    config.caps_threshold_chars = (config.caps_threshold_chars / 2).max(6);
    config.emoji_spam_max = (config.emoji_spam_max / 2).max(5);
    config.mentions_max = (config.mentions_max / 2).max(3);
}

/// M1 — Genere une raison descriptive a partir des flags detecteurs
/// quand le backend n'en retourne pas. Evite l'affichage generique "Automod".
fn build_fallback_reason(flags: &detectors::DetectionFlags) -> String {
    let mut parts = Vec::new();
    if flags.phishing { parts.push("lien de phishing"); }
    if flags.insult { parts.push("langage inapproprie"); }
    if flags.spam { parts.push("spam"); }
    if flags.link { parts.push("lien non autorise"); }
    if parts.is_empty() {
        "Contenu inapproprie detecte".to_string()
    } else {
        format!("Detection : {}", parts.join(", "))
    }
}


/// Envoie le message au backend pour analyse et execute l'action.
async fn send_to_backend(
    ctx: &Context,
    msg: &Message,
    flags: detectors::DetectionFlags,
    mute_duration_secs: u64,
    log_channel_id: u64,
    ai_review_mode: bool,
    colors: &EmbedColors,
    context_max_messages: u8,
    context_max_chars: usize,
) {
    // Recuperer les N derniers messages du canal pour le contexte conversationnel
    let context_messages = if context_max_messages == 0 {
        Vec::new()
    } else {
        match msg
            .channel_id
            .messages(
                &ctx.http,
                serenity::builder::GetMessages::new()
                    .before(msg.id)
                    .limit(context_max_messages),
            )
            .await
        {
            Ok(messages) => messages
                .into_iter()
                .rev() // ordre chronologique
                .filter(|m| !m.author.bot)
                .map(|m| crate::api_client::ContextMessage {
                    username: m.author.name.clone(),
                    content: m.content.chars().take(context_max_chars).collect(),
                })
                .collect(),
            Err(e) => {
                tracing::warn!(error = %e, "Echec recuperation contexte canal");
                Vec::new()
            }
        }
    };

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
        context_messages,
    };

    let data = ctx.data.read().await;
    let base = match data.get::<ApiClientKey>() {
        Some(client) => Arc::clone(client),
        None => {
            error!("BaseApiClient introuvable dans le contexte");
            return;
        }
    };
    let grpc = match data.get::<sentinel_shared::grpc_client::GrpcClientKey>() {
        Some(g) => Arc::clone(g),
        None => {
            error!("SentinelGrpcClient introuvable dans le contexte");
            return;
        }
    };
    drop(data);

    let api_client = ApiClient::new(Arc::clone(&base), grpc);

    match api_client.analyze(&request).await {
        Ok(response) => {
            info!(action = ?response.action, reason = ?response.reason, "Reponse du backend");

            // M1 — si le backend ne remonte pas de raison, en generer une depuis
            // les flags (plutot que "Automod" generique).
            let fallback_reason = build_fallback_reason(&request.flags);
            let effective_reason = response
                .reason
                .clone()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or(fallback_reason);

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
                    effective_reason,
                );

                base.send_log(
                    if matches!(response.action, Action::Ban) { "error" } else { "warn" },
                    &guild_id,
                    &log_message,
                );

                // L'ancien send_discord_log est remplace par la carte review
                // (on ne l'envoie plus pour eviter le doublon).
            }

            // Phase 8 — mode review par feature. Chaque type de detection
            // peut etre en mode review (carte moderateur) ou auto (action
            // directe). La config key est `{feature}_review_mode` (defaut
            // true = review pour tout).
            if ai_review_mode && log_channel_id != 0 {
                send_review_card(
                    ctx, msg, &response.action,
                    &effective_reason,
                    response.score.unwrap_or(0.0),
                    &request.flags,
                    log_channel_id, colors,
                ).await;
            } else {
                // Mode auto ou pas de salon review → action directe.
                if let Err(e) = execute_action(ctx, msg, &response.action, Some(effective_reason.as_str()), mute_duration_secs, colors).await {
                    error!(error = %e, "Erreur lors de l'execution de l'action");
                }
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
            // H2 — Mute AVANT delete : si delete echoue le user reste quand meme
            // mute. L'inverse masquait l'echec.
            if let (Some(guild_id), Ok(member)) = (msg.guild_id, msg.member(&ctx.http).await) {
                let mut member = guild_id.member(&ctx.http, member.user.id).await?;
                // M10 — Borne la duree du mute a 28 jours (limite Discord)
                // pour eviter un overflow de timestamp sur l'addition i64.
                const MAX_MUTE_SECS: u64 = 28 * 24 * 3600;
                let safe_duration = mute_duration_secs.min(MAX_MUTE_SECS);
                let secs = match std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                {
                    Ok(d) => match (d.as_secs() as i64).checked_add(safe_duration as i64) {
                        Some(v) => v,
                        None => {
                            error!("Overflow timestamp mute (cas improbable)");
                            return Ok(());
                        }
                    },
                    Err(e) => {
                        error!(error = %e, "Erreur horloge systeme pour le calcul du mute");
                        return Ok(());
                    }
                };
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
            // Delete best-effort apres mute reussi (l'inverse cacherait un
            // echec mute si le delete reussit).
            if let Err(e) = msg.delete(&ctx.http).await {
                warn!(error = %e, message_id = %msg.id, "Echec suppression message apres mute automod");
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

// send_discord_log supprime — remplace par send_review_card (Phase 8).
