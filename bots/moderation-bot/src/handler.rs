use futures_util::StreamExt;
use serenity::async_trait;
use serenity::builder::{CreateEmbed, CreateEmbedFooter, CreateMessage};
use serenity::model::application::Interaction;
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use tracing::{error, info, warn};

use sentinel_shared::heartbeat::{ApiClientKey, register_guilds};

use crate::api_client::ApiClient;
use crate::commands;

/// Cle TypeMap pour le client API specifique a la moderation.
pub struct ModerationApiKey;
impl TypeMapKey for ModerationApiKey {
    type Value = ApiClient;
}

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, "Moderation bot connecte");

        register_guilds(&ctx, &ready).await;

        if let Err(e) = serenity::model::application::Command::set_global_commands(
            &ctx.http,
            commands::all(),
        )
        .await
        {
            error!(error = %e, "Impossible d'enregistrer les slash commands");
        } else {
            info!("Slash commands enregistrees : warn, mute, unmute, ban, unban, history, note");
        }

        // Ecouter Redis pour les actions de moderation depuis le desktop
        let ctx_clone = ctx.clone();
        tokio::spawn(async move {
            listen_redis_moderation(ctx_clone).await;
        });
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            let cmd_name = command.data.name.clone();
            let moderator = command.user.name.clone();
            let guild_id = command.guild_id.map(|g| g.to_string()).unwrap_or_default();

            match cmd_name.as_str() {
                "warn" => commands::warn::handle(&ctx, &command).await,
                "mute" => commands::mute::handle(&ctx, &command).await,
                "unmute" => commands::mute::handle_unmute(&ctx, &command).await,
                "ban" => commands::ban::handle(&ctx, &command).await,
                "unban" => commands::ban::handle_unban(&ctx, &command).await,
                "history" => commands::history::handle(&ctx, &command).await,
                "note" => commands::notes::handle(&ctx, &command).await,
                _ => {}
            }

            let data = ctx.data.read().await;
            if let Some(api) = data.get::<ApiClientKey>() {
                api.send_log("info", &guild_id, &format!("Commande /{} executee par {}", cmd_name, moderator));
            }
        }
    }
}

