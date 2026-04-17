use serenity::builder::{
    CreateActionRow, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption,
};
use serenity::model::application::ComponentInteraction;
use serenity::model::id::{ChannelId, UserId};
use serenity::model::Permissions;
use serenity::prelude::*;
use tracing::{error, info, warn};

use super::super::api_client::{ApiClient, TransferOwnershipRequest};
use super::super::VoiceOwnerMapKey;

/// Handle transfer interactions.
pub async fn handle(ctx: &Context, component: &ComponentInteraction) {
    let custom_id = component.data.custom_id.as_str();

    match custom_id {
        "btn_transfer" => handle_transfer_menu(ctx, component).await,
        "select_transfer" => handle_transfer_select(ctx, component).await,
        _ => {
            warn!(custom_id = %custom_id, "Transfer interaction inconnue");
        }
    }
}

async fn handle_transfer_menu(ctx: &Context, component: &ComponentInteraction) {
    let Some((voice_channel_id, _ch)) = super::require_admin(ctx, component).await else {
        return;
    };

    let guild_id = component.guild_id.unwrap_or_default();
    let owner_id = component.user.id;

    let members = get_voice_members(ctx, guild_id, voice_channel_id, Some(owner_id)).await;

    if members.is_empty() {
        super::respond_ephemeral(ctx, component, "Aucun membre disponible pour le transfert.").await;
        return;
    }

    let options: Vec<CreateSelectMenuOption> = members
        .iter()
        .map(|(id, name)| CreateSelectMenuOption::new(name, id.get().to_string()))
        .collect();

    let select = CreateSelectMenu::new(
        "select_transfer",
        CreateSelectMenuKind::String { options },
    )
    .placeholder("Choisissez le nouveau proprietaire");

    let row = CreateActionRow::SelectMenu(select);

    let msg = CreateInteractionResponseMessage::new()
        .content("A qui souhaitez-vous transferer la propriete du salon ?")
        .components(vec![row])
        .ephemeral(true);

    let response = CreateInteractionResponse::Message(msg);
    if let Err(e) = component.create_response(&ctx.http, response).await {
        warn!(error = %e, "Erreur envoi menu transfer");
    }
}

