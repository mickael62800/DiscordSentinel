use serenity::builder::{
    CreateActionRow, CreateButton, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateSelectMenu, CreateSelectMenuKind,
    CreateSelectMenuOption,
};
use serenity::model::application::{ButtonStyle, ComponentInteraction, ComponentInteractionDataKind};
use serenity::model::id::{ChannelId, UserId};
use serenity::model::Permissions;
use serenity::prelude::*;
use tracing::{error, info, warn};

use super::api_client::{ApiClient, AddWhitelistRequest, BanFromChannelRequest};

/// Handle access control interactions: invite, kick, ban.
pub async fn handle(ctx: &Context, component: &ComponentInteraction) {
    let custom_id = component.data.custom_id.as_str();

    match custom_id {
        "select_invite" => handle_invite(ctx, component).await,
        "btn_kick" => handle_kick_menu(ctx, component).await,
        "select_kick" => handle_kick_select(ctx, component).await,
        "btn_ban" => handle_ban_menu(ctx, component).await,
        "select_ban" => handle_ban_select(ctx, component).await,
        other if other.starts_with("ban_duration_") => handle_ban_duration(ctx, component).await,
        _ => {
            warn!(custom_id = %custom_id, "Access control interaction inconnue");
        }
    }
}

// ── Invite ──

async fn handle_invite(ctx: &Context, component: &ComponentInteraction) {
    super::defer_ephemeral(ctx, component).await;
    let Some((voice_channel_id, ch)) = super::require_admin_deferred(ctx, component).await else {
        return;
    };

    let guild_id = component.guild_id.unwrap_or_default();

    let selected_users = match &component.data.kind {
        ComponentInteractionDataKind::UserSelect { values } => values.clone(),
        _ => {
            super::respond_followup_ephemeral(ctx, component, "❌ Interaction invalide, relance l'action depuis le panneau du salon.").await;
            return;
        }
    };

    if selected_users.is_empty() {
        super::respond_followup_ephemeral(ctx, component, "Aucun utilisateur selectionne.").await;
        return;
    }

    let target_id = selected_users[0];

    let overwrite = serenity::model::channel::PermissionOverwrite {
        allow: Permissions::VIEW_CHANNEL | Permissions::CONNECT | Permissions::SPEAK,
        deny: Permissions::empty(),
        kind: serenity::model::channel::PermissionOverwriteType::Member(target_id),
    };
    if let Err(e) = voice_channel_id.create_permission(&ctx.http, overwrite).await {
        error!(error = %e, "Erreur permission invite");
        super::respond_followup_ephemeral(ctx, component, "❌ Impossible d'inviter ce membre (le bot manque peut-etre la permission de gerer ce salon).").await;
        return;
    }

    if let Some(ref text_id_str) = ch.text_channel_id {
        if let Ok(text_id) = text_id_str.parse::<u64>() {
            let text_overwrite = serenity::model::channel::PermissionOverwrite {
                allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY,
                deny: Permissions::empty(),
                kind: serenity::model::channel::PermissionOverwriteType::Member(target_id),
            };
            if let Err(e) = ChannelId::new(text_id).create_permission(&ctx.http, text_overwrite).await {
                tracing::warn!(error = %e, "failed to grant invite permission on text channel");
            }
        }
    }

    if let Some(ref members_id_str) = ch.members_channel_id {
        if let Ok(members_id) = members_id_str.parse::<u64>() {
            let members_overwrite = serenity::model::channel::PermissionOverwrite {
                allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY,
                deny: Permissions::empty(),
                kind: serenity::model::channel::PermissionOverwriteType::Member(target_id),
            };
            if let Err(e) = ChannelId::new(members_id).create_permission(&ctx.http, members_overwrite).await {
                tracing::warn!(error = %e, "failed to grant invite permission on members channel");
            }
        }
    }

    let target_name = target_id
        .to_user(&ctx.http)
        .await
        .map(|u| u.name.clone())
        .unwrap_or_else(|_| target_id.get().to_string());

    let request = AddWhitelistRequest {
        guild_id: guild_id.get().to_string(),
        owner_id: ch.owner_id.clone(),
        target_id: target_id.get().to_string(),
        target_name: target_name.clone(),
    };

    {
        let data = ctx.data.read().await;
        let Some(api) = ApiClient::from_data(&data) else {
            error!("ApiClient ou GrpcClient manquants dans TypeMap");
            return;
        };
        if let Err(e) = api.add_to_whitelist(&request).await {
            warn!(error = %e, "Erreur API whitelist");
        }
    }

    super::respond_followup_ephemeral(
        ctx,
        component,
        &format!("<@{target_id}> a ete invite dans le salon."),
    )
    .await;

    info!(voice = %voice_channel_id, target = %target_id, "Utilisateur invite");
}

