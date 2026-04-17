//! Handler du bouton "Reprendre le salon" poste quand le owner quitte.

use serenity::all::ButtonStyle;
use serenity::model::application::ComponentInteraction;
use serenity::model::id::ChannelId;
use serenity::model::Permissions;
use serenity::prelude::*;
use tracing::{info, warn};

use super::api_client::ApiClient;
use super::{TextToVoiceMapKey, VoiceOwnerMapKey};

/// Gere le clic sur `btn_claim_ownership_{voice_channel_id}`.
pub async fn handle(ctx: &Context, component: &ComponentInteraction) {
    let custom_id = &component.data.custom_id;
    let voice_id_str = custom_id.strip_prefix("btn_claim_ownership_").unwrap_or("");
    let voice_channel_id = match voice_id_str.parse::<u64>() {
        Ok(id) => ChannelId::new(id),
        Err(_) => {
            super::respond_ephemeral(ctx, component, "Erreur interne : ID salon invalide.").await;
            return;
        }
    };

    let user_id = component.user.id;

    let is_in_voice = component
        .guild_id
        .and_then(|gid| ctx.cache.guild(gid))
        .map(|guild| {
            guild
                .voice_states
                .values()
                .any(|vs| vs.channel_id == Some(voice_channel_id) && vs.user_id == user_id)
        })
        .unwrap_or(false);

    if !is_in_voice {
        super::respond_ephemeral(
            ctx,
            component,
            "Tu dois etre dans le salon vocal pour reprendre le controle.",
        )
        .await;
        return;
    }

    let new_owner_name = component
        .member
        .as_ref()
        .map(|m| m.display_name().to_string())
        .unwrap_or_else(|| component.user.name.clone());

    {
        let data = ctx.data.read().await;
        let Some(api) = ApiClient::from_data(&data) else {
            super::respond_ephemeral(ctx, component, "Erreur interne API.").await;
            return;
        };
        let req = super::api_client::TransferOwnershipRequest {
            new_owner_id: user_id.get().to_string(),
            new_owner_name: new_owner_name.clone(),
        };
        if let Err(e) = api
            .transfer_ownership(&voice_channel_id.get().to_string(), &req)
            .await
        {
            warn!(error = %e, "Erreur API claim ownership");
            super::respond_ephemeral(ctx, component, "Echec du transfert cote serveur.").await;
            return;
        }

        if let Some(map) = data.get::<VoiceOwnerMapKey>() {
            map.insert(voice_channel_id, user_id);
        }
    }

    let new_owner_perm = serenity::model::channel::PermissionOverwrite {
        allow: Permissions::VIEW_CHANNEL
            | Permissions::CONNECT
            | Permissions::SPEAK
            | Permissions::MOVE_MEMBERS
            | Permissions::MUTE_MEMBERS
            | Permissions::DEAFEN_MEMBERS
            | Permissions::MANAGE_CHANNELS,
        deny: Permissions::empty(),
        kind: serenity::model::channel::PermissionOverwriteType::Member(user_id),
    };
    if let Err(e) = voice_channel_id
        .create_permission(&ctx.http, new_owner_perm)
        .await
    {
        warn!(error = %e, "failed to grant owner permission on claim");
    }

    let text_channel_id = {
        let data = ctx.data.read().await;
        data.get::<TextToVoiceMapKey>().and_then(|map| {
            map.iter()
                .find(|entry| *entry.value() == voice_channel_id)
                .map(|entry| *entry.key())
        })
    };
    if let Some(tid) = text_channel_id {
        let text_perm = serenity::model::channel::PermissionOverwrite {
            allow: Permissions::VIEW_CHANNEL
                | Permissions::SEND_MESSAGES
                | Permissions::READ_MESSAGE_HISTORY,
            deny: Permissions::empty(),
            kind: serenity::model::channel::PermissionOverwriteType::Member(user_id),
        };
        if let Err(e) = tid.create_permission(&ctx.http, text_perm).await {
            warn!(error = %e, "failed to grant admin panel access on claim");
        }
    }

    let disabled_button = serenity::builder::CreateButton::new("btn_claim_done")
        .label(format!("{new_owner_name} a repris le salon"))
        .style(ButtonStyle::Secondary)
        .disabled(true);

    let embed = serenity::builder::CreateEmbed::new()
        .title("\u{2705} Nouveau proprietaire")
        .description(format!(
            "<@{}> a repris le controle du salon.",
            user_id.get()
        ))
        .color(0x2ECC71)
        .timestamp(serenity::model::Timestamp::now());

    if let Err(e) = component
        .create_response(
            &ctx.http,
            serenity::builder::CreateInteractionResponse::UpdateMessage(
                serenity::builder::CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(vec![serenity::builder::CreateActionRow::Buttons(vec![
                        disabled_button,
                    ])]),
            ),
        )
        .await
    {
        warn!(error = %e, "failed to update claim message");
    }

    info!(
        voice = %voice_channel_id,
        new_owner = %user_id,
        "Ownership reprise via candidature"
    );
}
