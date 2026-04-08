use serenity::builder::{
    CreateActionRow, CreateButton, CreateChannel, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage,
};
use serenity::model::application::ComponentInteraction;
use serenity::model::channel::ChannelType;
use serenity::model::id::ChannelId;
use serenity::model::Permissions;
use serenity::prelude::*;
use tracing::{error, info, warn};

use sentinel_shared::heartbeat::ApiClientKey;

use crate::api_client::{ApiClient, CreateVoiceChannelRequest};
use crate::handler::{PendingChannelsKey, TextToVoiceMapKey, MembersToVoiceMapKey};

/// Handle setup interactions: toggle hidden, open, cancel.
pub async fn handle(ctx: &Context, component: &ComponentInteraction) {
    let custom_id = component.data.custom_id.as_str();

    match custom_id {
        "btn_toggle_hidden" => handle_toggle_hidden(ctx, component).await,
        "btn_open" => handle_open(ctx, component).await,
        "btn_cancel" => handle_cancel(ctx, component).await,
        _ => {
            warn!(custom_id = %custom_id, "Setup interaction inconnue");
        }
    }
}

async fn handle_toggle_hidden(ctx: &Context, component: &ComponentInteraction) {
    let text_channel_id = component.channel_id;
    let user_id = component.user.id;

    let new_hidden = {
        let data = ctx.data.read().await;
        let pending = match data.get::<PendingChannelsKey>() {
            Some(p) => p,
            None => {
                super::respond_ephemeral(ctx, component, "Erreur interne.").await;
                return;
            }
        };

        if !pending.is_owner(&text_channel_id, user_id) {
            super::respond_ephemeral(ctx, component, "Vous n'etes pas le proprietaire.").await;
            return;
        }

        pending.toggle_hidden(&text_channel_id)
    };

    let Some(hidden) = new_hidden else {
        super::respond_ephemeral(ctx, component, "Aucune configuration en attente trouvee.").await;
        return;
    };

    let status = if hidden { "Cache (invisible)" } else { "Visible" };

    let embed = CreateEmbed::new()
        .title("Configuration du salon prive")
        .description(format!(
            "**Visibilite :** {status}\n\n\
             Cliquez sur les boutons ci-dessous pour configurer votre salon."
        ))
        .color(0x3498db);

    let toggle_label = if hidden { "Rendre visible" } else { "Cacher" };
    let toggle_emoji = if hidden { '\u{1f441}' } else { '\u{1f648}' };

    let buttons = CreateActionRow::Buttons(vec![
        CreateButton::new("btn_toggle_hidden")
            .label(toggle_label)
            .emoji(toggle_emoji)
            .style(serenity::model::application::ButtonStyle::Secondary),
        CreateButton::new("btn_open")
            .label("Ouvrir le salon")
            .emoji('\u{2705}')
            .style(serenity::model::application::ButtonStyle::Success),
        CreateButton::new("btn_cancel")
            .label("Annuler")
            .emoji('\u{274c}')
            .style(serenity::model::application::ButtonStyle::Danger),
    ]);

    let msg = CreateInteractionResponseMessage::new()
        .embed(embed)
        .components(vec![buttons]);

    let response = CreateInteractionResponse::UpdateMessage(msg);
    if let Err(e) = component.create_response(&ctx.http, response).await {
        warn!(error = %e, "Erreur mise a jour toggle hidden");
    }
}

