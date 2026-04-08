use serenity::all::{
    Context, CreateActionRow, CreateButton, CreateSelectMenu, CreateSelectMenuKind,
    ComponentInteraction, PermissionOverwrite, PermissionOverwriteType,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::builder::{CreateChannel, CreateMessage};
use serenity::model::channel::ChannelType;
use serenity::model::Permissions;
use tracing::{error, info, warn};

use sentinel_shared::heartbeat::ApiClientKey;

use crate::api_client::ApiClient;

use super::constants::*;
use super::helpers::*;

/// Bouton "Inviter quelqu'un" — reserve a l'utilisateur (pas le staff)
pub async fn handle_invite_button(ctx: &Context, component: &ComponentInteraction) {
    // Seul l'utilisateur du ticket peut inviter, pas le staff
    if let Some(guild_id) = component.guild_id {
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

        if is_staff {
            if let Err(e) = component.create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Seul l'utilisateur du ticket peut inviter des personnes.")
                        .ephemeral(true),
                ),
            ).await {
                warn!(error = %e, "Failed to send staff-only invite rejection");
            }
            return;
        }
    }

    let select = CreateSelectMenu::new(
        INVITE_SELECT_ID,
        CreateSelectMenuKind::User { default_users: None },
    )
    .placeholder("Selectionnez un membre a inviter...");

    let row = CreateActionRow::SelectMenu(select);

    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content("**Qui souhaitez-vous inviter dans ce ticket ?**")
            .components(vec![row])
            .ephemeral(true),
    );

    if let Err(e) = component.create_response(&ctx.http, response).await {
        error!(error = %e, "Erreur affichage menu invitation");
    }
}

/// Gere la selection d'un utilisateur a inviter via le UserSelect
pub async fn handle_invite_select(ctx: &Context, component: &ComponentInteraction) {
    let user_ids = match &component.data.kind {
        serenity::all::ComponentInteractionDataKind::UserSelect { values } => values.clone(),
        _ => return,
    };

    let user_id = match user_ids.first() {
        Some(id) => *id,
        None => return,
    };

    // Ne pas inviter un bot
    if let Ok(user) = user_id.to_user(&ctx.http).await {
        if user.bot {
            if let Err(e) = component.create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Impossible d'inviter un bot dans le ticket.")
                        .ephemeral(true),
                ),
            ).await {
                warn!(error = %e, "Failed to send bot invite rejection");
            }
            return;
        }
    }

    // Ajouter la permission VIEW_CHANNEL pour cet utilisateur
    let overwrite = PermissionOverwrite {
        allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY,
        deny: Permissions::empty(),
        kind: PermissionOverwriteType::Member(user_id),
    };

    if let Err(e) = component.channel_id.create_permission(&ctx.http, overwrite).await {
        error!(error = %e, "Impossible d'inviter l'utilisateur");
        if let Err(e) = component.create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Erreur lors de l'invitation.")
                    .ephemeral(true),
            ),
        ).await {
            warn!(error = %e, "Failed to send invite error response");
        }
        return;
    }

    // Message visible dans le salon
    if let Err(e) = component.channel_id.say(
        &ctx.http,
        format!("<@{}> a ete invite dans ce ticket.", user_id),
    ).await {
        warn!(error = %e, "Failed to send invite notification in channel");
    }

    if let Err(e) = component.create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(format!("<@{}> a ete invite avec succes !", user_id))
                .ephemeral(true),
        ),
    ).await {
        warn!(error = %e, "Failed to send invite success response");
    }

    // Sync invited_user_id vers l'API
    if let Some(ref ticket_id) = get_ticket_id_from_channel(ctx, component.channel_id).await {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            let api = ApiClient::new(base.clone());
            if let Err(e) = api.update_ticket_channel(ticket_id, None, Some(user_id.to_string())).await {
                error!(error = %e, ticket_id = %ticket_id, "Erreur sync invited_user_id vers API");
            }
        }
    }

    info!(user_id = %user_id, channel = %component.channel_id, "Utilisateur invite dans le ticket via menu");
}

