use serenity::all::{
    Context, CreateActionRow, CreateButton, ComponentInteraction,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::model::channel::ChannelType;
use tracing::{error, info, warn};

use sentinel_shared::heartbeat::ApiClientKey;

use crate::modules::tickets::api_client::ApiClient;

use super::constants::*;
use super::helpers::*;

/// Bouton "Fermer le ticket"
pub async fn handle_close_button(ctx: &Context, component: &ComponentInteraction) {
    let guild_id = match component.guild_id {
        Some(id) => id,
        None => return,
    };

    let is_staff = is_staff_member(ctx, guild_id, component.user.id).await;

    if is_staff {
        let confirm_btn = CreateButton::new(CLOSE_CONFIRM_ID)
            .label("Valider la fermeture")
            .style(serenity::all::ButtonStyle::Danger);
        let cancel_btn = CreateButton::new(CLOSE_CANCEL_ID)
            .label("Annuler")
            .style(serenity::all::ButtonStyle::Secondary);

        let row = CreateActionRow::Buttons(vec![confirm_btn, cancel_btn]);

        if let Err(e) = component.create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("**Voulez-vous fermer ce ticket ?**\nLe salon sera supprime apres validation.")
                    .components(vec![row])
                    .ephemeral(true),
            ),
        ).await {
            warn!(error = %e, "Failed to send close confirmation prompt");
        }
    } else {
        if let Err(e) = component.create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Votre demande de fermeture a ete envoyee au staff.")
                    .ephemeral(true),
            ),
        ).await {
            warn!(error = %e, "Failed to send close request acknowledgement");
        }

        let msg = serenity::builder::CreateMessage::new()
            .content(format!(
                "**<@{}> souhaite fermer ce ticket.**\n\
                 En attente de validation d'un administrateur ou moderateur.",
                component.user.id
            ));

        if let Err(e) = component.channel_id.send_message(&ctx.http, msg).await {
            warn!(error = %e, "Failed to send close request message");
        }
    }
}

