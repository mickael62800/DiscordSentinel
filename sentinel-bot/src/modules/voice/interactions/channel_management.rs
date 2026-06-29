use serenity::builder::{
    CreateActionRow, CreateInputText, CreateInteractionResponse, CreateModal, EditChannel,
};
use serenity::model::application::{ComponentInteraction, InputTextStyle, ModalInteraction};
use serenity::model::Permissions;
use serenity::prelude::*;
use tracing::{error, info, warn};

use super::api_client::{ApiClient, SavePresetRequest, UpdateVoiceChannelRequest};

/// Handle channel management interactions.
pub async fn handle(ctx: &Context, component: &ComponentInteraction) {
    let custom_id = component.data.custom_id.as_str();

    match custom_id {
        "btn_hide" => handle_hide(ctx, component).await,
        "btn_lock" => handle_lock(ctx, component).await,
        "btn_limit" => handle_limit_modal(ctx, component).await,
        "btn_rename" => handle_rename_modal(ctx, component).await,
        "btn_status" => handle_status_modal(ctx, component).await,
        "btn_save_prefs" => handle_save_prefs(ctx, component).await,
        _ => {
            warn!(custom_id = %custom_id, "Channel management interaction inconnue");
        }
    }
}

/// Handle modal submissions for rename, status and limit.
pub async fn handle_modal(ctx: &Context, modal: &ModalInteraction) {
    let custom_id = modal.data.custom_id.as_str();

    match custom_id {
        "modal_rename" => handle_modal_rename(ctx, modal).await,
        "modal_status" => handle_modal_status(ctx, modal).await,
        "modal_limit" => handle_modal_limit(ctx, modal).await,
        _ => {
            warn!(custom_id = %custom_id, "Channel management modal inconnue");
        }
    }
}

// ── Sauvegarde des parametres (preset par proprietaire) ──

/// Memorise l'etat courant du salon (nom, limite, visibilite, verrou, file)
/// comme preset du proprietaire. Reapplique automatiquement a la prochaine
/// creation d'un salon temporaire par cet utilisateur.
async fn handle_save_prefs(ctx: &Context, component: &ComponentInteraction) {
    super::defer_ephemeral(ctx, component).await;
    let Some((_voice_channel_id, ch)) = super::require_admin_deferred(ctx, component).await else {
        return;
    };

    let guild_id = component.guild_id.unwrap_or_default();

    let request = SavePresetRequest {
        owner_id: ch.owner_id.clone(),
        channel_name: Some(ch.channel_name.clone()),
        member_limit: ch.member_limit,
        visibility: ch.visibility.clone(),
        locked: ch.locked,
        queue_enabled: ch.queue_enabled,
    };

    {
        let data = ctx.data.read().await;
        if let Some(api) = ApiClient::from_data(&data) {
            api.save_preset(&guild_id.get().to_string(), &request).await;
        }
    }

    super::respond_followup_ephemeral(
        ctx,
        component,
        "Parametres sauvegardes. Ils seront reappliques a ton prochain salon (avec ta liste d'amis).",
    )
    .await;

    info!(owner = %ch.owner_id, "Preset salon vocal sauvegarde");
}

// ── Hide / Show ──

async fn handle_hide(ctx: &Context, component: &ComponentInteraction) {
    super::defer_ephemeral(ctx, component).await;
    let Some((voice_channel_id, ch)) = super::require_admin_deferred(ctx, component).await else {
        return;
    };

    let guild_id = component.guild_id.unwrap_or_default();
    let everyone_role = serenity::model::id::RoleId::new(guild_id.get());

    let currently_hidden = ch.visibility == "hidden";
    let new_visibility = if currently_hidden {
        "visible"
    } else {
        "hidden"
    };

    if currently_hidden {
        if let Err(e) = voice_channel_id
            .delete_permission(
                &ctx.http,
                serenity::model::channel::PermissionOverwriteType::Role(everyone_role),
            )
            .await
        {
            tracing::warn!(error = %e, "failed to delete permission when making channel visible");
        }
    } else {
        let overwrite = serenity::model::channel::PermissionOverwrite {
            allow: Permissions::empty(),
            deny: Permissions::VIEW_CHANNEL,
            kind: serenity::model::channel::PermissionOverwriteType::Role(everyone_role),
        };
        if let Err(e) = voice_channel_id
            .create_permission(&ctx.http, overwrite)
            .await
        {
            tracing::warn!(error = %e, "failed to set permission when hiding channel");
        }
    }

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
        let Some(api) = ApiClient::from_data(&data) else {
            error!("ApiClient ou GrpcClient manquants dans TypeMap");
            return;
        };
        if let Err(e) = api
            .update_channel(&voice_channel_id.get().to_string(), &update)
            .await
        {
            error!(error = %e, "Erreur API update visibility");
        }
    }

    let status_text = if currently_hidden {
        "Le salon est maintenant **visible**."
    } else {
        "Le salon est maintenant **cache**."
    };

    super::respond_followup_ephemeral(ctx, component, status_text).await;

    info!(
        voice = %voice_channel_id,
        visibility = %new_visibility,
        "Visibilite changee"
    );
}