async fn handle_open(ctx: &Context, component: &ComponentInteraction) {
    let text_channel_id = component.channel_id;
    let user_id = component.user.id;

    // Retrieve and remove the pending channel
    let pending = {
        let data = ctx.data.read().await;
        let pending_channels = match data.get::<PendingChannelsKey>() {
            Some(p) => p,
            None => {
                super::respond_ephemeral(ctx, component, "Erreur interne.").await;
                return;
            }
        };

        if !pending_channels.is_owner(&text_channel_id, user_id) {
            super::respond_ephemeral(ctx, component, "Vous n'etes pas le proprietaire.").await;
            return;
        }

        pending_channels.remove(&text_channel_id)
    };

    let Some(pending) = pending else {
        super::respond_ephemeral(ctx, component, "Aucune configuration en attente trouvee.").await;
        return;
    };

    let guild_id = pending.guild_id;
    let hidden = pending.hidden;

    // Acknowledge immediately with a deferred update
    let ack = CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new()
            .embed(
                CreateEmbed::new()
                    .title("Creation en cours...")
                    .description("Votre salon prive est en cours de creation.")
                    .color(0xf39c12),
            )
            .components(vec![]),
    );
    if let Err(e) = component.create_response(&ctx.http, ack).await {
        warn!(error = %e, "Erreur ack creation salon");
        return;
    }

    let channel_name = format!("Salon de {}", component.user.name);

    // Create the category
    let category = CreateChannel::new(&channel_name)
        .kind(ChannelType::Category);
    let category_channel = match guild_id.create_channel(&ctx.http, category).await {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "Erreur creation categorie");
            if let Err(e) = component
                .channel_id
                .say(&ctx.http, "Erreur lors de la creation de la categorie.")
                .await
            {
                tracing::warn!(error = %e, "failed to send category creation error message");
            }
            return;
        }
    };

    let category_id = category_channel.id;

    // Create the voice channel inside the category
    let voice_builder = CreateChannel::new(&channel_name)
        .kind(ChannelType::Voice)
        .category(category_id);
    let voice_channel = match guild_id.create_channel(&ctx.http, voice_builder).await {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "Erreur creation salon vocal");
            if let Err(e) = category_id.delete(&ctx.http).await {
                tracing::warn!(error = %e, "failed to delete category after voice creation error");
            }
            return;
        }
    };

    let voice_channel_id = voice_channel.id;

    // Set permissions on the voice channel
    let everyone_role = serenity::model::id::RoleId::new(guild_id.get());

    if hidden {
        // Hidden: deny view for everyone
        let deny_perms = Permissions::VIEW_CHANNEL;
        let overwrite = serenity::model::channel::PermissionOverwrite {
            allow: Permissions::empty(),
            deny: deny_perms,
            kind: serenity::model::channel::PermissionOverwriteType::Role(everyone_role),
        };
        if let Err(e) = voice_channel_id.create_permission(&ctx.http, overwrite).await {
            tracing::warn!(error = %e, "failed to set hidden permission on voice channel");
        }
    }

    // Owner gets full permissions on voice
    let owner_overwrite = serenity::model::channel::PermissionOverwrite {
        allow: Permissions::VIEW_CHANNEL
            | Permissions::CONNECT
            | Permissions::SPEAK
            | Permissions::MANAGE_CHANNELS,
        deny: Permissions::empty(),
        kind: serenity::model::channel::PermissionOverwriteType::Member(user_id),
    };
    if let Err(e) = voice_channel_id.create_permission(&ctx.http, owner_overwrite).await {
        tracing::warn!(error = %e, "failed to set owner permission on voice channel");
    }

    // Create text admin panel inside category
    let admin_text = CreateChannel::new("admin-panel")
        .kind(ChannelType::Text)
        .category(category_id);
    let admin_text_channel = match guild_id.create_channel(&ctx.http, admin_text).await {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "Erreur creation panel admin");
            if let Err(e) = voice_channel_id.delete(&ctx.http).await {
                tracing::warn!(error = %e, "failed to delete voice channel after admin panel creation error");
            }
            if let Err(e) = category_id.delete(&ctx.http).await {
                tracing::warn!(error = %e, "failed to delete category after admin panel creation error");
            }
            return;
        }
    };

    // Set admin panel permissions: only the owner can see
    let deny_everyone = serenity::model::channel::PermissionOverwrite {
        allow: Permissions::empty(),
        deny: Permissions::VIEW_CHANNEL,
        kind: serenity::model::channel::PermissionOverwriteType::Role(everyone_role),
    };
    if let Err(e) = admin_text_channel.id.create_permission(&ctx.http, deny_everyone.clone()).await {
        tracing::warn!(error = %e, "failed to deny everyone on admin panel");
    }

    let allow_owner = serenity::model::channel::PermissionOverwrite {
        allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY,
        deny: Permissions::empty(),
        kind: serenity::model::channel::PermissionOverwriteType::Member(user_id),
    };
    if let Err(e) = admin_text_channel.id.create_permission(&ctx.http, allow_owner).await {
        tracing::warn!(error = %e, "failed to allow owner on admin panel");
    }

    // Create members text panel inside category
    let members_text = CreateChannel::new("membres")
        .kind(ChannelType::Text)
        .category(category_id);
    let members_text_channel = match guild_id.create_channel(&ctx.http, members_text).await {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "Erreur creation panel membres");
            if let Err(e) = admin_text_channel.id.delete(&ctx.http).await {
                tracing::warn!(error = %e, "failed to delete admin panel during cleanup");
            }
            if let Err(e) = voice_channel_id.delete(&ctx.http).await {
                tracing::warn!(error = %e, "failed to delete voice channel during cleanup");
            }
            if let Err(e) = category_id.delete(&ctx.http).await {
                tracing::warn!(error = %e, "failed to delete category during cleanup");
            }
            return;
        }
    };

    // Members panel: visible only in voice
    if let Err(e) = members_text_channel.id.create_permission(&ctx.http, deny_everyone).await {
        tracing::warn!(error = %e, "failed to deny everyone on members panel");
    }

    // Send admin panel embed with controls
    send_admin_panel(ctx, admin_text_channel.id, &channel_name).await;

    // Send members panel embed with vote kick
    send_members_panel(ctx, members_text_channel.id, &channel_name).await;

    // Register in local maps
    {
        let data = ctx.data.read().await;
        if let Some(map) = data.get::<TextToVoiceMapKey>() {
            map.insert(admin_text_channel.id, voice_channel_id);
        }
        if let Some(map) = data.get::<MembersToVoiceMapKey>() {
            map.insert(members_text_channel.id, voice_channel_id);
        }
    }

    // Register channel via API
    let visibility = if hidden { "hidden" } else { "visible" };

    let request = CreateVoiceChannelRequest {
        guild_id: guild_id.get().to_string(),
        owner_id: user_id.get().to_string(),
        owner_name: component.user.name.clone(),
        channel_id: voice_channel_id.get().to_string(),
        text_channel_id: Some(admin_text_channel.id.get().to_string()),
        members_channel_id: Some(members_text_channel.id.get().to_string()),
        queue_channel_id: None,
        category_id: Some(category_id.get().to_string()),
        channel_name: channel_name.clone(),
        kind: "private".to_string(),
        visibility: visibility.to_string(),
        queue_enabled: false,
    };

    {
        let data = ctx.data.read().await;
        let base = data.get::<ApiClientKey>().expect("ApiClient");
        let api = ApiClient::new(base.clone());
        if let Err(e) = api.create_channel(&request).await {
            error!(error = %e, "Erreur enregistrement API du salon");
        }
    }

    // Store owner in local map
    {
        let data = ctx.data.read().await;
        if let Some(map) = data.get::<crate::handler::VoiceOwnerMapKey>() {
            map.insert(voice_channel_id, user_id);
        }
    }

    // Delete the setup text channel (it was the pending config panel)
    if let Err(e) = text_channel_id.delete(&ctx.http).await {
        tracing::warn!(error = %e, "failed to delete setup text channel");
    }

    // Deplacer le createur dans le salon vocal
    if let Err(e) = guild_id.move_member(&ctx.http, user_id, voice_channel_id).await {
        warn!(error = %e, "Impossible de deplacer le membre dans le salon vocal");
    }

    info!(
        voice = %voice_channel_id,
        owner = %user_id,
        hidden = hidden,
        "Salon prive cree"
    );

    // Log
    crate::embeds::log_channel_created(
        ctx,
        user_id.get(),
        "Prive",
        &channel_name,
        &format!("Cache: {hidden}"),
    )
    .await;
}