/// Un admin/modo valide la fermeture du ticket
pub async fn handle_close_confirm(ctx: &Context, component: &ComponentInteraction) {
    let guild_id = match component.guild_id {
        Some(id) => id,
        None => return,
    };

    let is_staff = match guild_id.member(&ctx.http, component.user.id).await {
        Ok(member) => {
            if let Some(guild) = guild_id.to_guild_cached(&ctx.cache) {
                let permissions = guild.member_permissions(&member);
                permissions.manage_messages() || permissions.administrator()
            } else {
                false
            }
        }
        Err(_) => false,
    };

    if !is_staff {
        if let Err(e) = component.create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Seuls les administrateurs et moderateurs peuvent valider la fermeture.")
                    .ephemeral(true),
            ),
        ).await {
            warn!(error = %e, "Failed to send staff-only close rejection");
        }
        return;
    }

    let channel_id = component.channel_id;
    let channel_name = channel_id
        .to_channel(&ctx.http)
        .await
        .ok()
        .and_then(|c| c.guild())
        .map(|c| c.name.clone())
        .unwrap_or_default();

    let ticket_id = get_ticket_id_from_channel(ctx, channel_id).await;

    if let Err(e) = component.create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(format!(
                    "Ticket ferme par <@{}>. Ce salon sera supprime dans 5 secondes.",
                    component.user.id
                ))
        ),
    ).await {
        warn!(error = %e, "Failed to send ticket close confirmation");
    }

    let data = ctx.data.read().await;
    if let Some(base) = data.get::<ApiClientKey>() {
        if let Some(ref id) = ticket_id {
            if let Some(grpc) = data.get::<sentinel_shared::grpc_client::GrpcClientKey>() {
                let api = ApiClient::new(base.clone(), grpc.clone());
                if let Err(e) = api.close_ticket(id).await {
                    error!(error = %e, ticket_id = %id, "Erreur fermeture ticket API");
                }
            }
        } else {
            warn!(channel = %channel_name, "Impossible de trouver l'UUID du ticket dans le topic du salon");
        }

        base.send_log(
            "info",
            &component.guild_id.map(|g| g.to_string()).unwrap_or_default(),
            &format!(
                "Ticket ferme : {} (id: {}) par {}",
                channel_name,
                ticket_id.as_deref().unwrap_or("inconnu"),
                component.user.name
            ),
        );
    }

    info!(
        channel = %channel_name,
        ticket_id = %ticket_id.as_deref().unwrap_or("inconnu"),
        user = %component.user.name,
        "Ticket ferme (valide par le staff)"
    );

    let vocal_name = format!("vocal-{}", channel_name);
    if let Ok(channels) = guild_id.channels(&ctx.http).await {
        for (ch_id, ch) in &channels {
            if ch.kind == ChannelType::Voice && ch.name == vocal_name {
                if let Err(e) = ch_id.delete(&ctx.http).await {
                    warn!(error = %e, vocal = %vocal_name, "Impossible de supprimer le salon vocal du ticket");
                } else {
                    info!(vocal = %vocal_name, "Salon vocal du ticket supprime");
                }
            }
        }
    }

    let transcript_enabled = {
        let data2 = ctx.data.read().await;
        if let Some(base) = data2.get::<ApiClientKey>() {
            let gc = match base.get_guild_config_for(&guild_id.to_string(), crate::modules::tickets::MODULE_BOT_NAME).await {
                Ok(cfg) => cfg,
                Err(e) => {
                    tracing::warn!(error = %e, guild_id = %guild_id, "Echec chargement config guild");
                    std::collections::HashMap::new()
                }
            };
            sentinel_shared::api_client::BaseApiClient::config_bool(&gc, "transcript_dm_enabled", true)
        } else {
            true
        }
    };

    let close_delay = {
        let data2 = ctx.data.read().await;
        if let Some(base) = data2.get::<ApiClientKey>() {
            let gc = match base.get_guild_config_for(&guild_id.to_string(), crate::modules::tickets::MODULE_BOT_NAME).await {
                Ok(cfg) => cfg,
                Err(e) => {
                    tracing::warn!(error = %e, guild_id = %guild_id, "Echec chargement config guild");
                    std::collections::HashMap::new()
                }
            };
            sentinel_shared::api_client::BaseApiClient::config_u64(&gc, "close_delay_secs", 5)
        } else {
            5
        }
    };

    let satisfaction_enabled = {
        let data2 = ctx.data.read().await;
        if let Some(base) = data2.get::<ApiClientKey>() {
            let gc = base
                .get_guild_config_for(&guild_id.to_string(), crate::modules::tickets::MODULE_BOT_NAME)
                .await
                .unwrap_or_default();
            sentinel_shared::api_client::BaseApiClient::config_bool(&gc, "satisfaction_enabled", true)
        } else {
            true
        }
    };

    if transcript_enabled {
    if let Some(ref id) = ticket_id {
        let data2 = ctx.data.read().await;
        if let (Some(base), Some(grpc)) = (
            data2.get::<ApiClientKey>(),
            data2.get::<sentinel_shared::grpc_client::GrpcClientKey>(),
        ) {
            let api = ApiClient::new(base.clone(), grpc.clone());
            if let Ok(detail) = api.get_ticket(id).await {
                if let Ok(author_id) = detail.ticket.author_id.parse::<u64>() {
                    let user_id = serenity::model::id::UserId::new(author_id);
                    if let Ok(dm_channel) = user_id.create_dm_channel(&ctx.http).await {
                        let mut transcript = format!(
                            "**Transcript du ticket #{short_id}**\n\
                             **Sujet :** {title}\n\
                             **Type :** {category}\n\
                             **Statut :** Ferme\n\
                             ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n",
                            short_id = &id[..8.min(id.len())],
                            title = detail.ticket.title,
                            category = detail.ticket.category,
                        );

                        for msg in &detail.messages {
                            transcript.push_str(&format!(
                                "**[{}]** {} :\n> {}\n\n",
                                msg.author_role, msg.author_name, msg.content
                            ));
                        }

                        if detail.messages.is_empty() {
                            transcript.push_str("_Aucun message dans ce ticket._\n");
                        }

                        let mut buf = String::new();
                        let mut char_count = 0usize;
                        for ch in transcript.chars() {
                            if char_count + 1 > 1900 {
                                if let Err(e) = dm_channel.say(&ctx.http, &buf).await {
                                    warn!(error = %e, "Failed to send transcript DM chunk");
                                }
                                buf.clear();
                                char_count = 0;
                            }
                            buf.push(ch);
                            char_count += 1;
                        }
                        if !buf.is_empty() {
                            if let Err(e) = dm_channel.say(&ctx.http, &buf).await {
                                warn!(error = %e, "Failed to send transcript DM chunk");
                            }
                        }

                        // Survey satisfaction (1-5 etoiles) si enable.
                        if satisfaction_enabled {
                            let survey = crate::modules::tickets::satisfaction::build_survey_message(id);
                            if let Err(e) = dm_channel.send_message(&ctx.http, survey).await {
                                warn!(error = %e, "Failed to send satisfaction survey DM");
                            }
                        }
                    }
                }
            }
        }
    }
    }

    tokio::time::sleep(tokio::time::Duration::from_secs(close_delay)).await;
    if let Err(e) = channel_id.delete(&ctx.http).await {
        warn!(error = %e, "Failed to delete ticket channel");
    }
}