// ── Lock / Unlock ──

async fn handle_lock(ctx: &Context, component: &ComponentInteraction) {
    super::defer_ephemeral(ctx, component).await;
    let Some((voice_channel_id, ch)) = super::require_admin_deferred(ctx, component).await else {
        return;
    };

    let guild_id = component.guild_id.unwrap_or_default();
    let everyone_role = serenity::model::id::RoleId::new(guild_id.get());

    let currently_locked = ch.locked;
    let new_locked = !currently_locked;

    let existing_overwrite = voice_channel_id
        .to_channel(&ctx.http)
        .await
        .ok()
        .and_then(|c| c.guild())
        .and_then(|c| {
            c.permission_overwrites
                .iter()
                .find(|ow| {
                    ow.kind
                        == serenity::model::channel::PermissionOverwriteType::Role(everyone_role)
                })
                .cloned()
        });

    let (base_allow, base_deny) = match &existing_overwrite {
        Some(ow) => (ow.allow, ow.deny),
        None => (Permissions::empty(), Permissions::empty()),
    };

    if currently_locked {
        let overwrite = serenity::model::channel::PermissionOverwrite {
            allow: base_allow | Permissions::CONNECT,
            deny: base_deny - Permissions::CONNECT,
            kind: serenity::model::channel::PermissionOverwriteType::Role(everyone_role),
        };
        if let Err(e) = voice_channel_id
            .create_permission(&ctx.http, overwrite)
            .await
        {
            tracing::warn!(error = %e, "failed to set permission when unlocking channel");
        }
    } else {
        let current_members: Vec<serenity::model::id::UserId> = ctx
            .cache
            .guild(guild_id)
            .map(|g| {
                g.voice_states
                    .values()
                    .filter(|vs| vs.channel_id == Some(voice_channel_id))
                    .map(|vs| vs.user_id)
                    .collect()
            })
            .unwrap_or_default();

        for user_id in current_members {
            let existing_user = voice_channel_id
                .to_channel(&ctx.http)
                .await
                .ok()
                .and_then(|c| c.guild())
                .and_then(|c| {
                    c.permission_overwrites
                        .iter()
                        .find(|ow| {
                            ow.kind
                                == serenity::model::channel::PermissionOverwriteType::Member(
                                    user_id,
                                )
                        })
                        .cloned()
                });
            let (u_allow, u_deny) = match existing_user {
                Some(ow) => (ow.allow, ow.deny),
                None => (Permissions::empty(), Permissions::empty()),
            };
            let overwrite = serenity::model::channel::PermissionOverwrite {
                allow: u_allow
                    | Permissions::VIEW_CHANNEL
                    | Permissions::CONNECT
                    | Permissions::SPEAK,
                deny: u_deny - Permissions::CONNECT,
                kind: serenity::model::channel::PermissionOverwriteType::Member(user_id),
            };
            if let Err(e) = voice_channel_id
                .create_permission(&ctx.http, overwrite)
                .await
            {
                tracing::warn!(error = %e, user = %user_id, "failed to whitelist member on lock");
            }
        }

        let overwrite = serenity::model::channel::PermissionOverwrite {
            allow: base_allow - Permissions::CONNECT,
            deny: base_deny | Permissions::CONNECT,
            kind: serenity::model::channel::PermissionOverwriteType::Role(everyone_role),
        };
        if let Err(e) = voice_channel_id
            .create_permission(&ctx.http, overwrite)
            .await
        {
            tracing::warn!(error = %e, "failed to set permission when locking channel");
        }
    }

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
        let Some(api) = ApiClient::from_data(&data) else {
            error!("ApiClient ou GrpcClient manquants dans TypeMap");
            return;
        };
        if let Err(e) = api
            .update_channel(&voice_channel_id.get().to_string(), &update)
            .await
        {
            error!(error = %e, "Erreur API update lock");
        }
    }

    let status_text = if new_locked {
        "Le salon est maintenant **verrouille**. Personne ne peut rejoindre."
    } else {
        "Le salon est maintenant **deverrouille**."
    };

    super::respond_followup_ephemeral(ctx, component, status_text).await;

    info!(voice = %voice_channel_id, locked = new_locked, "Lock change");
}

