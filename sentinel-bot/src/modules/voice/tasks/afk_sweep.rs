use serenity::model::id::ChannelId;
use serenity::prelude::*;
use tracing::{info, warn};

use crate::shared::api_client::BaseApiClient;
use crate::shared::heartbeat::ApiClientKey;

use super::embeds;
use super::{AfkTrackerKey, VoiceOwnerMapKey};

pub fn spawn_afk_sweep(ctx: Context) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            run_afk_sweep(&ctx).await;
        }
    });
}

async fn run_afk_sweep(ctx: &Context) {
    let data = ctx.data.read().await;

    let base = match data.get::<ApiClientKey>() {
        Some(b) => b,
        None => return,
    };

    let afk_tracker = match data.get::<AfkTrackerKey>() {
        Some(t) => t,
        None => return,
    };

    let voice_owner_map = match data.get::<VoiceOwnerMapKey>() {
        Some(m) => m,
        None => return,
    };

    // Cloner pour liberer le lock
    let afk_tracker = afk_tracker.clone();
    let voice_owner_map = voice_owner_map.clone();
    let base = base.clone();
    drop(data);

    for guild_id in ctx.cache.guilds() {
        let guild_config = match base.get_guild_config_for(&guild_id.to_string(), crate::modules::voice::MODULE_BOT_NAME).await {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::warn!(error = %e, guild_id = %guild_id, "Echec chargement config guild");
                std::collections::HashMap::new()
            }
        };

        let afk_enabled = BaseApiClient::config_bool(&guild_config, "afk_enabled", false);
        if !afk_enabled {
            continue;
        }

        let afk_channel_id = BaseApiClient::config_u64(&guild_config, "afk_channel_id", 0);
        if afk_channel_id == 0 {
            continue;
        }

        let afk_timeout_minutes = BaseApiClient::config_u64(&guild_config, "afk_timeout_minutes", 10);
        let afk_move_owner = BaseApiClient::config_bool(&guild_config, "afk_move_owner", false);
        let afk_channel = ChannelId::new(afk_channel_id);

        let timeout_secs = afk_timeout_minutes * 60;

        for (user_id, since) in afk_tracker.afk_users() {
            let elapsed_secs = since.elapsed().as_secs();
            if elapsed_secs < timeout_secs {
                continue;
            }

            // Trouver le salon vocal actuel de l'utilisateur
            let current_channel = if let Some(guild) = ctx.cache.guild(guild_id) {
                guild.voice_states
                    .get(&user_id)
                    .and_then(|vs| vs.channel_id)
            } else {
                continue;
            };

            let current_channel = match current_channel {
                Some(ch) => ch,
                None => {
                    afk_tracker.clear(user_id);
                    continue;
                }
            };

            // Ne deplacer que depuis les salons temporaires
            if !voice_owner_map.contains_key(&current_channel) {
                afk_tracker.clear(user_id);
                continue;
            }

            // Ne pas deplacer le proprietaire si desactive
            if !afk_move_owner {
                if let Some(owner) = voice_owner_map.get(&current_channel) {
                    if *owner.value() == user_id {
                        continue;
                    }
                }
            }

            // Ne pas deplacer vers le meme salon
            if current_channel == afk_channel {
                afk_tracker.clear(user_id);
                continue;
            }

            // Deplacer l'utilisateur
            match guild_id.move_member(&ctx.http, user_id, afk_channel).await {
                Ok(_) => {
                    let from_name = embeds::get_channel_name(ctx, current_channel).await;
                    let to_name = embeds::get_channel_name(ctx, afk_channel).await;
                    let afk_minutes = elapsed_secs / 60;

                    embeds::log_afk_move(ctx, user_id.get(), &from_name, &to_name, afk_minutes).await;
                    afk_tracker.clear(user_id);

                    info!(
                        user_id = %user_id,
                        from = %from_name,
                        afk_minutes = %afk_minutes,
                        "Utilisateur AFK deplace automatiquement"
                    );
                }
                Err(e) => {
                    warn!(error = %e, user_id = %user_id, "Impossible de deplacer l'utilisateur AFK");
                    afk_tracker.clear(user_id);
                }
            }
        }
    }
}