// ── Kick ──

async fn handle_kick_menu(ctx: &Context, component: &ComponentInteraction) {
    let Some((voice_channel_id, _ch)) = super::require_admin(ctx, component).await else {
        return;
    };

    let guild_id = component.guild_id.unwrap_or_default();
    let owner_id = component.user.id;

    let members = get_voice_members(ctx, guild_id, voice_channel_id, Some(owner_id)).await;

    if members.is_empty() {
        super::respond_ephemeral(ctx, component, "Aucun membre a expulser dans le salon vocal.").await;
        return;
    }

    let options: Vec<CreateSelectMenuOption> = members
        .iter()
        .map(|(id, name)| CreateSelectMenuOption::new(name, id.get().to_string()))
        .collect();

    let select = CreateSelectMenu::new(
        "select_kick",
        CreateSelectMenuKind::String { options },
    )
    .placeholder("Choisissez un membre a expulser");

    let row = CreateActionRow::SelectMenu(select);

    let msg = CreateInteractionResponseMessage::new()
        .content("Qui souhaitez-vous expulser ?")
        .components(vec![row])
        .ephemeral(true);

    let response = CreateInteractionResponse::Message(msg);
    if let Err(e) = component.create_response(&ctx.http, response).await {
        warn!(error = %e, "Erreur envoi menu kick");
    }
}

