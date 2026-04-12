pub mod appeal;
pub mod ban;
pub mod call;
pub mod compare;
pub mod context;
pub mod evidence;
pub mod expirations;
pub mod export;
pub mod history;
pub mod mass;
pub mod modstats;
pub mod mute;
pub mod notes;
pub mod review;
pub mod template;
pub mod transcript;
pub mod unwarn;
pub mod warn;

use serenity::all::{ChannelId, Context, CreateEmbed, CreateEmbedFooter, CreateMessage};
use serenity::builder::CreateCommand;
use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::heartbeat::ApiClientKey;

/// Envoie un embed de log dans le salon de logs configure pour la guild.
/// Si `log_channel_id` n'est pas configure, ne fait rien.
pub async fn log_to_channel(ctx: &Context, guild_id: &str, embed: CreateEmbed) {
    let log_channel_id = {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            let config = base.get_guild_config(guild_id).await.unwrap_or_default();
            config
                .get("log_channel_id")
                .and_then(|v| v.parse::<u64>().ok())
        } else {
            None
        }
    };

    let channel = match log_channel_id {
        Some(id) if id > 0 => ChannelId::new(id),
        _ => return,
    };

    if let Err(e) = channel
        .send_message(&ctx.http, CreateMessage::new().embed(embed))
        .await
    {
        tracing::warn!(error = %e, "Echec envoi log dans le salon de logs moderation");
    }
}

pub fn all() -> Vec<CreateCommand> {
    vec![
        warn::register(),
        mute::register(),
        mute::register_unmute(),
        ban::register(),
        ban::register_unban(),
        history::register(),
        notes::register(),
        call::register(),
        context::register(),
        appeal::register(),
        export::register(),
        expirations::register(),
        compare::register(),
        modstats::register(),
        evidence::register(),
        review::register(),
        template::register(),
        transcript::register(),
        mass::register_massmute(),
        mass::register_massban(),
        unwarn::register(),
    ]
}
