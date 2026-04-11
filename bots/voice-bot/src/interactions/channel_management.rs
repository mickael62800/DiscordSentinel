use serenity::builder::{
    CreateActionRow, CreateButton, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateModal, CreateInputText,
    EditChannel,
};
use serenity::model::application::{ButtonStyle, ComponentInteraction, InputTextStyle, ModalInteraction};
use serenity::model::id::ChannelId;
use serenity::model::Permissions;
use serenity::prelude::*;
use tracing::{error, info, warn};

use sentinel_shared::heartbeat::ApiClientKey;

use crate::api_client::{ApiClient, UpdateVoiceChannelRequest};

/// Handle channel management interactions.
pub async fn handle(ctx: &Context, component: &ComponentInteraction) {
    let custom_id = component.data.custom_id.as_str();

    match custom_id {
        "btn_hide" => handle_hide(ctx, component).await,
        "btn_lock" => handle_lock(ctx, component).await,
        "btn_limit" => handle_limit_menu(ctx, component).await,
        "btn_rename" => handle_rename_modal(ctx, component).await,
        "btn_status" => handle_status_modal(ctx, component).await,
        other if other.starts_with("limit_") => handle_limit_select(ctx, component).await,
        _ => {
            warn!(custom_id = %custom_id, "Channel management interaction inconnue");
        }
    }
}

/// Handle modal submissions for rename and status.
pub async fn handle_modal(ctx: &Context, modal: &ModalInteraction) {
    let custom_id = modal.data.custom_id.as_str();

    match custom_id {
        "modal_rename" => handle_modal_rename(ctx, modal).await,
        "modal_status" => handle_modal_status(ctx, modal).await,
        _ => {
            warn!(custom_id = %custom_id, "Channel management modal inconnue");
        }
    }
}

// ── Hide / Show ──

async fn handle_hide(ctx: &Context, component: &ComponentInteraction) {
    let Some((voice_channel_id, ch)) = super::require_admin(ctx, component).await else {
        return;
    };

    let guild_id = component.guild_id.unwrap_or_default();
    let everyone_role = serenity::model::id::RoleId::new(guild_id.get());

    let currently_hidden = ch.visibility == "hidden";
    let new_visibility = if currently_hidden { "visible" } else { "hidden" };

    // Toggle Discord permissions
    if currently_hidden {
        // Make visible: remove the deny on VIEW_CHANNEL for @everyone
        if let Err(e) = voice_channel_id
            .delete_permission(&ctx.http, serenity::model::channel::PermissionOverwriteType::Role(everyone_role))
            .await
        {
            tracing::warn!(error = %e, "failed to delete permission when making channel visible");
        }
    } else {
        // Hide: deny VIEW_CHANNEL for @everyone
        let overwrite = serenity::model::channel::PermissionOverwrite {
            allow: Permissions::empty(),
            deny: Permissions::VIEW_CHANNEL,
            kind: serenity::model::channel::PermissionOverwriteType::Role(everyone_role),
        };
        if let Err(e) = voice_channel_id.create_permission(&ctx.http, overwrite).await {
            tracing::warn!(error = %e, "failed to set permission when hiding channel");
        }
    }

    // Update API
    let update = UpdateVoiceChannelRequest {
        visibility: Some(new_visibility.to_string()),
        locked: None,
        queue_enabled: None,
        name: None,
        status: None,
        member_limit: None,
        queue_channel_id: None,
    };

    {
        let data = ctx.data.read().await;
        let base = data.get::<ApiClientKey>().expect("ApiClient");
        let api = ApiClient::new(base.clone(), data.get::<sentinel_shared::grpc_client::GrpcClientKey>().expect("GrpcClientKey manquant").clone());
        if let Err(e) = api.update_channel(&voice_channel_id.get().to_string(), &update).await {
            error!(error = %e, "Erreur API update visibility");
        }
    }

    let status_text = if currently_hidden {
        "Le salon est maintenant **visible**."
    } else {
        "Le salon est maintenant **cache**."
    };

    super::respond_ephemeral(ctx, component, status_text).await;

    info!(
        voice = %voice_channel_id,
        visibility = %new_visibility,
        "Visibilite changee"
    );
}

// ── Lock / Unlock ──