// ── Limit (modal free-form) ──

async fn handle_limit_modal(ctx: &Context, component: &ComponentInteraction) {
    if super::require_admin(ctx, component).await.is_none() {
        return;
    }

    let modal = CreateModal::new("modal_limit", "Limite de membres").components(vec![
        CreateActionRow::InputText(
            CreateInputText::new(
                InputTextStyle::Short,
                "Nombre (0 = aucune limite)",
                "limit_input",
            )
            .placeholder("Ex: 8 -- laisser 0 pour supprimer la limite")
            .min_length(1)
            .max_length(3)
            .required(true),
        ),
    ]);

    let response = CreateInteractionResponse::Modal(modal);
    if let Err(e) = component.create_response(&ctx.http, response).await {
        warn!(error = %e, "Erreur ouverture modal limite");
    }
}

/// SECURITE : re-verifie que l'auteur de la SOUMISSION du modal est bien le
/// proprietaire du salon. Les custom_id de modal sont statiques donc
/// forgeables : le check fait par le bouton qui ouvre le modal ne suffit pas,
/// un non-proprietaire pourrait soumettre un modal forge pour modifier le
/// salon d'autrui (cf. revue securite).
async fn modal_submitter_is_owner(
    ctx: &Context,
    voice_channel_id: serenity::model::id::ChannelId,
    user_id: serenity::model::id::UserId,
) -> bool {
    let data = ctx.data.read().await;
    let Some(api) = ApiClient::from_data(&data) else {
        return false;
    };
    matches!(
        api.get_channel(&voice_channel_id.get().to_string()).await,
        Ok(Some(ch)) if ch.owner_id == user_id.get().to_string()
    )
}

async fn handle_modal_limit(ctx: &Context, modal: &ModalInteraction) {
    let text_channel_id = modal.channel_id;
    let voice_channel_id = if let Some(vc) = super::find_voice_from_text(ctx, text_channel_id).await
    {
        vc
    } else {
        super::respond_ephemeral_modal(ctx, modal, "Impossible de trouver le salon vocal.").await;
        return;
    };

    if !modal_submitter_is_owner(ctx, voice_channel_id, modal.user.id).await {
        super::respond_ephemeral_modal(ctx, modal, "Seul le proprietaire peut modifier ce salon.")
            .await;
        return;
    }

    let raw = modal
        .data
        .components
        .first()
        .and_then(|row| row.components.first())
        .and_then(|c| match c {
            serenity::model::application::ActionRowComponent::InputText(input) => {
                input.value.clone()
            }
            _ => None,
        })
        .unwrap_or_default();

    let limit: i32 = match raw.trim().parse::<i32>() {
        Ok(n) if (0..=99).contains(&n) => n,
        _ => {
            super::respond_ephemeral_modal(
                ctx,
                modal,
                "Valeur invalide. Entrez un nombre entre 0 et 99 (0 = aucune limite).",
            )
            .await;
            return;
        }
    };

    let edit = EditChannel::new().user_limit(limit as u32);
    if let Err(e) = voice_channel_id.edit(&ctx.http, edit).await {
        error!(error = %e, "Erreur modification limite Discord");
        super::respond_ephemeral_modal(ctx, modal, "Erreur lors de la modification de la limite.")
            .await;
        return;
    }

    let member_limit = if limit == 0 { None } else { Some(limit) };
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
        let Some(api) = ApiClient::from_data(&data) else {
            error!("ApiClient ou GrpcClient manquants dans TypeMap");
            return;
        };
        if let Err(e) = api
            .update_channel(&voice_channel_id.get().to_string(), &update)
            .await
        {
            error!(error = %e, "Erreur API update limit");
        }
    }

    let limit_text = if limit == 0 {
        "La limite de membres a ete **supprimee**.".to_string()
    } else {
        format!("La limite a ete definie a **{limit}** membres.")
    };
    super::respond_ephemeral_modal(ctx, modal, &limit_text).await;
    info!(voice = %voice_channel_id, limit = limit, "Limite changee");
}

// ── Rename (opens modal) ──

