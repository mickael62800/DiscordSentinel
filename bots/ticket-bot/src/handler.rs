use serenity::async_trait;
use serenity::model::application::Interaction;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use tracing::{error, info, warn};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::embeds::neutral_embed;
use sentinel_shared::heartbeat::{ApiClientKey, register_guilds};

use crate::api_client::ApiClient;
use crate::commands;
use crate::commands::ticket;
use crate::config::ConfigKey;
use crate::faq;
use crate::satisfaction;
use crate::sla::SlaTracker;
use crate::templates;

pub struct SlaTrackerKey;
impl TypeMapKey for SlaTrackerKey {
    type Value = SlaTracker;
}

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, "Ticket bot connecte");

        // Enregistrer les slash commands globales
        if let Err(e) = serenity::model::application::Command::set_global_commands(
            &ctx.http,
            commands::all(),
        )
        .await
        {
            error!(error = %e, "Impossible d'enregistrer les slash commands");
        } else {
            info!("Slash commands enregistrees");
        }

        register_guilds(&ctx, &ready).await;

        // Deployer automatiquement le panel dans le salon configure
        deploy_panel_if_needed(&ctx).await;

        // Lancer la fermeture automatique des tickets inactifs (toutes les 30 min)
        let ctx_clone2 = ctx.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1800)).await;
                close_inactive_tickets(&ctx_clone2).await;
            }
        });

        // Phase 5B : consumer durable Redis Streams avec XREADGROUP + XACK.
        // Replay au redemarrage des events emis pendant que le bot etait down.
        let ctx_clone3 = ctx.clone();
        tokio::spawn(async move {
            let consumer = sentinel_shared::event_bus::default_consumer_name();
            sentinel_shared::event_bus::listen_stream_group(
                "ticket-bot".to_string(),
                consumer,
                move |payload| {
                    let ctx = ctx_clone3.clone();
                    async move {
                        handle_redis_event(&ctx, &payload).await;
                    }
                },
            )
            .await;
        });
    }

    /// Gestion des slash commands ET des interactions composants (boutons, menus, modals).
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let guild_id_str = match &interaction {
            Interaction::Command(c) => c.guild_id.map(|g| g.to_string()),
            Interaction::Component(c) => c.guild_id.map(|g| g.to_string()),
            Interaction::Modal(m) => m.guild_id.map(|g| g.to_string()),
            _ => None,
        };
        if let Some(guild_id) = guild_id_str {
            let data = ctx.data.read().await;
            if let Some(api) = data.get::<ApiClientKey>() {
                let config = match api.get_guild_config(&guild_id).await {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        tracing::warn!(error = %e, guild_id = %guild_id, "Echec chargement config guild");
                        std::collections::HashMap::new()
                    }
                };
                if !BaseApiClient::config_bool(&config, "enabled", true) {
                    return;
                }
            }
        }

        match interaction {
            Interaction::Command(command) => {
                match command.data.name.as_str() {
                    "ticket" => commands::ticket::handle(&ctx, &command).await,
                    _ => {}
                }
            }
            Interaction::Component(component) => {
                match component.data.custom_id.as_str() {
                    ticket::PANEL_BUTTON_ID => ticket::handle_panel_click_with_faq(&ctx, &component).await,
                    ticket::TYPE_SELECT_ID => ticket::handle_type_select(&ctx, &component).await,
                    ticket::CLOSE_BUTTON_ID => ticket::handle_close_button(&ctx, &component).await,
                    ticket::INVITE_BUTTON_ID => ticket::handle_invite_button(&ctx, &component).await,
                    ticket::INVITE_SELECT_ID => ticket::handle_invite_select(&ctx, &component).await,
                    ticket::VOCAL_BUTTON_ID => ticket::handle_vocal_button(&ctx, &component).await,
                    ticket::VOCAL_USER_ACCEPT_ID => ticket::handle_vocal_user_accept(&ctx, &component).await,
                    ticket::VOCAL_USER_DECLINE_ID => ticket::handle_vocal_user_decline(&ctx, &component).await,
                    ticket::CLOSE_CONFIRM_ID => ticket::handle_close_confirm(&ctx, &component).await,
                    ticket::CLOSE_CANCEL_ID => ticket::handle_close_cancel(&ctx, &component).await,
                    templates::TEMPLATE_BUTTON_ID => ticket::handle_template_button(&ctx, &component).await,
                    templates::TEMPLATE_SELECT_ID => ticket::handle_template_select(&ctx, &component).await,
                    faq::FAQ_CONTINUE_ID => ticket::handle_faq_continue(&ctx, &component).await,
                    id if id.starts_with(satisfaction::SATISFACTION_PREFIX) => {
                        ticket::handle_satisfaction_click(&ctx, &component).await;
                    }
                    _ => {}
                }
            }
            Interaction::Modal(modal) => {
                if ticket::is_ticket_modal(&modal.data.custom_id) {
                    ticket::handle_modal_submit(&ctx, &modal).await;
                }
            }
            _ => {}
        }
    }

    /// Sync des messages dans les salons ticket vers le backend.
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        if let Some(guild_id) = msg.guild_id {
            let data = ctx.data.read().await;
            if let Some(api) = data.get::<ApiClientKey>() {
                let config = match api.get_guild_config(&guild_id.to_string()).await {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        tracing::warn!(error = %e, guild_id = %guild_id, "Echec chargement config guild");
                        std::collections::HashMap::new()
                    }
                };
                if !BaseApiClient::config_bool(&config, "enabled", true) {
                    return;
                }
            }
        }

        let channel_name = msg
            .channel_id
            .to_channel(&ctx.http)
            .await
            .ok()
            .and_then(|c| c.guild())
            .map(|c| c.name.clone())
            .unwrap_or_default();

        if !channel_name.starts_with("ticket-") {
            return;
        }

        // Sync message vers le backend — recuperer l'UUID depuis le topic du salon
        let ticket_id = match ticket::get_ticket_id_from_channel(&ctx, msg.channel_id).await {
            Some(id) => id,
            None => {
                warn!(channel = %channel_name, "Impossible de trouver l'UUID du ticket pour la sync des messages");
                return;
            }
        };

        let data = ctx.data.read().await;
        let base = match data.get::<ApiClientKey>() {
            Some(client) => client,
            None => return,
        };
        let api = ApiClient::new(base.clone(), data.get::<sentinel_shared::grpc_client::GrpcClientKey>().expect("GrpcClientKey").clone());

        let author_role = match msg.guild_id {
            Some(guild_id) => {
                if let Ok(member) = guild_id.member(&ctx.http, msg.author.id).await {
                    if let Some(guild) = guild_id.to_guild_cached(&ctx.cache) {
                        let permissions = guild.member_permissions(&member);
                        if permissions.manage_messages() {
                            "moderator"
                        } else {
                            "user"
                        }
                    } else {
                        "user"
                    }
                } else {
                    "user"
                }
            }
            None => "user",
        };

        if let Err(e) = api
            .reply_ticket(&ticket_id, &msg.content, &msg.author.name, author_role)
            .await
        {
            error!(error = %e, ticket_id = %ticket_id, "Erreur sync message vers backend");
        }

        // SLA tracking : enregistrer la premiere reponse staff
        if author_role == "moderator" {
            if let Some(sla) = data.get::<SlaTrackerKey>() {
                if let Some(duration) = sla.record_staff_response(&ticket_id) {
                    let formatted = crate::sla::format_sla_duration(duration);
                    info!(ticket_id = %ticket_id, first_response = %formatted, "SLA premiere reponse enregistree");

                    // Persister via l'API
                    let now = chrono::Utc::now().to_rfc3339();
                    api.update_ticket_sla(&ticket_id, Some(&now), None, None).await;

                    // Event temps reel pour le desktop
                    if let Some(base) = data.get::<ApiClientKey>() {
                        base.publish_event("ticket_sla_updated", serde_json::json!({
                            "ticket_id": ticket_id,
                            "first_response_at": now,
                            "first_response_duration": formatted,
                        }));
                    }
                }
            }
        }

    }
}