async fn handle_cancel(ctx: &Context, component: &ComponentInteraction) {
    let text_channel_id = component.channel_id;
    let user_id = component.user.id;

    let removed = {
        let data = ctx.data.read().await;
        let pending = match data.get::<PendingChannelsKey>() {
            Some(p) => p,
            None => {
                super::respond_ephemeral(ctx, component, "Erreur interne.").await;
                return;
            }
        };

        if !pending.is_owner(&text_channel_id, user_id) {
            super::respond_ephemeral(ctx, component, "Vous n'etes pas le proprietaire.").await;
            return;
        }

        pending.remove(&text_channel_id)
    };

    if removed.is_none() {
        super::respond_ephemeral(ctx, component, "Aucune configuration en attente trouvee.").await;
        return;
    }

    // Acknowledge and delete the setup channel
    let ack = CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new()
            .embed(
                CreateEmbed::new()
                    .title("Annule")
                    .description("La creation du salon a ete annulee.")
                    .color(0xe74c3c),
            )
            .components(vec![]),
    );
    if let Err(e) = component.create_response(&ctx.http, ack).await {
        warn!(error = %e, "Erreur ack annulation");
    }

    // Delete the text channel after a short delay
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    if let Err(e) = text_channel_id.delete(&ctx.http).await {
        tracing::warn!(error = %e, "failed to delete cancelled setup channel");
    }

    info!(user = %user_id, "Creation salon prive annulee");
}

