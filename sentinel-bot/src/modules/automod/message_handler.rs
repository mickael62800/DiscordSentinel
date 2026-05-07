//! Handler du traitement des messages pour automod.
//! Analyse spam / insultes / liens / phishing / flood / caps / unicode / attachments.

use std::time::Instant;

use serenity::model::channel::Message;
use serenity::prelude::*;
use tracing::{info, warn};

use crate::shared::api_client::BaseApiClient;
use crate::shared::embeds::{warn_embed, moderate_embed};
use crate::shared::heartbeat::ApiClientKey;

use super::api_client::Action;
use super::backend::{analyze_message_images, send_to_backend};
use super::config::{apply_night_mode, build_detector_config, build_embed_colors, is_night_mode};
use super::detectors;
use super::review::send_review_card;
use super::{FloodTrackerKey, ProcessedMessagesKey, SlowmodeTrackerKey};

/// Defaults si l'API ne repond pas
const DEFAULT_FLOOD_MAX_MESSAGES: u64 = 5;
const DEFAULT_FLOOD_WINDOW_SECS: u64 = 10;
const DEFAULT_MUTE_DURATION_SECS: u64 = 600;

/// Main automod message handler. Called from the sentinel handler's message event.
/// Analyzes messages for spam, insults, links, phishing, flood, caps, etc.
pub(super) async fn process(ctx: &Context, msg: &Message) {
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
    let config = crate::shared::discord_helpers::guild_config_or_default(ctx, &guild_id, crate::modules::automod::MODULE_BOT_NAME).await;

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
                send_review_card(ctx, msg, &Action::Delete, &reason, 1.0, &flags, log_channel_id, &colors).await;
            } else {
                let embed = moderate_embed("Fichier suspect supprime")
                    .color(colors.delete)
                    .field("Raison", &reason, false)
                    .field("Fichier", &attachment.filename, false)
                    .thumbnail(msg.author.face());
                let builder = serenity::builder::CreateMessage::new().embed(embed);
                if let Err(e) = msg.channel_id.send_message(&ctx.http, builder).await {
                    warn!(error = %e, "Echec envoi notification fichier suspect");
                }
                if let Err(e) = msg.delete(&ctx.http).await {
                    warn!(error = %e, message_id = %msg.id, "Echec suppression message fichier suspect");
                }
            }

            let log_msg = format!("Fichier suspect -- {} : {}", msg.author.name, attachment.filename);
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
                send_review_card(ctx, msg, &Action::Warn, "Flood detecte -- messages envoyes trop rapidement.", 0.9, &flags, log_channel_id, &colors).await;
            } else {
                let embed = warn_embed("Avertissement AutoMod")
                    .color(colors.warn)
                    .field("Raison", "Merci de ne pas envoyer autant de messages aussi rapidement.", false)
                    .thumbnail(msg.author.face());
                let builder = serenity::builder::CreateMessage::new().embed(embed);
                if let Err(e) = msg.channel_id.send_message(&ctx.http, builder).await {
                    warn!(error = %e, "Echec envoi avertissement flood");
                }

                let flags = detectors::DetectionFlags { spam: true, insult: false, link: false, phishing: false };
                let ctx_max_msgs = BaseApiClient::config_u64(&config, "context_max_messages", 3) as u8;
                let ctx_max_chars = BaseApiClient::config_u64(&config, "context_max_chars", 200) as usize;
                let flood_review_min_score: f64 = config
                    .get("review_min_score")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                let ctx_clone = ctx.clone();
                let msg_clone = msg.clone();
                tokio::spawn(async move {
                    let ai_review = true; // flood passe par le backend IA en review
                    send_to_backend(&ctx_clone, &msg_clone, flags, mute_duration_secs, log_channel_id, ai_review, &colors, ctx_max_msgs, ctx_max_chars, flood_review_min_score).await;
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
            send_review_card(ctx, msg, &Action::Warn, "Abus de majuscules detecte.", 0.8, &flags, log_channel_id, &colors).await;
        } else {
            let embed = warn_embed("Avertissement AutoMod")
                .color(colors.warn)
                .field("Raison", "Merci d'ecrire normalement sans tout mettre en majuscules.", false)
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
    let vision_enabled = BaseApiClient::config_bool(&config, "vision_enabled", true);
    let review_min_score: f64 = config
        .get("review_min_score")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    tokio::spawn(async move {
        let ai_review = BaseApiClient::config_bool(&config, "ai_review_mode", true);

        // Analyse texte
        send_to_backend(&ctx_clone, &msg_clone, flags, mute_duration_secs, log_channel_id, ai_review, &colors, context_max_messages, context_max_chars, review_min_score).await;

        // Analyse image : si le message contient des images, les analyser via l'API.
        if vision_enabled {
            analyze_message_images(&ctx_clone, &msg_clone, mute_duration_secs, log_channel_id, &colors).await;
        }
    });
}