/// Un admin/modo refuse la fermeture
pub async fn handle_close_cancel(ctx: &Context, component: &ComponentInteraction) {
    let guild_id = match component.guild_id {
        Some(id) => id,
        None => return,
    };

    let is_staff = match guild_id.member(&ctx.http, component.user.id).await {
        Ok(member) => {
            if let Some(guild) = guild_id.to_guild_cached(&ctx.cache) {
                let permissions = guild.member_permissions(&member);
                permissions.manage_messages() || permissions.administrator()
            } else {
                false
            }
        }
        Err(_) => false,
    };

    if !is_staff {
        if let Err(e) = component.create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Seuls les administrateurs et moderateurs peuvent gerer la fermeture.")
                    .ephemeral(true),
            ),
        ).await {
            warn!(error = %e, "Failed to send staff-only cancel rejection");
        }
        return;
    }

    if let Err(e) = component.create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(format!(
                    "<@{}> a decide de garder ce ticket ouvert. La discussion continue.",
                    component.user.id
                ))
        ),
    ).await {
        warn!(error = %e, "Failed to send cancel close response");
    }
}

/// Gere le clic sur un bouton de satisfaction (1-5 etoiles).
pub async fn handle_satisfaction_click(ctx: &Context, component: &ComponentInteraction) {
    let rating = match crate::modules::tickets::satisfaction::extract_rating(&component.data.custom_id) {
        Some(r) => r,
        None => return,
    };

    let guild_id = component.guild_id.map(|g| g.to_string()).unwrap_or_default();
    let ticket_id = crate::modules::tickets::satisfaction::extract_ticket_id(&component.data.custom_id);
    {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            base.send_log(
                "info",
                &guild_id,
                &format!(
                    "Satisfaction ticket : {} a donne {}/5 etoiles",
                    component.user.name, rating
                ),
            );
        }
        if let (Some(tid), Some(api)) = (ticket_id, crate::modules::tickets::api_client::ApiClient::from_data(&data)) {
            api.update_ticket_sla(tid, None, None, Some(rating)).await;
        }
    }

    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(format!("Merci pour votre retour ! Vous avez donne **{}/5** etoiles.", rating))
            .ephemeral(true),
    );

    if let Err(e) = component.create_response(&ctx.http, response).await {
        warn!(error = %e, "Failed to send satisfaction response");
    }
    info!(user = %component.user.name, rating = rating, "Satisfaction ticket enregistree");
}