/// Ecoute Redis pour poster les actions de moderation depuis le desktop dans Discord.
async fn listen_redis_moderation(ctx: Context) {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let channel_name = std::env::var("REDIS_CHANNEL").unwrap_or_else(|_| "sentinel:events".to_string());

    loop {
        match redis::Client::open(redis_url.as_str()) {
            Ok(client) => {
                match client.get_async_pubsub().await {
                    Ok(mut pubsub) => {
                        if let Err(e) = pubsub.subscribe(&channel_name).await {
                            error!(error = %e, "Erreur abonnement Redis");
                            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                            continue;
                        }

                        info!(channel = %channel_name, "Ecoute Redis pour logs moderation");

                        loop {
                            let msg = pubsub.on_message().next().await;
                            if let Some(msg) = msg {
                                let payload: String = match msg.get_payload() {
                                    Ok(p) => p,
                                    Err(_) => continue,
                                };
                                handle_redis_moderation_event(&ctx, &payload).await;
                            } else {
                                warn!("Connexion Redis perdue, reconnexion...");
                                break;
                            }
                        }
                    }
                    Err(e) => error!(error = %e, "Erreur connexion Redis"),
                }
            }
            Err(e) => error!(error = %e, "Erreur creation client Redis"),
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

/// Traite un event moderation recu depuis Redis et poste un embed dans le salon de logs.
async fn handle_redis_moderation_event(ctx: &Context, payload: &str) {
    let event: serde_json::Value = match serde_json::from_str(payload) {
        Ok(v) => v,
        Err(_) => return,
    };

    let event_type = event.get("event").and_then(|e| e.as_str()).unwrap_or("");
    if event_type != "moderation_action" {
        return;
    }

    let data = match event.get("data") {
        Some(d) => d,
        None => return,
    };

    let action_type = data.get("action_type").and_then(|v| v.as_str()).unwrap_or("");
    let target_id = data.get("target_id").and_then(|v| v.as_str()).unwrap_or("");
    let target_name = data.get("target_name").and_then(|v| v.as_str()).unwrap_or(target_id);
    let moderator = data.get("moderator_name").and_then(|v| v.as_str()).unwrap_or("Inconnu");
    let guild_id = data.get("guild_id").and_then(|v| v.as_str()).unwrap_or("");
    let reason = data.get("reason").and_then(|v| v.as_str()).unwrap_or("Aucune raison specifiee");

    if guild_id.is_empty() {
        return;
    }

    // Trouver le salon de logs via la config
    let log_channel_id = {
        let ctx_data = ctx.data.read().await;
        if let Some(base) = ctx_data.get::<ApiClientKey>() {
            let config = base.get_guild_config(guild_id).await.unwrap_or_default();
            config.get("log_channel_id")
                .and_then(|v| v.parse::<u64>().ok())
        } else {
            None
        }
    };

    let log_channel = match log_channel_id {
        Some(id) if id > 0 => ChannelId::new(id),
        _ => return,
    };

    // Construire l'embed (meme style que AutoMod)
    let (icon, action_label, color) = match action_type {
        "ban_permanent" => ("\u{1f6ab}", "Bannissement permanent", 0xdc2626u32),
        "ban_temp" => ("\u{1f6ab}", "Bannissement temporaire", 0xdc2626),
        "ban" => ("\u{1f6ab}", "Bannissement", 0xdc2626),
        "unban" => ("\u{1f513}", "Debannissement", 0x2ecc71),
        "mute" | "mute_temp" => ("\u{1f507}", "Mute", 0xef4444),
        "unmute" => ("\u{1f50a}", "Unmute", 0x2ecc71),
        "warn" => ("\u{26a0}\u{fe0f}", "Avertissement", 0xf59e0b),
        "kick" => ("\u{1f462}", "Expulsion", 0xf97316),
        "delete" => ("\u{1f5d1}\u{fe0f}", "Message supprime", 0xf97316),
        _ => ("\u{1f4cb}", "Action de moderation", 0x95a5a6),
    };

    let source = if moderator == "Desktop App" { "Desktop App" } else { moderator };

    // Recuperer l'avatar et le username reel de la cible
    let (avatar_url, real_name) = if let Ok(uid) = target_id.parse::<u64>() {
        let user_id = serenity::model::id::UserId::new(uid);
        match user_id.to_user(&ctx.http).await {
            Ok(user) => (Some(user.face()), user.name.clone()),
            Err(_) => (None, target_name.to_string()),
        }
    } else {
        (None, target_name.to_string())
    };

    let mut embed = CreateEmbed::new()
        .author(serenity::builder::CreateEmbedAuthor::new(
            format!("{} {}", icon, action_label),
        ))
        .title("Moderation - Action manuelle")
        .color(color)
        .field(
            "\u{1f464} Utilisateur",
            format!("<@{}> (`{}`)", target_id, real_name),
            true,
        )
        .field(
            "\u{1f6e1}\u{fe0f} Moderateur",
            source,
            true,
        )
        .field(
            "\u{2699}\u{fe0f} Action",
            action_label,
            true,
        )
        .field(
            "\u{1f4dd} Raison",
            format!("```{}```", reason),
            false,
        )
        .footer(CreateEmbedFooter::new(
            format!("Cible: {} | Moderateur: {} \u{2022} {}", target_id, source, action_type),
        ))
        .timestamp(serenity::model::Timestamp::now());

    if let Some(url) = avatar_url {
        embed = embed.thumbnail(url);
    }

    let msg = CreateMessage::new().embed(embed);
    if let Err(e) = log_channel.send_message(&ctx.http, msg).await {
        error!(error = %e, "Erreur envoi log moderation dans Discord");
    }
}