/// Bouton "Passer en vocal" — reserve aux admins/modos
/// Propose directement a l'utilisateur (pas d'etape de confirmation staff)
pub async fn handle_vocal_button(ctx: &Context, component: &ComponentInteraction) {
    let guild_id = match component.guild_id {
        Some(id) => id,
        None => return,
    };

    // Verifier que l'utilisateur est admin ou moderateur
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
                    .content("Seuls les administrateurs et moderateurs peuvent proposer un vocal.")
                    .ephemeral(true),
            ),
        ).await {
            warn!(error = %e, "Failed to send vocal staff-only rejection");
        }
        return;
    }

    // Verifier qu'un salon vocal n'existe pas deja
    let channel_name = component.channel_id
        .to_channel(&ctx.http)
        .await
        .ok()
        .and_then(|c| c.guild())
        .map(|c| c.name.clone())
        .unwrap_or_default();

    let vocal_name = format!("vocal-{}", channel_name);
    if let Ok(channels) = guild_id.channels(&ctx.http).await {
        if channels.values().any(|c| c.kind == ChannelType::Voice && c.name == vocal_name) {
            if let Err(e) = component.create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Un salon vocal existe deja pour ce ticket.")
                        .ephemeral(true),
                ),
            ).await {
                warn!(error = %e, "Failed to send vocal already exists response");
            }
            return;
        }
    }

    // Verifier qu'il n'y a pas deja une proposition en attente
    let bot_id = ctx.cache.current_user().id;
    if let Ok(messages) = component.channel_id.messages(&ctx.http, serenity::all::GetMessages::new().limit(10)).await {
        let pending = messages.iter().any(|m| {
            m.author.id == bot_id
                && m.content.contains("Souhaitez-vous passer en vocal")
                && !m.components.is_empty()
        });

        if pending {
            if let Err(e) = component.create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Une proposition de vocal est deja en attente de reponse.")
                        .ephemeral(true),
                ),
            ).await {
                warn!(error = %e, "Failed to send vocal pending response");
            }
            return;
        }
    }

    // Repondre au staff en ephemeral (seul le staff voit ca)
    if let Err(e) = component.create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content("Proposition de vocal envoyee.")
                .ephemeral(true),
        ),
    ).await {
        warn!(error = %e, "Failed to send vocal proposal confirmation");
    }

    // Message visible avec boutons pour l'utilisateur
    let accept_btn = CreateButton::new(VOCAL_USER_ACCEPT_ID)
        .label("Oui, passer en vocal")
        .style(serenity::all::ButtonStyle::Success);
    let decline_btn = CreateButton::new(VOCAL_USER_DECLINE_ID)
        .label("Non merci")
        .style(serenity::all::ButtonStyle::Secondary);

    let row = CreateActionRow::Buttons(vec![accept_btn, decline_btn]);

    let msg = CreateMessage::new()
        .content(
            "Le staff vous propose de passer en **vocal** pour echanger plus facilement.\n\
             Un salon vocal prive sera cree si vous acceptez.\n\n\
             Souhaitez-vous passer en vocal ?"
        )
        .components(vec![row]);

    if let Err(e) = component.channel_id.send_message(&ctx.http, msg).await {
        warn!(error = %e, "Failed to send vocal proposal message");
    }
}

/// L'utilisateur accepte le passage en vocal — creation du salon
pub async fn handle_vocal_user_accept(ctx: &Context, component: &ComponentInteraction) {
    let guild_id = match component.guild_id {
        Some(id) => id,
        None => return,
    };

    let channel_name = component.channel_id
        .to_channel(&ctx.http)
        .await
        .ok()
        .and_then(|c| c.guild())
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "ticket".to_string());

    let vocal_name = format!("vocal-{}", channel_name);

    // Verifier qu'un salon vocal n'existe pas deja pour ce ticket
    if let Ok(channels) = guild_id.channels(&ctx.http).await {
        let already_exists = channels.values().any(|c| {
            c.kind == ChannelType::Voice && c.name == vocal_name
        });

        if already_exists {
            if let Err(e) = component.create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Le salon vocal existe deja pour ce ticket.")
                        .ephemeral(true),
                ),
            ).await {
                warn!(error = %e, "Failed to send vocal channel already exists response");
            }
            return;
        }
    }

    // Copier les permissions du salon texte
    let text_channel = match component.channel_id.to_channel(&ctx.http).await {
        Ok(c) => c,
        Err(_) => return,
    };

    let guild_channel = text_channel.guild();
    let overwrites = guild_channel
        .as_ref()
        .map(|c| c.permission_overwrites.clone())
        .unwrap_or_default();

    let category_id = guild_channel
        .as_ref()
        .and_then(|c| c.parent_id);

    let mut create = CreateChannel::new(&vocal_name)
        .kind(ChannelType::Voice)
        .permissions(overwrites);

    if let Some(cat_id) = category_id {
        create = create.category(cat_id);
    }

    match guild_id.create_channel(&ctx.http, create).await {
        Ok(vc) => {
            // Remplacer le message de proposition par le resultat (supprime les boutons)
            if let Err(e) = component.create_response(
                &ctx.http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .content(format!(
                            "Salon vocal cree ! Rejoignez <#{}> pour discuter.",
                            vc.id
                        ))
                        .components(vec![])
                ),
            ).await {
                warn!(error = %e, "Failed to send vocal channel created response");
            }

            info!(vocal = %vc.name, ticket = %channel_name, "Salon vocal cree pour ticket (accepte par l'utilisateur)");

            // Sync voice_channel_id vers l'API
            if let Some(ref ticket_id) = get_ticket_id_from_channel(ctx, component.channel_id).await {
                let data = ctx.data.read().await;
                if let Some(base) = data.get::<ApiClientKey>() {
                    let api = ApiClient::new(base.clone());
                    if let Err(e) = api.update_ticket_channel(ticket_id, Some(vc.id.to_string()), None).await {
                        error!(error = %e, ticket_id = %ticket_id, "Erreur sync voice_channel_id vers API");
                    }
                }
            }
        }
        Err(e) => {
            error!(error = %e, "Impossible de creer le salon vocal");
            if let Err(e) = component.create_response(
                &ctx.http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .content("Impossible de creer le salon vocal.")
                        .components(vec![])
                ),
            ).await {
                warn!(error = %e, "Failed to send vocal creation error response");
            }
        }
    }
}

