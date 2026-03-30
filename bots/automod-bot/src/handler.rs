use std::sync::Arc;
use std::time::Instant;

use dashmap::{DashMap, DashSet};
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::{ChannelId, MessageId, UserId};
use serenity::prelude::*;
use tracing::{error, info, warn};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::heartbeat::{ApiClientKey, register_guilds};

use crate::api_client::{Action, AnalyzeRequest, ApiClient, MessageMetadata};
use crate::detectors;

use sentinel_shared::embeds::{warn_embed, moderate_embed, danger_embed, critical_embed};

/// Deduplication des messages deja traites
pub struct ProcessedMessagesKey;

impl TypeMapKey for ProcessedMessagesKey {
    type Value = Arc<DashSet<MessageId>>;
}

/// Flood tracker : (channel_id, user_id) -> liste de timestamps
pub struct FloodTrackerKey;

impl TypeMapKey for FloodTrackerKey {
    type Value = Arc<DashMap<(ChannelId, UserId), Vec<Instant>>>;
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
                if !processed.insert(msg.id) {
                    return;
                }
                if processed.len() > 1000 {
                    let to_remove: Vec<_> = processed.iter().take(500).map(|e| *e).collect();
                    for id in to_remove {
                        processed.remove(&id);
                    }
                }
            }
        }

        // Charger la config depuis l'API pour ce guild (une seule fois par message)
        let guild_id = msg.guild_id.map(|id| id.to_string()).unwrap_or_default();
        let config = {
            let data = ctx.data.read().await;
            if let Some(api) = data.get::<ApiClientKey>() {
                api.get_guild_config(&guild_id).await.unwrap_or_default()
            } else {
                std::collections::HashMap::new()
            }
        };

        let flood_max_messages = BaseApiClient::config_u64(&config, "flood_max_messages", DEFAULT_FLOOD_MAX_MESSAGES) as usize;
        let flood_window_secs = BaseApiClient::config_u64(&config, "flood_window_secs", DEFAULT_FLOOD_WINDOW_SECS);
        let mute_duration_secs = BaseApiClient::config_u64(&config, "mute_duration_secs", DEFAULT_MUTE_DURATION_SECS);

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
                timestamps.len() >= flood_max_messages
            } else {
                false
            }
        };

        if is_flood {
            // Clear le compteur
            let data = ctx.data.read().await;
            if let Some(tracker) = data.get::<FloodTrackerKey>() {
                tracker.remove(&(msg.channel_id, msg.author.id));
            }
            drop(data);

            // Avertir + traiter comme spam
            let embed = warn_embed("⚠\u{fe0f} Avertissement AutoMod")
                .field("📝 Raison", "Merci de ne pas envoyer autant de messages aussi rapidement.", false)
                .thumbnail(msg.author.face());
            let builder = serenity::builder::CreateMessage::new().embed(embed);
            let _ = msg.channel_id.send_message(&ctx.http, builder).await;

            info!(user = %msg.author.name, "Flood detecte");

            // Envoyer au backend comme spam
            let flags = detectors::DetectionFlags { spam: true, insult: false, link: false, phishing: false };
            let log_channel_id = BaseApiClient::config_u64(&config, "log_channel_id", 0);
            send_to_backend(&ctx, &msg, flags, mute_duration_secs, log_channel_id).await;
            return;
        }

        // 2. Detection caps (avertissement seulement, pas d'infraction)
        if detectors::spam::detect_caps(content) {
            let embed = warn_embed("⚠\u{fe0f} Avertissement AutoMod")
                .field("📝 Raison", "Merci d'ecrire normalement sans tout mettre en majuscules.", false)
                .thumbnail(msg.author.face());
            let builder = serenity::builder::CreateMessage::new().embed(embed);
            let _ = msg.channel_id.send_message(&ctx.http, builder).await;
            info!(user = %msg.author.name, "Caps detecte, avertissement envoye");
            // Pas d'appel backend, juste un avertissement
        }

        // 3. Analyse locale (contenu : spam, insulte, lien)
        let flags = detectors::analyze(content);

        if !flags.spam && !flags.insult && !flags.link && !flags.phishing {
            return;
        }

        info!(
            guild_id = ?msg.guild_id,
            user = %msg.author.name,
            flags.spam = flags.spam,
            flags.insult = flags.insult,
            flags.link = flags.link,
            flags.phishing = flags.phishing,
            "Message flagge"
        );

        let log_channel_id = BaseApiClient::config_u64(&config, "log_channel_id", 0);
        send_to_backend(&ctx, &msg, flags, mute_duration_secs, log_channel_id).await;
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, "Automod bot connecte");
        register_guilds(&ctx, &ready).await;
    }
}