async fn handle_rename_modal(ctx: &Context, component: &ComponentInteraction) {
    let text_channel_id = component.channel_id;

    let voice_channel_id = if let Some(vc) = super::find_voice_from_text(ctx, text_channel_id).await
    {
        vc
    } else {
        super::respond_ephemeral(
            ctx,
            component,
            "Ce salon n'est pas lie a un salon vocal temporaire.",
        )
        .await;
        return;
    };

    let is_owner = {
        let data = ctx.data.read().await;
        let Some(api) = ApiClient::from_data(&data) else {
            error!("ApiClient ou GrpcClient manquants dans TypeMap");
            return;
        };
        match api.get_channel(&voice_channel_id.get().to_string()).await {
            Ok(Some(ch)) => ch.owner_id == component.user.id.get().to_string(),
            _ => false,
        }
    };

    if !is_owner {
        super::respond_ephemeral(
            ctx,
            component,
            "Seul le proprietaire peut renommer le salon.",
        )
        .await;
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

    let voice_channel_id = if let Some(vc) = super::find_voice_from_text(ctx, text_channel_id).await
    {
        vc
    } else {
        super::respond_ephemeral(
            ctx,
            component,
            "Ce salon n'est pas lie a un salon vocal temporaire.",
        )
        .await;
        return;
    };

    let is_owner = {
        let data = ctx.data.read().await;
        let Some(api) = ApiClient::from_data(&data) else {
            error!("ApiClient ou GrpcClient manquants dans TypeMap");
            return;
        };
        match api.get_channel(&voice_channel_id.get().to_string()).await {
            Ok(Some(ch)) => ch.owner_id == component.user.id.get().to_string(),
            _ => false,
        }
    };

    if !is_owner {
        super::respond_ephemeral(
            ctx,
            component,
            "Seul le proprietaire peut changer le statut.",
        )
        .await;
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

    let voice_channel_id = if let Some(vc) = super::find_voice_from_text(ctx, text_channel_id).await
    {
        vc
    } else {
        super::respond_ephemeral_modal(ctx, modal, "Impossible de trouver le salon vocal.").await;
        return;
    };

    if !modal_submitter_is_owner(ctx, voice_channel_id, modal.user.id).await {
        super::respond_ephemeral_modal(ctx, modal, "Seul le proprietaire peut modifier ce salon.")
            .await;
        return;
    }

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

    // Renomme directement le salon vocal sur Discord. (L ancien code
    // tentait de renommer la categorie associee — vestige d une version
    // ou chaque user avait sa propre categorie. Aujourd hui les salons
    // temporaires n ont pas de categorie distincte → le rename ne faisait
    // rien et retournait "Pas de categorie associee".)
    let edit = EditChannel::new().name(&new_name);
    if let Err(e) = voice_channel_id.edit(&ctx.http, edit).await {
        error!(error = %e, voice = %voice_channel_id, "Erreur rename salon vocal Discord");
        super::respond_ephemeral_modal(ctx, modal, "Erreur lors du renommage.").await;
        return;
    }

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
        let Some(api) = ApiClient::from_data(&data) else {
            error!("ApiClient ou GrpcClient manquants dans TypeMap");
            return;
        };
        if let Err(e) = api
            .update_channel(&voice_channel_id.get().to_string(), &update)
            .await
        {
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

    let voice_channel_id = if let Some(vc) = super::find_voice_from_text(ctx, text_channel_id).await
    {
        vc
    } else {
        super::respond_ephemeral_modal(ctx, modal, "Impossible de trouver le salon vocal.").await;
        return;
    };

    if !modal_submitter_is_owner(ctx, voice_channel_id, modal.user.id).await {
        super::respond_ephemeral_modal(ctx, modal, "Seul le proprietaire peut modifier ce salon.")
            .await;
        return;
    }

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

    let discord_status = status.clone().unwrap_or_default();
    let edit = EditChannel::new().status(discord_status.as_str());
    if let Err(e) = voice_channel_id.edit(&ctx.http, edit).await {
        error!(error = %e, "Erreur application statut Discord");
        super::respond_ephemeral_modal(
            ctx,
            modal,
            "Erreur lors de l'application du statut sur Discord.",
        )
        .await;
        return;
    }

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
        let Some(api) = ApiClient::from_data(&data) else {
            error!("ApiClient ou GrpcClient manquants dans TypeMap");
            return;
        };
        if let Err(e) = api
            .update_channel(&voice_channel_id.get().to_string(), &update)
            .await
        {
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
