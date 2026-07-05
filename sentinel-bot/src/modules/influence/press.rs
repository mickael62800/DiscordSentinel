//! Agence de presse du jeu Influence : publie les actualites (scandales, lois,
//! organisations...) dans un salon dedie via un WEBHOOK, sous une persona
//! configurable (nom + avatar) — bien plus immersif qu'un embed du bot.

use serenity::all::{
    ChannelId, Context, CreateEmbed, CreateEmbedFooter, CreateWebhook, ExecuteWebhook,
};

use crate::shared::heartbeat::ApiClientKey;

/// Nom du webhook cree par le bot dans le salon presse (pour le retrouver).
const WEBHOOK_NAME: &str = "Influence Presse";

/// Publie une actualite dans le salon presse (si configure/active). No-op sinon.
pub async fn publish_news(ctx: &Context, guild_id: &str, title: &str, body: &str) {
    // Config influence-bot.
    let cfg = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(api) => api
                .get_guild_config_for(guild_id, super::MODULE_BOT_NAME)
                .await
                .unwrap_or_default(),
            None => return,
        }
    };
    let enabled = cfg
        .get("press_enabled")
        .map(|v| matches!(v.as_str(), "true" | "1" | "yes" | "on"))
        .unwrap_or(false);
    if !enabled {
        return;
    }
    let channel_id = match cfg.get("press_channel_id").and_then(|v| v.parse::<u64>().ok()) {
        Some(n) if n > 0 => ChannelId::new(n),
        _ => return,
    };
    let name = cfg
        .get("press_name")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "📰 Journal du serveur".to_string());
    let avatar = cfg.get("press_avatar_url").cloned().unwrap_or_default();

    // Retrouve ou cree le webhook du salon.
    let webhook = {
        let existing = channel_id
            .webhooks(&ctx.http)
            .await
            .unwrap_or_default()
            .into_iter()
            .find(|w| w.name.as_deref() == Some(WEBHOOK_NAME));
        match existing {
            Some(w) => w,
            None => match channel_id
                .create_webhook(&ctx.http, CreateWebhook::new(WEBHOOK_NAME))
                .await
            {
                Ok(w) => w,
                Err(e) => {
                    tracing::warn!(error = %e, "Influence presse : echec creation webhook");
                    return;
                }
            },
        }
    };

    let embed = CreateEmbed::new()
        .title(title.chars().take(250).collect::<String>())
        .description(body.chars().take(4000).collect::<String>())
        .color(0x8E44AD)
        .timestamp(serenity::model::Timestamp::now())
        .footer(CreateEmbedFooter::new("Agence de presse — Influence"));

    let mut exec = ExecuteWebhook::new().username(&name).embed(embed);
    if !avatar.trim().is_empty() {
        exec = exec.avatar_url(avatar.trim());
    }
    if let Err(e) = webhook.execute(&ctx.http, false, exec).await {
        tracing::warn!(error = %e, "Influence presse : echec execution webhook");
    }
}