/// Envoie le message au backend pour analyse et execute l'action.
async fn send_to_backend(ctx: &Context, msg: &Message, flags: detectors::DetectionFlags, mute_duration_secs: u64, log_channel_id: u64) {
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

            // Log l'action dans le journal
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

                // Log vers l'API backend (base de donnees)
                base.send_log(
                    if matches!(response.action, Action::Ban) { "error" } else { "warn" },
                    &guild_id,
                    &log_message,
                );

                // Log vers le salon Discord si configure
                if log_channel_id != 0 {
                    send_discord_log(
                        ctx, msg, &response.action, action_label,
                        response.reason.as_deref().unwrap_or("Automod"),
                        &request.flags, log_channel_id,
                    ).await;
                }
            }

            if let Err(e) = execute_action(ctx, msg, &response.action, response.reason.as_deref(), mute_duration_secs).await {
                error!(error = %e, "Erreur lors de l'execution de l'action");
            }
        }
        Err(e) => {
            warn!(error = %e, "Backend injoignable — action locale par defaut");
            if request.flags.phishing {
                let embed = moderate_embed("🗑\u{fe0f} Message supprime")
                    .field("📝 Raison", "Lien suspect detecte.", false)
                    .thumbnail(msg.author.face());
                let builder = serenity::builder::CreateMessage::new().embed(embed);
                let _ = msg.channel_id.send_message(&ctx.http, builder).await;
                let _ = msg.delete(&ctx.http).await;
            } else if request.flags.insult {
                let embed = moderate_embed("🗑\u{fe0f} Message supprime")
                    .field("📝 Raison", "Langage inapproprie.", false)
                    .thumbnail(msg.author.face());
                let builder = serenity::builder::CreateMessage::new().embed(embed);
                let _ = msg.channel_id.send_message(&ctx.http, builder).await;
                let _ = msg.delete(&ctx.http).await;
            }
        }
    }
}

/// Execute l'action decidee par le backend. Avertit toujours l'utilisateur.
async fn execute_action(
    ctx: &Context,
    msg: &Message,
    action: &Action,
    reason: Option<&str>,
    mute_duration_secs: u64,
) -> Result<(), serenity::Error> {
    let reason_text = reason.unwrap_or("Automod");

    match action {
        Action::None => {}
        Action::Warn => {
            let embed = warn_embed("⚠\u{fe0f} Avertissement AutoMod")
                .field("📝 Raison", reason_text, false)
                .thumbnail(msg.author.face());
            let builder = serenity::builder::CreateMessage::new().embed(embed);
            msg.channel_id.send_message(&ctx.http, builder).await?;
            info!(user = %msg.author.name, "Avertissement envoye");
        }
        Action::Delete => {
            // Avertir AVANT de supprimer
            let content_preview = if msg.content.len() > 200 {
                format!("{}...", &msg.content[..200])
            } else {
                msg.content.clone()
            };
            let embed = moderate_embed("🗑\u{fe0f} Message supprime")
                .field("📝 Raison", reason_text, false)
                .field("📄 Contenu original", format!("```{}```", content_preview), false)
                .thumbnail(msg.author.face());
            let builder = serenity::builder::CreateMessage::new().embed(embed);
            let _ = msg.channel_id.send_message(&ctx.http, builder).await;
            msg.delete(&ctx.http).await?;
            info!(message_id = %msg.id, "Message supprime");
        }
        Action::Mute => {
            let mute_minutes = mute_duration_secs / 60;
            let embed = danger_embed("🔇 Mute automatique")
                .field("📝 Raison", reason_text, false)
                .field("⏱ Duree", format!("{} minutes", mute_minutes), false)
                .thumbnail(msg.author.face());
            let builder = serenity::builder::CreateMessage::new().embed(embed);
            let _ = msg.channel_id.send_message(&ctx.http, builder).await;
            msg.delete(&ctx.http).await?;
            if let (Some(guild_id), Ok(member)) = (msg.guild_id, msg.member(&ctx.http).await) {
                let mut member = guild_id.member(&ctx.http, member.user.id).await?;
                let secs = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64
                    + mute_duration_secs as i64;
                let datetime = time::OffsetDateTime::from_unix_timestamp(secs)
                    .expect("timestamp invalide");
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
                    .field("📝 Raison", reason_text, false)
                    .thumbnail(msg.author.face());
                let builder = serenity::builder::CreateMessage::new().embed(embed);
                let _ = msg.channel_id.send_message(&ctx.http, builder).await;
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
) {
    let channel = serenity::model::id::ChannelId::new(log_channel_id);

    // Icone et couleur selon la severite
    let (icon, color) = match action {
        Action::Warn    => ("\u{26a0}\u{fe0f}", 0xf59e0b),  // Warning jaune
        Action::Delete  => ("\u{1f5d1}\u{fe0f}", 0xf97316), // Corbeille orange
        Action::Mute    => ("\u{1f507}", 0xef4444),          // Mute rouge
        Action::Ban     => ("\u{1f6ab}", 0xdc2626),          // Interdit rouge fonce
        Action::None    => ("\u{2705}", 0x22c55e),           // Check vert
    };

    // Construire la liste des detections
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

    // Tronquer le message si trop long
    let content_preview = if msg.content.len() > 300 {
        format!("{}...", &msg.content[..300])
    } else {
        msg.content.clone()
    };

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
