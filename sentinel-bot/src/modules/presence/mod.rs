//! Collecte de la presence en direct pour la page membre du site.
//!
//! # La regle qui compte
//!
//! La page membre est PUBLIQUE : n'importe qui sur Internet la consulte. Or
//! « Kalyx est dans #staff » est une information privee. On ne publie donc
//! que les salons ou @everyone a le droit de voir — ce que seul le bot peut
//! determiner, l'API n'ayant aucune vue sur les permissions Discord.
//!
//! Le filtre est FERMANT : en cas de doute (salon introuvable dans le cache,
//! guilde absente), on ne publie pas. Une section vide est sans consequence ;
//! une fuite ne se rattrape pas.

use std::sync::Arc;

use serenity::model::id::GuildId;
use serenity::model::permissions::Permissions;
use serenity::prelude::*;

use crate::shared::api_client::BaseApiClient;
use crate::shared::heartbeat::ApiClientKey;
use crate::shared::presence::{VoiceChannelDto, VoiceMemberDto};

/// Un salon est-il visible par tout le monde ?
///
/// Sur Discord, le role @everyone porte l'identifiant de la guilde. On
/// interroge les permissions de ce role sur le salon : s'il n'a pas
/// `VIEW_CHANNEL`, le salon est reserve, donc hors de la vitrine publique.
fn est_public(guild: &serenity::model::guild::Guild, channel_id: serenity::model::id::ChannelId) -> bool {
    let Some(channel) = guild.channels.get(&channel_id) else {
        // Salon inconnu du cache : on s'abstient plutot que de supposer.
        return false;
    };

    let everyone = serenity::model::id::RoleId::new(guild.id.get());
    let Some(role) = guild.roles.get(&everyone) else {
        return false;
    };

    // Permissions de base du role, puis application des surcharges du salon.
    let mut permissions = role.permissions;
    for surcharge in &channel.permission_overwrites {
        if let serenity::model::channel::PermissionOverwriteType::Role(id) = surcharge.kind {
            if id == everyone {
                permissions = (permissions & !surcharge.deny) | surcharge.allow;
            }
        }
    }

    permissions.contains(Permissions::VIEW_CHANNEL)
}

/// Reconstruit l'instantane vocal complet d'une guilde depuis le cache.
///
/// Instantane complet et non delta : appliquer des deltas supposerait qu'aucun
/// evenement ne se perde, et un seul manque ferait deriver la liste sans
/// jamais se corriger.
fn instantane(guild: &serenity::model::guild::Guild) -> Vec<VoiceChannelDto> {
    let mut par_salon: std::collections::HashMap<
        serenity::model::id::ChannelId,
        Vec<VoiceMemberDto>,
    > = std::collections::HashMap::new();

    for etat in guild.voice_states.values() {
        let Some(channel_id) = etat.channel_id else {
            continue;
        };
        if !est_public(guild, channel_id) {
            continue;
        }

        // Les bots occupent les salons sans y participer : un lecteur de
        // musique afficherait un faux participant.
        let membre = guild.members.get(&etat.user_id);
        if membre.map(|m| m.user.bot).unwrap_or(true) {
            continue;
        }

        let nom = membre
            .map(|m| {
                m.nick
                    .clone()
                    .or_else(|| m.user.global_name.clone())
                    .unwrap_or_else(|| m.user.name.clone())
            })
            .unwrap_or_default();

        par_salon.entry(channel_id).or_default().push(VoiceMemberDto {
            user_id: etat.user_id.to_string(),
            username: nom,
            self_mute: etat.self_mute,
            self_deaf: etat.self_deaf,
            server_mute: etat.mute,
            streaming: etat.self_stream.unwrap_or(false),
            video: etat.self_video,
        });
    }

    par_salon
        .into_iter()
        .filter_map(|(channel_id, members)| {
            let channel = guild.channels.get(&channel_id)?;
            Some(VoiceChannelDto {
                channel_id: channel_id.to_string(),
                channel_name: channel.name.clone(),
                members,
            })
        })
        .collect()
}

/// A appeler sur chaque changement d'etat vocal.
pub async fn on_voice_state_update(ctx: &Context, guild_id: GuildId) {
    let Some(api) = client_api(ctx).await else {
        return;
    };

    // Le cache doit etre lu de facon synchrone : garder une reference a la
    // guilde a travers un `await` bloquerait le cache pour tout le bot.
    let channels = {
        let Some(guild) = ctx.cache.guild(guild_id) else {
            return;
        };
        instantane(&guild)
    };

    api.publish_voice_presence(&guild_id.to_string(), channels);
}

/// A appeler sur chaque message. Enregistre une prise de parole.
pub async fn on_message(ctx: &Context, msg: &serenity::model::channel::Message) {
    let Some(guild_id) = msg.guild_id else {
        return;
    };
    if msg.author.bot {
        return;
    }

    let Some(api) = client_api(ctx).await else {
        return;
    };

    // Bloc synchrone : garder une reference au cache a travers un `await`
    // le bloquerait pour tout le bot.
    let nom_salon = {
        let Some(guild) = ctx.cache.guild(guild_id) else {
            return;
        };
        if !est_public(&guild, msg.channel_id) {
            return;
        }
        match guild.channels.get(&msg.channel_id) {
            Some(c) => c.name.clone(),
            None => return,
        }
    };

    let auteur = msg
        .author_nick(&ctx.http)
        .await
        .or_else(|| msg.author.global_name.clone())
        .unwrap_or_else(|| msg.author.name.clone());

    api.touch_text_presence(
        &guild_id.to_string(),
        &msg.channel_id.to_string(),
        &nom_salon,
        &auteur,
    );
}

async fn client_api(ctx: &Context) -> Option<Arc<BaseApiClient>> {
    let data = ctx.data.read().await;
    data.get::<ApiClientKey>().cloned()
}