/// Send the admin control panel embed with management buttons.
async fn send_admin_panel(ctx: &Context, channel_id: ChannelId, channel_name: &str) {
    let embed = CreateEmbed::new()
        .title(format!("Admin - {channel_name}"))
        .description(
            "Gerez votre salon vocal avec les boutons ci-dessous.\n\n\
             **Visibilite** : Cacher/Montrer le salon\n\
             **Verrouillage** : Empecher les nouveaux membres\n\
             **Limite** : Definir un nombre max de membres\n\
             **Renommer** : Changer le nom du salon\n\
             **Statut** : Ajouter un statut au salon",
        )
        .color(0x3498db);

    let row1 = CreateActionRow::Buttons(vec![
        CreateButton::new("btn_hide")
            .label("Cacher/Montrer")
            .emoji('\u{1f441}')
            .style(serenity::model::application::ButtonStyle::Secondary),
        CreateButton::new("btn_lock")
            .label("Verrouiller")
            .emoji('\u{1f512}')
            .style(serenity::model::application::ButtonStyle::Secondary),
        CreateButton::new("btn_limit")
            .label("Limite")
            .emoji('\u{1f465}')
            .style(serenity::model::application::ButtonStyle::Secondary),
        CreateButton::new("btn_rename")
            .label("Renommer")
            .emoji('\u{270f}')
            .style(serenity::model::application::ButtonStyle::Secondary),
        CreateButton::new("btn_status")
            .label("Statut")
            .emoji('\u{1f4dd}')
            .style(serenity::model::application::ButtonStyle::Secondary),
    ]);

    let row2 = CreateActionRow::Buttons(vec![
        CreateButton::new("select_invite")
            .label("Inviter")
            .emoji('\u{2709}')
            .style(serenity::model::application::ButtonStyle::Success),
        CreateButton::new("btn_kick")
            .label("Expulser")
            .emoji('\u{1f462}')
            .style(serenity::model::application::ButtonStyle::Danger),
        CreateButton::new("btn_ban")
            .label("Bannir")
            .emoji('\u{1f6ab}')
            .style(serenity::model::application::ButtonStyle::Danger),
    ]);

    let row3 = CreateActionRow::Buttons(vec![
        CreateButton::new("btn_coadmin")
            .label("Co-admin")
            .emoji('\u{1f91d}')
            .style(serenity::model::application::ButtonStyle::Primary),
        CreateButton::new("btn_transfer")
            .label("Transferer")
            .emoji('\u{1f501}')
            .style(serenity::model::application::ButtonStyle::Primary),
        CreateButton::new("btn_queue")
            .label("File d'attente")
            .emoji('\u{1f3ab}')
            .style(serenity::model::application::ButtonStyle::Primary),
    ]);

    let msg = CreateMessage::new()
        .embed(embed)
        .components(vec![row1, row2, row3]);

    if let Err(e) = channel_id.send_message(&ctx.http, msg).await {
        error!(error = %e, "Erreur envoi panel admin");
    }
}

/// Send the members panel embed with vote kick controls.
async fn send_members_panel(ctx: &Context, channel_id: ChannelId, channel_name: &str) {
    let embed = CreateEmbed::new()
        .title(format!("Membres - {channel_name}"))
        .description(
            "Bienvenue dans le salon ! Utilisez le menu ci-dessous pour lancer un vote kick.",
        )
        .color(0x2ecc71);

    let row = CreateActionRow::Buttons(vec![
        CreateButton::new("select_votekick")
            .label("Vote kick")
            .emoji('\u{1f5f3}')
            .style(serenity::model::application::ButtonStyle::Danger),
    ]);

    let msg = CreateMessage::new().embed(embed).components(vec![row]);

    if let Err(e) = channel_id.send_message(&ctx.http, msg).await {
        error!(error = %e, "Erreur envoi panel membres");
    }
}