/// Deploie le panel de creation de ticket dans le salon configure,
/// sauf s'il y en a deja un (pour eviter les doublons a chaque redemarrage).
async fn deploy_panel_if_needed(ctx: &Context) {
    let data = ctx.data.read().await;

    // Chercher le channel_id depuis la config env
    let channel_id = {
        let config = data.get::<ConfigKey>();
        config.and_then(|c| c.ticket_channel_id)
    };

    // Si pas de config env, chercher dans la guild config de chaque guild (cle: assistance_channel_id)
    let channel_ids: Vec<u64> = if let Some(id) = channel_id {
        vec![id]
    } else if let Some(base) = data.get::<ApiClientKey>() {
        let mut ids = Vec::new();
        for guild in ctx.cache.guilds() {
            let guild_config = match base.get_guild_config(&guild.to_string()).await {
                Ok(cfg) => cfg,
                Err(e) => {
                    tracing::warn!(error = %e, guild_id = %guild, "Echec chargement config guild");
                    std::collections::HashMap::new()
                }
            };
            if let Some(ch_id_str) = guild_config.get("assistance_channel_id") {
                if let Ok(ch_id) = ch_id_str.parse::<u64>() {
                    ids.push(ch_id);
                }
            }
        }
        ids
    } else {
        vec![]
    };

    if channel_ids.is_empty() {
        warn!("Aucun salon de ticket configure (TICKET_CHANNEL_ID ou guild config 'assistance_channel_id'). Le panel ne sera pas deploye automatiquement.");
        return;
    }

    let bot_id = ctx.cache.current_user().id;

    for ch_id in channel_ids {
        let channel_id = ChannelId::new(ch_id);

        // Supprimer les anciens panels du bot pour eviter les doublons
        // et garantir que le contenu est toujours a jour
        if let Ok(messages) = channel_id.messages(&ctx.http, serenity::all::GetMessages::new().limit(20)).await {
            for msg in &messages {
                if msg.author.id == bot_id
                    && !msg.components.is_empty()
                    && msg.content.contains("Assistance & Support")
                {
                    if let Err(e) = msg.delete(&ctx.http).await {
                        warn!(error = %e, "Impossible de supprimer l'ancien panel");
                    }
                }
            }
        }

        // Deployer le panel a jour
        match channel_id.send_message(&ctx.http, ticket::build_panel_message()).await {
            Ok(_) => info!(channel_id = %ch_id, "Panel de tickets deploye"),
            Err(e) => error!(error = %e, channel_id = %ch_id, "Impossible de deployer le panel de tickets"),
        }
    }
}

