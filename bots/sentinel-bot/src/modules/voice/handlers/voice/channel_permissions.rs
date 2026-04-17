//! Gestion des permissions Discord sur le panel membres associe a un salon vocal.

use serenity::model::channel::{PermissionOverwrite, PermissionOverwriteType};
use serenity::model::id::{ChannelId, UserId};
use serenity::model::Permissions;
use serenity::prelude::*;

use crate::modules::voice::{MembersToVoiceMapKey, VoiceOwnerMapKey};

/// Donne a un nouveau participant l'acces au salon texte "panel membres"
/// associe a `voice_channel_id`.
pub async fn grant_members_panel_access(
    ctx: &Context,
    voice_channel_id: ChannelId,
    user_id: UserId,
) {
    let members_channel_id = {
        let data = ctx.data.read().await;
        data.get::<MembersToVoiceMapKey>()
            .and_then(|map| {
                map.iter()
                    .find(|entry| *entry.value() == voice_channel_id)
                    .map(|entry| *entry.key())
            })
    };

    if let Some(mid) = members_channel_id {
        let perm = PermissionOverwrite {
            allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(user_id),
        };
        if let Err(e) = mid.create_permission(&ctx.http, perm).await {
            tracing::warn!(error = %e, "failed to grant members panel access");
        }
    }
}

/// Retire l'acces au panel membres pour un membre qui quitte le vocal.
///
/// Ne retire pas l'acces au proprietaire (il garde le panel ouvert meme quand
/// il n'est pas dans le vocal).
pub async fn revoke_members_panel_access(
    ctx: &Context,
    voice_channel_id: ChannelId,
    user_id: UserId,
) {
    // Ne pas retirer l'acces au owner ou co-admins
    let is_admin = {
        let data = ctx.data.read().await;
        data.get::<VoiceOwnerMapKey>()
            .and_then(|map| map.get(&voice_channel_id))
            .map(|owner| *owner == user_id)
            .unwrap_or(false)
    };

    if is_admin {
        return;
    }

    let members_channel_id = {
        let data = ctx.data.read().await;
        data.get::<MembersToVoiceMapKey>()
            .and_then(|map| {
                map.iter()
                    .find(|entry| *entry.value() == voice_channel_id)
                    .map(|entry| *entry.key())
            })
    };

    if let Some(mid) = members_channel_id {
        if let Err(e) = mid
            .delete_permission(&ctx.http, PermissionOverwriteType::Member(user_id))
            .await
        {
            tracing::warn!(error = %e, "failed to revoke members panel access");
        }
    }
}