async fn handle_lock(ctx: &Context, component: &ComponentInteraction) {
    let Some((voice_channel_id, ch)) = super::require_admin(ctx, component).await else {
        return;
    };

    let guild_id = component.guild_id.unwrap_or_default();
    let everyone_role = serenity::model::id::RoleId::new(guild_id.get());

    let currently_locked = ch.locked;
    let new_locked = !currently_locked;

    // Lire les permissions existantes pour @everyone et merger
    let existing_overwrite = voice_channel_id
        .to_channel(&ctx.http).await.ok()
        .and_then(|c| c.guild())
        .and_then(|c| {
            c.permission_overwrites.iter()
                .find(|ow| ow.kind == serenity::model::channel::PermissionOverwriteType::Role(everyone_role))
                .cloned()
        });

    let (base_allow, base_deny) = match &existing_overwrite {
        Some(ow) => (ow.allow, ow.deny),
        None => (Permissions::empty(), Permissions::empty()),
    };

    // Toggle Discord permissions (merger, pas ecraser)
    if currently_locked {
        // Unlock: retirer CONNECT des deny, ajouter aux allow
        let overwrite = serenity::model::channel::PermissionOverwrite {
            allow: base_allow | Permissions::CONNECT,
            deny: base_deny - Permissions::CONNECT,
            kind: serenity::model::channel::PermissionOverwriteType::Role(everyone_role),
        };
        if let Err(e) = voice_channel_id.create_permission(&ctx.http, overwrite).await {
            tracing::warn!(error = %e, "failed to set permission when unlocking channel");
        }
    } else {
        // Lock: ajouter CONNECT aux deny, retirer des allow
        let overwrite = serenity::model::channel::PermissionOverwrite {
            allow: base_allow - Permissions::CONNECT,
            deny: base_deny | Permissions::CONNECT,
            kind: serenity::model::channel::PermissionOverwriteType::Role(everyone_role),
        };
        if let Err(e) = voice_channel_id.create_permission(&ctx.http, overwrite).await {
            tracing::warn!(error = %e, "failed to set permission when locking channel");
        }
    }

    // Update API
    let update = UpdateVoiceChannelRequest {
        visibility: None,
        locked: Some(new_locked),
        queue_enabled: None,
        name: None,
        status: None,
        member_limit: None,
        queue_channel_id: None,
    };

    {
        let data = ctx.data.read().await;
        let base = data.get::<ApiClientKey>().expect("ApiClient");
        let api = ApiClient::new(base.clone(), data.get::<sentinel_shared::grpc_client::GrpcClientKey>().expect("GrpcClientKey manquant").clone());
        if let Err(e) = api.update_channel(&voice_channel_id.get().to_string(), &update).await {
            error!(error = %e, "Erreur API update lock");
        }
    }

    let status_text = if new_locked {
        "Le salon est maintenant **verrouille**. Personne ne peut rejoindre."
    } else {
        "Le salon est maintenant **deverrouille**."
    };

    super::respond_ephemeral(ctx, component, status_text).await;

    info!(voice = %voice_channel_id, locked = new_locked, "Lock change");
}

// ── Limit ──

async fn handle_limit_menu(ctx: &Context, component: &ComponentInteraction) {
    let Some((_voice_channel_id, _ch)) = super::require_admin(ctx, component).await else {
        return;
    };

    // Show a row of buttons for common limits
    let embed = CreateEmbed::new()
        .title("Limite de membres")
        .description("Choisissez une limite de membres pour votre salon vocal.")
        .color(0x3498db);

    let row1 = CreateActionRow::Buttons(vec![
        CreateButton::new("limit_0")
            .label("Aucune")
            .style(ButtonStyle::Secondary),
        CreateButton::new("limit_2")
            .label("2")
            .style(ButtonStyle::Primary),
        CreateButton::new("limit_5")
            .label("5")
            .style(ButtonStyle::Primary),
        CreateButton::new("limit_10")
            .label("10")
            .style(ButtonStyle::Primary),
        CreateButton::new("limit_25")
            .label("25")
            .style(ButtonStyle::Primary),
    ]);

    let msg = CreateInteractionResponseMessage::new()
        .embed(embed)
        .components(vec![row1])
        .ephemeral(true);

    let response = CreateInteractionResponse::Message(msg);
    if let Err(e) = component.create_response(&ctx.http, response).await {
        warn!(error = %e, "Erreur envoi menu limite");
    }
}