/// Ferme automatiquement les tickets inactifs depuis plus de 7 jours.
/// Supprime le salon Discord et ferme le ticket via l'API.
async fn close_inactive_tickets(ctx: &Context) {
    let data = ctx.data.read().await;
    let base = match data.get::<ApiClientKey>() {
        Some(b) => b,
        None => return,
    };
    let api = ApiClient::new(base.clone(), data.get::<sentinel_shared::grpc_client::GrpcClientKey>().expect("GrpcClientKey").clone());

    let tickets = match api.list_tickets().await {
        Ok(t) => t,
        Err(_) => return,
    };

    let now = chrono::Utc::now();

    // Lire le timeout depuis la config de chaque guild (ou defaut 7 jours)
    let mut timeout_days = 7i64;
    for guild in ctx.cache.guilds() {
        let guild_config = match base.get_guild_config(&guild.to_string()).await {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::warn!(error = %e, guild_id = %guild, "Echec chargement config guild");
                std::collections::HashMap::new()
            }
        };
        let configured = sentinel_shared::api_client::BaseApiClient::config_u64(&guild_config, "inactive_close_days", 7);
        if configured == 0 {
            return; // 0 = desactive
        }
        timeout_days = configured as i64;
        break;
    }

    for ticket in &tickets {
        if ticket.status == "closed" {
            continue;
        }

        // Parser updated_at pour verifier l'inactivite
        let updated_at = match chrono::DateTime::parse_from_rfc3339(&ticket.updated_at) {
            Ok(dt) => dt.with_timezone(&chrono::Utc),
            Err(_) => continue,
        };

        let inactive_days = (now - updated_at).num_days();
        if inactive_days < timeout_days {
            continue;
        }

        // Fermer le ticket via l'API
        if let Err(e) = api.close_ticket(&ticket.id).await {
            warn!(error = %e, ticket_id = %ticket.id, "Erreur fermeture ticket inactif");
            continue;
        }

        // Supprimer le salon Discord s'il existe
        if let Some(ref channel_id_str) = ticket.channel_id {
            if let Ok(ch_id) = channel_id_str.parse::<u64>() {
                let channel_id = ChannelId::new(ch_id);

                // Envoyer un message avant suppression
                let embed = neutral_embed("\u{1f550} Ticket ferme automatiquement")
                    .description(format!(
                        "Ce ticket a ete ferme apres {} jours d'inactivite.",
                        timeout_days
                    ));
                if let Err(e) = channel_id.send_message(
                    &ctx.http,
                    serenity::builder::CreateMessage::new().embed(embed),
                ).await {
                    warn!(error = %e, "Failed to send auto-close notification");
                }

                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                if let Err(e) = channel_id.delete(&ctx.http).await {
                    warn!(error = %e, "Failed to delete inactive ticket channel");
                }
            }
        }

        info!(ticket_id = %ticket.id, inactive_days = %inactive_days, "Ticket inactif ferme automatiquement");
    }
}

