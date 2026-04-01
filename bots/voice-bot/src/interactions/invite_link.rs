use serenity::builder::{
    CreateActionRow, CreateButton, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use serenity::model::application::{ButtonStyle, ComponentInteraction};
use serenity::prelude::*;
use tracing::{error, info, warn};

use sentinel_shared::heartbeat::ApiClientKey;

use crate::api_client::{ApiClient, CreateInviteLinkRequest};
use crate::embeds;

pub async fn handle(ctx: &Context, component: &ComponentInteraction) {
    let custom_id = component.data.custom_id.as_str();

    match custom_id {
        "btn_invite_link" => handle_create_menu(ctx, component).await,
        other if other.starts_with("invite_duration_") => {
            handle_duration_select(ctx, component).await;
        }
        _ => {
            warn!(custom_id = %custom_id, "Invite link interaction inconnue");
        }
    }
}

// ── Step 1: Show duration selection ──

async fn handle_create_menu(ctx: &Context, component: &ComponentInteraction) {
    let Some((voice_channel_id, _ch)) = super::require_admin(ctx, component).await else {
        return;
    };

    let vc_id = voice_channel_id.get();

    let buttons = vec![
        CreateButton::new(format!("invite_duration_{vc_id}_900"))
            .label("15 min")
            .style(ButtonStyle::Secondary),
        CreateButton::new(format!("invite_duration_{vc_id}_1800"))
            .label("30 min")
            .style(ButtonStyle::Primary),
        CreateButton::new(format!("invite_duration_{vc_id}_3600"))
            .label("1 heure")
            .style(ButtonStyle::Secondary),
        CreateButton::new(format!("invite_duration_{vc_id}_86400"))
            .label("24 heures")
            .style(ButtonStyle::Secondary),
    ];

    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content("Choisissez la duree du lien d'invitation :")
            .components(vec![CreateActionRow::Buttons(buttons)])
            .ephemeral(true),
    );

    if let Err(e) = component.create_response(&ctx.http, response).await {
        error!(error = %e, "Erreur envoi menu duree invite");
    }
}

// ── Step 2: Generate invite link ──

async fn handle_duration_select(ctx: &Context, component: &ComponentInteraction) {
    let custom_id = &component.data.custom_id;

    // Parse: invite_duration_{channel_id}_{secs}
    let parts: Vec<&str> = custom_id.splitn(4, '_').collect();
    if parts.len() < 4 {
        super::respond_ephemeral(ctx, component, "Erreur: format d'interaction invalide.").await;
        return;
    }

    let channel_id_str = parts[2];
    let duration_secs: i64 = match parts[3].parse() {
        Ok(v) => v,
        Err(_) => {
            super::respond_ephemeral(ctx, component, "Erreur: duree invalide.").await;
            return;
        }
    };

    let user = &component.user;

    let data = ctx.data.read().await;
    let Some(base) = data.get::<ApiClientKey>() else {
        super::respond_ephemeral(ctx, component, "Erreur interne (API client).").await;
        return;
    };

    let api = ApiClient::new(base.clone());
    let request = CreateInviteLinkRequest {
        created_by: user.id.get().to_string(),
        created_by_name: user.name.clone(),
        duration_secs: Some(duration_secs),
        max_uses: None,
    };

    match api.create_invite_link(channel_id_str, &request).await {
        Ok(link) => {
            let duration_label = match duration_secs {
                900 => "15 minutes".to_string(),
                1800 => "30 minutes".to_string(),
                3600 => "1 heure".to_string(),
                86400 => "24 heures".to_string(),
                s => format!("{s} secondes"),
            };

            super::respond_ephemeral(
                ctx,
                component,
                &format!(
                    "Lien d'invitation cree !\n\nCode : **`{}`**\nValide : {}\n\nPartagez ce code — les membres peuvent l'utiliser avec `!join {}`",
                    link.code, duration_label, link.code
                ),
            )
            .await;

            // Log embed
            let channel_name = embeds::get_channel_name(ctx, serenity::model::id::ChannelId::new(channel_id_str.parse().unwrap_or(0))).await;
            embeds::log_invite_link_created(ctx, user.id.get(), &channel_name, &link.code, &duration_label).await;

            info!(code = %link.code, channel = %channel_id_str, creator = %user.id, "Invite link cree");
        }
        Err(e) => {
            error!(error = %e, "Erreur creation invite link");
            super::respond_ephemeral(ctx, component, "Erreur lors de la creation du lien d'invitation.").await;
        }
    }
}
