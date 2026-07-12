//! Journal d'événements générique pour les jeux : chaque jeu peut écrire ses
//! événements marquants (combats, vols, fondations, mises…) dans un salon de
//! logs dédié, choisi en config.
//!
//! Opt-in : la clé config `<jeu>_log_channel_id` (type `channel`) est vide par
//! défaut → aucun log. Fail-open et fire-and-forget : un échec d'écriture ne
//! doit jamais casser une commande de jeu.

use serenity::all::{ChannelId, Context, CreateEmbed, CreateMessage, GuildId};
use tracing::debug;

use crate::shared::discord_helpers::guild_config_or_default;

/// Salon de logs d'un jeu, d'après sa config. `None` si non configuré.
async fn log_channel(
    ctx: &Context,
    bot_name: &str,
    log_key: &str,
    gid: GuildId,
) -> Option<ChannelId> {
    let cfg = guild_config_or_default(ctx, &gid.to_string(), bot_name).await;
    let raw = cfg.get(log_key).filter(|s| !s.trim().is_empty())?;
    raw.trim().parse::<u64>().ok().map(ChannelId::new)
}

/// Écrit un événement de jeu dans le salon de logs, si configuré.
///
/// `title` est la nature de l'événement (ex. « Combat », « Organisation »),
/// `description` le détail lisible. Ne renvoie rien : un échec est silencieux
/// (log interne uniquement), le jeu continue.
pub async fn log_event(
    ctx: &Context,
    bot_name: &str,
    log_key: &str,
    gid: GuildId,
    title: &str,
    description: impl Into<String>,
) {
    let Some(channel) = log_channel(ctx, bot_name, log_key, gid).await else {
        return; // pas de salon de logs configuré : rien à faire
    };
    let embed = CreateEmbed::new()
        .title(title)
        .description(description)
        .timestamp(serenity::model::Timestamp::now());
    if let Err(e) = channel
        .send_message(&ctx.http, CreateMessage::new().embed(embed))
        .await
    {
        debug!(guild_id = %gid, game = bot_name, error = %e, "écriture du log de jeu échouée");
    }
}