async fn handle_limit_select(ctx: &Context, component: &ComponentInteraction) {
    let custom_id = component.data.custom_id.as_str();
    let limit_str = custom_id.strip_prefix("limit_").unwrap_or("0");
    let limit: i32 = limit_str.parse().unwrap_or(0);

    // We need the voice channel ID - find it from the text panel
    let text_channel_id = component.channel_id;
    let voice_channel_id = if let Some(vc) = super::find_voice_from_text(ctx, text_channel_id).await {
        vc
    } else if let Some(vc) = super::find_voice_from_members(ctx, text_channel_id).await {
        vc
    } else {
        super::respond_ephemeral(ctx, component, "Impossible de trouver le salon vocal associe.").await;
        return;
    };

    // Set the user limit on the Discord voice channel
    let member_limit = if limit == 0 { None } else { Some(limit) };

    let edit = if let Some(lim) = member_limit {
        EditChannel::new().user_limit(lim as u32)
    } else {
        EditChannel::new().user_limit(0)
    };

    if let Err(e) = voice_channel_id.edit(&ctx.http, edit).await {
        error!(error = %e, "Erreur modification limite Discord");
        super::respond_ephemeral(ctx, component, "Erreur lors de la modification de la limite.").await;
        return;
    }

    // Update API
    let update = UpdateVoiceChannelRequest {
        visibility: None,
        locked: None,
        queue_enabled: None,
        name: None,
        status: None,
        member_limit: Some(member_limit),
        queue_channel_id: None,
    };

    {
        let data = ctx.data.read().await;
        let base = data.get::<ApiClientKey>().expect("ApiClient");
        let api = ApiClient::new(base.clone(), data.get::<sentinel_shared::grpc_client::GrpcClientKey>().expect("GrpcClientKey manquant").clone());
        if let Err(e) = api.update_channel(&voice_channel_id.get().to_string(), &update).await {
            error!(error = %e, "Erreur API update limit");
        }
    }

    let limit_text = if limit == 0 {
        "La limite de membres a ete **supprimee**.".to_string()
    } else {
        format!("La limite a ete definie a **{limit}** membres.")
    };

    super::respond_ephemeral(ctx, component, &limit_text).await;

    info!(voice = %voice_channel_id, limit = limit, "Limite changee");
}

// ── Rename (opens modal) ──

async fn handle_rename_modal(ctx: &Context, component: &ComponentInteraction) {
    let text_channel_id = component.channel_id;

    // Verify the user is admin before showing the modal
    let voice_channel_id = if let Some(vc) = super::find_voice_from_text(ctx, text_channel_id).await {
        vc
    } else {
        super::respond_ephemeral(ctx, component, "Ce salon n'est pas lie a un salon vocal temporaire.").await;
        return;
    };

    // Check ownership via API
    let is_owner = {
        let data = ctx.data.read().await;
        let base = data.get::<ApiClientKey>().expect("ApiClient");
        let api = ApiClient::new(base.clone(), data.get::<sentinel_shared::grpc_client::GrpcClientKey>().expect("GrpcClientKey manquant").clone());
        match api.get_channel(&voice_channel_id.get().to_string()).await {
            Ok(Some(ch)) => ch.owner_id == component.user.id.get().to_string(),
            _ => false,
        }
    };

    if !is_owner {
        super::respond_ephemeral(ctx, component, "Seul le proprietaire peut renommer le salon.").await;
        return;
    }

    let modal = CreateModal::new("modal_rename", "Renommer le salon").components(vec![
        CreateActionRow::InputText(
            CreateInputText::new(InputTextStyle::Short, "Nouveau nom", "rename_input")
                .placeholder("Entrez le nouveau nom du salon")
                .min_length(1)
                .max_length(100)
                .required(true),
        ),
    ]);

    let response = CreateInteractionResponse::Modal(modal);
    if let Err(e) = component.create_response(&ctx.http, response).await {
        warn!(error = %e, "Erreur ouverture modal rename");
    }
}

// ── Status (opens modal) ──

async fn handle_status_modal(ctx: &Context, component: &ComponentInteraction) {
    let text_channel_id = component.channel_id;

    let voice_channel_id = if let Some(vc) = super::find_voice_from_text(ctx, text_channel_id).await {
        vc
    } else {
        super::respond_ephemeral(ctx, component, "Ce salon n'est pas lie a un salon vocal temporaire.").await;
        return;
    };

    let is_owner = {
        let data = ctx.data.read().await;
        let base = data.get::<ApiClientKey>().expect("ApiClient");
        let api = ApiClient::new(base.clone(), data.get::<sentinel_shared::grpc_client::GrpcClientKey>().expect("GrpcClientKey manquant").clone());
        match api.get_channel(&voice_channel_id.get().to_string()).await {
            Ok(Some(ch)) => ch.owner_id == component.user.id.get().to_string(),
            _ => false,
        }
    };

    if !is_owner {
        super::respond_ephemeral(ctx, component, "Seul le proprietaire peut changer le statut.").await;
        return;
    }

    let modal = CreateModal::new("modal_status", "Statut du salon").components(vec![
        CreateActionRow::InputText(
            CreateInputText::new(InputTextStyle::Short, "Statut", "status_input")
                .placeholder("Entrez un statut (laissez vide pour supprimer)")
                .max_length(128)
                .required(false),
        ),
    ]);

    let response = CreateInteractionResponse::Modal(modal);
    if let Err(e) = component.create_response(&ctx.http, response).await {
        warn!(error = %e, "Erreur ouverture modal status");
    }
}

// ── Modal handlers ──