async fn handle_transfer_select(ctx: &Context, component: &ComponentInteraction) {
    let text_channel_id = component.channel_id;

    let voice_channel_id = if let Some(vc) = super::find_voice_from_text(ctx, text_channel_id).await {
        vc
    } else {
        super::respond_ephemeral(ctx, component, "Impossible de trouver le salon vocal associe.").await;
        return;
    };

    let ch = {
        let data = ctx.data.read().await;
        let Some(api) = ApiClient::from_data(&data) else {
            error!("ApiClient ou GrpcClient manquants dans TypeMap");
            return;
        };
        match api.get_channel(&voice_channel_id.get().to_string()).await {
            Ok(Some(ch)) => ch,
            _ => {
                super::respond_ephemeral(ctx, component, "Salon introuvable.").await;
                return;
            }
        }
    };

    let old_owner_id = component.user.id;
    if ch.owner_id != old_owner_id.get().to_string() {
        super::respond_ephemeral(ctx, component, "Seul le proprietaire peut transferer le salon.").await;
        return;
    }

    let selected_value = match &component.data.kind {
        serenity::model::application::ComponentInteractionDataKind::StringSelect { values } => {
            match values.first() {
                Some(v) => v.clone(),
                None => {
                    super::respond_ephemeral(ctx, component, "Aucun membre selectionne.").await;
                    return;
                }
            }
        }
        _ => {
            super::respond_ephemeral(ctx, component, "Selection invalide.").await;
            return;
        }
    };

    let new_owner_id: u64 = match selected_value.parse() {
        Ok(id) => id,
        Err(_) => {
            super::respond_ephemeral(ctx, component, "Selection invalide.").await;
            return;
        }
    };

    let new_owner_user_id = UserId::new(new_owner_id);

    let new_owner_name = new_owner_user_id
        .to_user(&ctx.http)
        .await
        .map(|u| u.name.clone())
        .unwrap_or_else(|_| new_owner_id.to_string());

    let request = TransferOwnershipRequest {
        new_owner_id: new_owner_id.to_string(),
        new_owner_name: new_owner_name.clone(),
    };

    {
        let data = ctx.data.read().await;
        let Some(api) = ApiClient::from_data(&data) else {
            error!("ApiClient ou GrpcClient manquants dans TypeMap");
            return;
        };
        if let Err(e) = api
            .transfer_ownership(&voice_channel_id.get().to_string(), &request)
            .await
        {
            error!(error = %e, "Erreur API transfer ownership -- abort");
            super::respond_ephemeral(
                ctx, component,
                "Echec du transfert cote serveur. Aucune modification appliquee.",
            ).await;
            return;
        }

        if let Some(map) = data.get::<VoiceOwnerMapKey>() {
            map.insert(voice_channel_id, new_owner_user_id);
        }
    }

    let new_owner_overwrite = serenity::model::channel::PermissionOverwrite {
        allow: Permissions::VIEW_CHANNEL
            | Permissions::CONNECT
            | Permissions::SPEAK
            | Permissions::MANAGE_CHANNELS,
        deny: Permissions::empty(),
        kind: serenity::model::channel::PermissionOverwriteType::Member(new_owner_user_id),
    };
    if let Err(e) = voice_channel_id
        .create_permission(&ctx.http, new_owner_overwrite)
        .await
    {
        tracing::warn!(error = %e, "failed to grant new owner permission on voice channel");
    }

    let old_owner_overwrite = serenity::model::channel::PermissionOverwrite {
        allow: Permissions::VIEW_CHANNEL | Permissions::CONNECT | Permissions::SPEAK,
        deny: Permissions::empty(),
        kind: serenity::model::channel::PermissionOverwriteType::Member(old_owner_id),
    };
    if let Err(e) = voice_channel_id
        .create_permission(&ctx.http, old_owner_overwrite)
        .await
    {
        tracing::warn!(error = %e, "failed to downgrade old owner permission on voice channel");
    }

    if let Some(ref text_id_str) = ch.text_channel_id {
        if let Ok(text_id) = text_id_str.parse::<u64>() {
            let text_channel = ChannelId::new(text_id);

            let new_text_overwrite = serenity::model::channel::PermissionOverwrite {
                allow: Permissions::VIEW_CHANNEL
                    | Permissions::SEND_MESSAGES
                    | Permissions::READ_MESSAGE_HISTORY,
                deny: Permissions::empty(),
                kind: serenity::model::channel::PermissionOverwriteType::Member(new_owner_user_id),
            };
            if let Err(e) = text_channel
                .create_permission(&ctx.http, new_text_overwrite)
                .await
            {
                tracing::warn!(error = %e, "failed to grant new owner permission on admin text panel");
            }

            if let Err(e) = text_channel
                .delete_permission(
                    &ctx.http,
                    serenity::model::channel::PermissionOverwriteType::Member(old_owner_id),
                )
                .await
            {
                tracing::warn!(error = %e, "failed to remove old owner permission from admin text panel");
            }
        }
    }

    super::respond_ephemeral(
        ctx,
        component,
        &format!(
            "La propriete du salon a ete transferee a <@{new_owner_id}>."
        ),
    )
    .await;

    info!(
        voice = %voice_channel_id,
        old_owner = %old_owner_id,
        new_owner = %new_owner_user_id,
        "Propriete transferee"
    );
}

async fn get_voice_members(
    ctx: &Context,
    guild_id: serenity::model::id::GuildId,
    voice_channel_id: ChannelId,
    exclude: Option<UserId>,
) -> Vec<(UserId, String)> {
    let mut members = Vec::new();

    let guild = match ctx.cache.guild(guild_id) {
        Some(g) => g.clone(),
        None => return members,
    };

    for (user_id, voice_state) in &guild.voice_states {
        if voice_state.channel_id == Some(voice_channel_id) {
            if let Some(exc) = exclude {
                if *user_id == exc {
                    continue;
                }
            }

            let name = user_id
                .to_user(&ctx.http)
                .await
                .map(|u| u.name.clone())
                .unwrap_or_else(|_| user_id.get().to_string());

            members.push((*user_id, name));
        }
    }

    members
}
