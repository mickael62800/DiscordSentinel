use serenity::builder::{
    CreateActionRow, CreateInteractionResponse, CreateInteractionResponseMessage, CreateSelectMenu,
    CreateSelectMenuKind, CreateSelectMenuOption,
};
use serenity::model::application::ComponentInteraction;
use serenity::model::id::{ChannelId, UserId};
use serenity::model::Permissions;
use serenity::prelude::*;
use tracing::{error, info, warn};

use super::api_client::{AddCoAdminRequest, ApiClient};

/// Handle co-admin interactions: promote/demote.
pub async fn handle(ctx: &Context, component: &ComponentInteraction) {
    let custom_id = component.data.custom_id.as_str();

    match custom_id {
        "btn_coadmin" => handle_coadmin_menu(ctx, component).await,
        "select_coadmin" => handle_coadmin_select(ctx, component).await,
        _ => {
            warn!(custom_id = %custom_id, "Co-admin interaction inconnue");
        }
    }
}

async fn handle_coadmin_menu(ctx: &Context, component: &ComponentInteraction) {
    let Some((voice_channel_id, ch)) = super::require_admin(ctx, component).await else {
        return;
    };

    // OWNER-ONLY : la gestion des co-admins reste reservee au proprietaire,
    // meme si require_admin autorise desormais les co-admins.
    if !super::is_owner(&ch, component.user.id.get()) {
        super::respond_ephemeral(
            ctx,
            component,
            "Seul le proprietaire peut gerer les co-admins.",
        )
        .await;
        return;
    }

    let guild_id = component.guild_id.unwrap_or_default();
    let owner_id = component.user.id;

    let members = get_voice_members(ctx, guild_id, voice_channel_id, Some(owner_id)).await;

    if members.is_empty() {
        super::respond_ephemeral(
            ctx,
            component,
            "Aucun membre disponible pour devenir co-admin.",
        )
        .await;
        return;
    }

    let options: Vec<CreateSelectMenuOption> = members
        .iter()
        .map(|(id, name)| CreateSelectMenuOption::new(name, id.get().to_string()))
        .collect();

    let select = CreateSelectMenu::new("select_coadmin", CreateSelectMenuKind::String { options })
        .placeholder("Choisissez un co-admin");

    let row = CreateActionRow::SelectMenu(select);

    let msg = CreateInteractionResponseMessage::new()
        .content("Choisissez un membre a promouvoir/revoquer comme co-admin :")
        .components(vec![row])
        .ephemeral(true);

    let response = CreateInteractionResponse::Message(msg);
    if let Err(e) = component.create_response(&ctx.http, response).await {
        warn!(error = %e, "Erreur envoi menu coadmin");
    }
}

async fn handle_coadmin_select(ctx: &Context, component: &ComponentInteraction) {
    super::defer_ephemeral(ctx, component).await;
    let text_channel_id = component.channel_id;

    let voice_channel_id = if let Some(vc) = super::find_voice_from_text(ctx, text_channel_id).await
    {
        vc
    } else {
        super::respond_followup_ephemeral(
            ctx,
            component,
            "Impossible de trouver le salon vocal associe.",
        )
        .await;
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
                super::respond_followup_ephemeral(ctx, component, "Salon introuvable.").await;
                return;
            }
        }
    };

    if ch.owner_id != component.user.id.get().to_string() {
        super::respond_followup_ephemeral(
            ctx,
            component,
            "Seul le proprietaire peut gerer les co-admins.",
        )
        .await;
        return;
    }

    let selected_value = match &component.data.kind {
        serenity::model::application::ComponentInteractionDataKind::StringSelect { values } => {
            match values.first() {
                Some(v) => v.clone(),
                None => {
                    super::respond_followup_ephemeral(ctx, component, "Aucun membre selectionne.")
                        .await;
                    return;
                }
            }
        }
        _ => {
            super::respond_followup_ephemeral(ctx, component, "Selection invalide.").await;
            return;
        }
    };

    let target_id: u64 = match selected_value.parse() {
        Ok(id) => id,
        Err(_) => {
            super::respond_followup_ephemeral(ctx, component, "Selection invalide.").await;
            return;
        }
    };

    let target_user_id = UserId::new(target_id);

    let target_name = target_user_id
        .to_user(&ctx.http)
        .await
        .map(|u| u.name.clone())
        .unwrap_or_else(|_| target_id.to_string());

    let overwrite = serenity::model::channel::PermissionOverwrite {
        allow: Permissions::VIEW_CHANNEL
            | Permissions::CONNECT
            | Permissions::SPEAK
            | Permissions::MANAGE_CHANNELS,
        deny: Permissions::empty(),
        kind: serenity::model::channel::PermissionOverwriteType::Member(target_user_id),
    };
    if let Err(e) = voice_channel_id
        .create_permission(&ctx.http, overwrite)
        .await
    {
        error!(error = %e, "Erreur permission coadmin");
    }

    if let Some(ref text_id_str) = ch.text_channel_id {
        if let Ok(text_id) = text_id_str.parse::<u64>() {
            let text_overwrite = serenity::model::channel::PermissionOverwrite {
                allow: Permissions::VIEW_CHANNEL
                    | Permissions::SEND_MESSAGES
                    | Permissions::READ_MESSAGE_HISTORY,
                deny: Permissions::empty(),
                kind: serenity::model::channel::PermissionOverwriteType::Member(target_user_id),
            };
            if let Err(e) = ChannelId::new(text_id)
                .create_permission(&ctx.http, text_overwrite)
                .await
            {
                tracing::warn!(error = %e, "failed to grant co-admin permission on text channel");
            }
        }
    }

    let request = AddCoAdminRequest {
        user_id: target_id.to_string(),
        user_name: target_name.clone(),
    };

    {
        let data = ctx.data.read().await;
        let Some(api) = ApiClient::from_data(&data) else {
            error!("ApiClient ou GrpcClient manquants dans TypeMap");
            return;
        };
        if let Err(e) = api
            .add_co_admin(&voice_channel_id.get().to_string(), &request)
            .await
        {
            error!(error = %e, "Erreur API add coadmin");
        }
    }

    super::respond_followup_ephemeral(
        ctx,
        component,
        &format!("<@{target_id}> est maintenant **co-admin** du salon."),
    )
    .await;

    info!(
        voice = %voice_channel_id,
        target = %target_user_id,
        "Co-admin ajoute"
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