async fn handle_modal_rename(ctx: &Context, modal: &ModalInteraction) {
    let text_channel_id = modal.channel_id;

    let voice_channel_id = if let Some(vc) = super::find_voice_from_text(ctx, text_channel_id).await {
        vc
    } else {
        super::respond_ephemeral_modal(ctx, modal, "Impossible de trouver le salon vocal.").await;
        return;
    };

    // Extract the input value
    let new_name = modal
        .data
        .components
        .first()
        .and_then(|row| row.components.first())
        .and_then(|c| match c {
            serenity::model::application::ActionRowComponent::InputText(input) => {
                input.value.clone()
            }
            _ => None,
        });

    let Some(new_name) = new_name else {
        super::respond_ephemeral_modal(ctx, modal, "Aucun nom fourni.").await;
        return;
    };

    let new_name = new_name.trim().to_string();
    if new_name.is_empty() {
        super::respond_ephemeral_modal(ctx, modal, "Le nom ne peut pas etre vide.").await;
        return;
    }

    // Rename the Discord voice channel
    let edit = EditChannel::new().name(&new_name);
    if let Err(e) = voice_channel_id.edit(&ctx.http, edit).await {
        error!(error = %e, "Erreur rename Discord");
        super::respond_ephemeral_modal(ctx, modal, "Erreur lors du renommage.").await;
        return;
    }

    // Also rename the category if it exists
    {
        let data = ctx.data.read().await;
        let base = data.get::<ApiClientKey>().expect("ApiClient");
        let api = ApiClient::new(base.clone(), data.get::<sentinel_shared::grpc_client::GrpcClientKey>().expect("GrpcClientKey manquant").clone());
        if let Ok(Some(ch)) = api.get_channel(&voice_channel_id.get().to_string()).await {
            if let Some(cat_id_str) = &ch.category_id {
                if let Ok(cat_id) = cat_id_str.parse::<u64>() {
                    let cat_edit = EditChannel::new().name(&new_name);
                    if let Err(e) = ChannelId::new(cat_id).edit(&ctx.http, cat_edit).await {
                        tracing::warn!(error = %e, "failed to rename category");
                    }
                }
            }
        }
    }

    // Update API
    let update = UpdateVoiceChannelRequest {
        visibility: None,
        locked: None,
        queue_enabled: None,
        name: Some(new_name.clone()),
        status: None,
        member_limit: None,
        queue_channel_id: None,
    };

    {
        let data = ctx.data.read().await;
        let base = data.get::<ApiClientKey>().expect("ApiClient");
        let api = ApiClient::new(base.clone(), data.get::<sentinel_shared::grpc_client::GrpcClientKey>().expect("GrpcClientKey manquant").clone());
        if let Err(e) = api.update_channel(&voice_channel_id.get().to_string(), &update).await {
            error!(error = %e, "Erreur API update name");
        }
    }

    super::respond_ephemeral_modal(
        ctx,
        modal,
        &format!("Le salon a ete renomme en **{new_name}**."),
    )
    .await;

    info!(voice = %voice_channel_id, name = %new_name, "Salon renomme");
}

async fn handle_modal_status(ctx: &Context, modal: &ModalInteraction) {
    let text_channel_id = modal.channel_id;

    let voice_channel_id = if let Some(vc) = super::find_voice_from_text(ctx, text_channel_id).await {
        vc
    } else {
        super::respond_ephemeral_modal(ctx, modal, "Impossible de trouver le salon vocal.").await;
        return;
    };

    // Extract the input value
    let new_status = modal
        .data
        .components
        .first()
        .and_then(|row| row.components.first())
        .and_then(|c| match c {
            serenity::model::application::ActionRowComponent::InputText(input) => {
                input.value.clone()
            }
            _ => None,
        });

    let status: Option<String> = new_status
        .map(|s| s.trim().to_string())
        .filter(|s: &String| !s.is_empty());

    // Update API
    let update = UpdateVoiceChannelRequest {
        visibility: None,
        locked: None,
        queue_enabled: None,
        name: None,
        status: Some(status.clone().unwrap_or_default()),
        member_limit: None,
        queue_channel_id: None,
    };

    {
        let data = ctx.data.read().await;
        let base = data.get::<ApiClientKey>().expect("ApiClient");
        let api = ApiClient::new(base.clone(), data.get::<sentinel_shared::grpc_client::GrpcClientKey>().expect("GrpcClientKey manquant").clone());
        if let Err(e) = api.update_channel(&voice_channel_id.get().to_string(), &update).await {
            error!(error = %e, "Erreur API update status");
        }
    }

    let reply = match &status {
        Some(s) => format!("Statut mis a jour : **{s}**"),
        None => "Statut supprime.".to_string(),
    };

    super::respond_ephemeral_modal(ctx, modal, &reply).await;

    info!(voice = %voice_channel_id, status = ?status, "Statut change");
}