/// L'utilisateur refuse le passage en vocal
pub async fn handle_vocal_user_decline(ctx: &Context, component: &ComponentInteraction) {
    // Remplacer le message de proposition par le refus (supprime les boutons)
    if let Err(e) = component.create_response(
        &ctx.http,
        CreateInteractionResponse::UpdateMessage(
            CreateInteractionResponseMessage::new()
                .content(format!(
                    "<@{}> a decline la discussion vocale. La conversation continue a l'ecrit.",
                    component.user.id
                ))
                .components(vec![])
        ),
    ).await {
        warn!(error = %e, "Failed to send vocal decline response");
    }
}

// ── Templates de reponses rapides ──

/// Gere le clic sur le bouton "Reponses rapides" dans un ticket.
pub async fn handle_template_button(ctx: &Context, component: &ComponentInteraction) {
    let guild_id = component.guild_id.map(|g| g.to_string()).unwrap_or_default();

    let templates_raw = {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            let gc = match base.get_guild_config(&guild_id).await {
                Ok(cfg) => cfg,
                Err(e) => {
                    tracing::warn!(error = %e, guild_id = %guild_id, "Echec chargement config guild");
                    std::collections::HashMap::new()
                }
            };
            sentinel_shared::api_client::BaseApiClient::config_or(&gc, "response_templates", "")
        } else {
            String::new()
        }
    };

    let templates = crate::templates::parse_templates(&templates_raw);

    if templates.is_empty() {
        let response = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content("Aucun template de reponse configure pour ce serveur.")
                .ephemeral(true),
        );
        if let Err(e) = component.create_response(&ctx.http, response).await {
            warn!(error = %e, "Failed to send empty templates response");
        }
        return;
    }

    let row = crate::templates::build_template_select(&templates);

    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content("**Choisissez une reponse rapide :**")
            .components(vec![row])
            .ephemeral(true),
    );

    if let Err(e) = component.create_response(&ctx.http, response).await {
        error!(error = %e, "Erreur envoi menu templates");
    }
}

/// Gere la selection d'un template → envoie le contenu dans le salon.
pub async fn handle_template_select(ctx: &Context, component: &ComponentInteraction) {
    let selected_index = match &component.data.kind {
        serenity::all::ComponentInteractionDataKind::StringSelect { values } => {
            values.first().and_then(|v| v.parse::<usize>().ok())
        }
        _ => None,
    };

    let index = match selected_index {
        Some(i) => i,
        None => return,
    };

    let guild_id = component.guild_id.map(|g| g.to_string()).unwrap_or_default();

    let templates_raw = {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            let gc = match base.get_guild_config(&guild_id).await {
                Ok(cfg) => cfg,
                Err(e) => {
                    tracing::warn!(error = %e, guild_id = %guild_id, "Echec chargement config guild");
                    std::collections::HashMap::new()
                }
            };
            sentinel_shared::api_client::BaseApiClient::config_or(&gc, "response_templates", "")
        } else {
            String::new()
        }
    };

    let templates = crate::templates::parse_templates(&templates_raw);

    if let Some(template) = templates.get(index) {
        // Envoyer le contenu du template dans le salon
        if let Err(e) = component.channel_id.say(&ctx.http, &template.content).await {
            warn!(error = %e, "Failed to send template content in channel");
        }

        let response = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(format!("Template \"{}\" envoye.", template.label))
                .ephemeral(true),
        );
        if let Err(e) = component.create_response(&ctx.http, response).await {
            warn!(error = %e, "Failed to send template applied response");
        }
    }
}