/// Traite un event recu depuis Redis.
/// Si c'est un ticket_message avec author_role=moderator, poste le message dans le salon Discord.
async fn handle_redis_event(ctx: &Context, payload: &str) {
    let event: serde_json::Value = match serde_json::from_str(payload) {
        Ok(v) => v,
        Err(_) => return,
    };

    let event_type = event.get("event").and_then(|e| e.as_str()).unwrap_or("");
    let data = match event.get("data") {
        Some(d) => d,
        None => return,
    };

    if event_type != "ticket_message" {
        return;
    }

    // Extraire les infos du message
    let ticket_id = match data.get("ticket_id").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => return,
    };
    let author_name = data.get("author_name").and_then(|v| v.as_str()).unwrap_or("Staff");

    // Chercher le salon Discord correspondant a ce ticket
    // On parcourt les guilds pour trouver un salon avec [ticket:ID] dans le topic
    let bot_id = ctx.cache.current_user().id;
    for guild_id in ctx.cache.guilds() {
        let channels = match guild_id.channels(&ctx.http).await {
            Ok(c) => c,
            Err(_) => continue,
        };

        for (_ch_id, channel) in &channels {
            if !channel.name.starts_with("ticket-") {
                continue;
            }

            let topic = channel.topic.as_deref().unwrap_or("");
            if let Some(id) = ticket::extract_ticket_id_from_topic(topic) {
                if id == ticket_id {
                    // Recuperer le dernier message du ticket depuis l'API
                    let data_lock = ctx.data.read().await;
                    if let Some(base) = data_lock.get::<ApiClientKey>() {
                        let api = ApiClient::new(base.clone(), data_lock.get::<sentinel_shared::grpc_client::GrpcClientKey>().expect("GrpcClientKey").clone());
                        if let Ok(detail) = api.get_ticket(ticket_id).await {
                            if let Some(last_msg) = detail.messages.last() {
                                if last_msg.author_role == "moderator" {
                                    // Verifier que ce message n'existe pas deja dans le salon Discord
                                    // (ecrit par un humain ou deja reposte par le bot)
                                    let already_in_channel = channel.id
                                        .messages(&ctx.http, serenity::all::GetMessages::new().limit(5))
                                        .await
                                        .ok()
                                        .map(|msgs| msgs.iter().any(|m| {
                                            // Message deja ecrit par un humain dans Discord
                                            (!m.author.bot && m.content == last_msg.content)
                                            // Ou deja reposte par le bot
                                            || (m.author.id == bot_id && m.content.contains(&last_msg.content))
                                        }))
                                        .unwrap_or(false);

                                    if !already_in_channel {
                                        if let Err(e) = channel.id.say(
                                            &ctx.http,
                                            format!("**[staff]** {} :\n> {}", author_name, last_msg.content),
                                        ).await {
                                            warn!(error = %e, "Failed to relay staff message from Redis");
                                        }
                                    }
                                }
                            }
                        }
                    }
                    return;
                }
            }
        }
    }
}