async fn handle_kick_select(ctx: &Context, component: &ComponentInteraction) {
    super::defer_ephemeral(ctx, component).await;
    let Some((_voice_channel_id_check, _ch)) = super::require_admin_deferred(ctx, component).await else {
        return;
    };

    let text_channel_id = component.channel_id;

    let voice_channel_id = if let Some(vc) = super::find_voice_from_text(ctx, text_channel_id).await {
        vc
    } else {
        super::respond_followup_ephemeral(ctx, component, "Impossible de trouver le salon vocal associe.").await;
        return;
    };

    let guild_id = component.guild_id.unwrap_or_default();

    let selected_value = match &component.data.kind {
        serenity::model::application::ComponentInteractionDataKind::StringSelect { values } => {
            match values.first() {
                Some(v) => v.clone(),
                None => {
                    super::respond_followup_ephemeral(ctx, component, "Aucun membre selectionne.").await;
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

    match guild_id
        .disconnect_member(&ctx.http, target_user_id)
        .await
    {
        Ok(_) => {
            info!(voice = %voice_channel_id, target = %target_user_id, "Membre expulse");
        }
        Err(e) => {
            error!(error = %e, "Erreur disconnect membre");
            super::respond_followup_ephemeral(ctx, component, "Erreur lors de l'expulsion.").await;
            return;
        }
    }

    super::respond_followup_ephemeral(
        ctx,
        component,
        &format!("<@{target_id}> a ete expulse du salon."),
    )
    .await;
}

// ── Ban ──

async fn handle_ban_menu(ctx: &Context, component: &ComponentInteraction) {
    let Some((voice_channel_id, _ch)) = super::require_admin(ctx, component).await else {
        return;
    };

    let guild_id = component.guild_id.unwrap_or_default();
    let owner_id = component.user.id;

    let members = get_voice_members(ctx, guild_id, voice_channel_id, Some(owner_id)).await;

    if members.is_empty() {
        super::respond_ephemeral(ctx, component, "Aucun membre a bannir dans le salon vocal.").await;
        return;
    }

    let options: Vec<CreateSelectMenuOption> = members
        .iter()
        .map(|(id, name)| CreateSelectMenuOption::new(name, id.get().to_string()))
        .collect();

    let select = CreateSelectMenu::new(
        "select_ban",
        CreateSelectMenuKind::String { options },
    )
    .placeholder("Choisissez un membre a bannir");

    let row = CreateActionRow::SelectMenu(select);

    let msg = CreateInteractionResponseMessage::new()
        .content("Qui souhaitez-vous bannir ?")
        .components(vec![row])
        .ephemeral(true);

    let response = CreateInteractionResponse::Message(msg);
    if let Err(e) = component.create_response(&ctx.http, response).await {
        warn!(error = %e, "Erreur envoi menu ban");
    }
}

async fn handle_ban_select(ctx: &Context, component: &ComponentInteraction) {
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

    let row = CreateActionRow::Buttons(vec![
        CreateButton::new(format!("ban_duration_{selected_value}_300"))
            .label("5 min")
            .style(ButtonStyle::Secondary),
        CreateButton::new(format!("ban_duration_{selected_value}_3600"))
            .label("1 heure")
            .style(ButtonStyle::Secondary),
        CreateButton::new(format!("ban_duration_{selected_value}_86400"))
            .label("24 heures")
            .style(ButtonStyle::Danger),
        CreateButton::new(format!("ban_duration_{selected_value}_0"))
            .label("Permanent")
            .style(ButtonStyle::Danger),
    ]);

    let msg = CreateInteractionResponseMessage::new()
        .content(format!("Duree du ban pour <@{selected_value}> ?"))
        .components(vec![row])
        .ephemeral(true);

    let response = CreateInteractionResponse::Message(msg);
    if let Err(e) = component.create_response(&ctx.http, response).await {
        warn!(error = %e, "Erreur envoi menu duree ban");
    }
}

async fn handle_ban_duration(ctx: &Context, component: &ComponentInteraction) {
    super::defer_ephemeral(ctx, component).await;
    let Some((_voice_channel_id_check, _ch)) = super::require_admin_deferred(ctx, component).await else {
        return;
    };

    let custom_id = component.data.custom_id.as_str();

    let parts: Vec<&str> = custom_id.strip_prefix("ban_duration_").unwrap_or("").rsplitn(2, '_').collect();
    if parts.len() < 2 {
        super::respond_followup_ephemeral(ctx, component, "Format invalide.").await;
        return;
    }

    let duration_secs: i64 = parts[0].parse().unwrap_or(0);
    let target_id_str = parts[1];
    let target_id: u64 = match target_id_str.parse() {
        Ok(id) => id,
        Err(_) => {
            super::respond_followup_ephemeral(ctx, component, "ID utilisateur invalide.").await;
            return;
        }
    };

    let target_user_id = UserId::new(target_id);
    let text_channel_id = component.channel_id;

    let voice_channel_id = if let Some(vc) = super::find_voice_from_text(ctx, text_channel_id).await {
        vc
    } else {
        super::respond_followup_ephemeral(ctx, component, "Impossible de trouver le salon vocal associe.").await;
        return;
    };

    let guild_id = component.guild_id.unwrap_or_default();

    if let Err(e) = guild_id.disconnect_member(&ctx.http, target_user_id).await {
        tracing::warn!(error = %e, "failed to disconnect banned member");
    }

    let overwrite = serenity::model::channel::PermissionOverwrite {
        allow: Permissions::empty(),
        deny: Permissions::VIEW_CHANNEL | Permissions::CONNECT,
        kind: serenity::model::channel::PermissionOverwriteType::Member(target_user_id),
    };
    if let Err(e) = voice_channel_id.create_permission(&ctx.http, overwrite).await {
        tracing::warn!(error = %e, "failed to set ban permission on voice channel");
    }

    let target_name = target_user_id
        .to_user(&ctx.http)
        .await
        .map(|u| u.name.clone())
        .unwrap_or_else(|_| target_id.to_string());

    let ban_request = BanFromChannelRequest {
        user_id: target_id.to_string(),
        user_name: target_name.clone(),
        banned_by: component.user.id.get().to_string(),
        reason: None,
        duration_secs: if duration_secs == 0 {
            None
        } else {
            Some(duration_secs)
        },
    };

    {
        let data = ctx.data.read().await;
        let Some(api) = ApiClient::from_data(&data) else {
            error!("ApiClient ou GrpcClient manquants dans TypeMap");
            return;
        };
        if let Err(e) = api
            .ban_user(&voice_channel_id.get().to_string(), &ban_request)
            .await
        {
            error!(error = %e, "Erreur API ban");
        }
    }

    let duration_text = match duration_secs {
        0 => "permanent".to_string(),
        300 => "5 minutes".to_string(),
        3600 => "1 heure".to_string(),
        86400 => "24 heures".to_string(),
        s => format!("{s} secondes"),
    };

    super::respond_followup_ephemeral(
        ctx,
        component,
        &format!(
            "<@{target_id}> a ete banni du salon ({duration_text})."
        ),
    )
    .await;

    info!(
        voice = %voice_channel_id,
        target = %target_user_id,
        duration = duration_secs,
        "Utilisateur banni"
    );
}

// ── Helpers ──

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
