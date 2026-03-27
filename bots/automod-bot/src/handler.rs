use std::sync::Arc;
use std::time::Instant;

use dashmap::{DashMap, DashSet};
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::{ChannelId, MessageId, UserId};
use serenity::prelude::*;
use tracing::{error, info, warn};

use crate::api_client::{Action, AnalyzeRequest, ApiClient, MessageMetadata};
use crate::detectors;

/// Cle pour acceder a l'ApiClient dans le TypeMap de Serenity.
pub struct ApiClientKey;

impl TypeMapKey for ApiClientKey {
    type Value = ApiClient;
}

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

        let flood_max_messages = ApiClient::config_u64(&config, "flood_max_messages", DEFAULT_FLOOD_MAX_MESSAGES) as usize;
        let flood_window_secs = ApiClient::config_u64(&config, "flood_window_secs", DEFAULT_FLOOD_WINDOW_SECS);
        let mute_duration_secs = ApiClient::config_u64(&config, "mute_duration_secs", DEFAULT_MUTE_DURATION_SECS);

        // Verifier les roles ignores
        let ignored_roles_str = ApiClient::config_or(&config, "ignored_roles", "");
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
            let _ = msg.reply(&ctx.http, format!(
                "<@{}> Merci de ne pas envoyer autant de messages aussi rapidement.",
                msg.author.id
            )).await;

            info!(user = %msg.author.name, "Flood detecte");

            // Envoyer au backend comme spam
            let flags = detectors::DetectionFlags { spam: true, insult: false, link: false };
            send_to_backend(&ctx, &msg, flags, mute_duration_secs).await;
            return;
        }

        // 2. Detection caps (avertissement seulement, pas d'infraction)
        if detectors::spam::detect_caps(content) {
            let _ = msg.reply(&ctx.http, format!(
                "<@{}> Merci d'ecrire normalement sans tout mettre en majuscules.",
                msg.author.id
            )).await;
            info!(user = %msg.author.name, "Caps detecte, avertissement envoye");
            // Pas d'appel backend, juste un avertissement
        }

        // 3. Analyse locale (contenu : spam, insulte, lien)
        let flags = detectors::analyze(content);

        if !flags.spam && !flags.insult && !flags.link {
            return;
        }

        info!(
            guild_id = ?msg.guild_id,
            user = %msg.author.name,
            flags.spam = flags.spam,
            flags.insult = flags.insult,
            flags.link = flags.link,
            "Message flagge"
        );

        send_to_backend(&ctx, &msg, flags, mute_duration_secs).await;
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, "Automod bot connecté");

        // Enregistrer les guilds aupres de l'API
        let data = ctx.data.read().await;
        if let Some(api) = data.get::<ApiClientKey>() {
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
}

/// Envoie le message au backend pour analyse et execute l'action.
async fn send_to_backend(ctx: &Context, msg: &Message, flags: detectors::DetectionFlags, mute_duration_secs: u64) {
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
    let api_client = match data.get::<ApiClientKey>() {
        Some(client) => client,
        None => {
            error!("ApiClient introuvable dans le contexte");
            return;
        }
    };

    match api_client.analyze(&request).await {
        Ok(response) => {
            info!(action = ?response.action, reason = ?response.reason, "Reponse du backend");

            if let Err(e) = execute_action(ctx, msg, &response.action, response.reason.as_deref(), mute_duration_secs).await {
                error!(error = %e, "Erreur lors de l'execution de l'action");
            }
        }
        Err(e) => {
            warn!(error = %e, "Backend injoignable — action locale par defaut");
            if request.flags.insult {
                let _ = msg.reply(&ctx.http, format!(
                    "<@{}> Ton message a ete supprime pour langage inapproprie.",
                    msg.author.id
                )).await;
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
            msg.reply(&ctx.http, format!(
                "<@{}> Avertissement : {reason_text}",
                msg.author.id
            )).await?;
            info!(user = %msg.author.name, "Avertissement envoye");
        }
        Action::Delete => {
            // Avertir AVANT de supprimer
            let _ = msg.channel_id.say(&ctx.http, format!(
                "<@{}> Ton message a ete supprime. Raison : {reason_text}",
                msg.author.id
            )).await;
            msg.delete(&ctx.http).await?;
            info!(message_id = %msg.id, "Message supprime");
        }
        Action::Mute => {
            let mute_minutes = mute_duration_secs / 60;
            let _ = msg.channel_id.say(&ctx.http, format!(
                "<@{}> Tu as ete mute {mute_minutes} minutes. Raison : {reason_text}",
                msg.author.id
            )).await;
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
            if let Some(guild_id) = msg.guild_id {
                let _ = msg.channel_id.say(&ctx.http, format!(
                    "<@{}> Tu as ete banni. Raison : {reason_text}",
                    msg.author.id
                )).await;
                guild_id
                    .ban_with_reason(&ctx.http, msg.author.id, 1, reason_text)
                    .await?;
                info!(user = %msg.author.name, "Utilisateur banni");
            }
        }
    }

    Ok(())
}
