//! Module tickets — gestion des tickets de support (ex ticket-bot).
//!
//! Migre depuis `bots/ticket-bot/` dans le bot unifie `sentinel-bot`.
//! Features : panels de creation, FAQ auto, SLA tracking + escalade,
//! transcripts DM a la fermeture, satisfaction surveys, templates de reponses
//! rapides, sync messages <-> API, relay staff depuis Redis.

pub const MODULE_BOT_NAME: &str = "ticket-bot";

pub mod api_client;
pub mod commands;
pub mod config;
pub mod faq;
pub mod satisfaction;
pub mod sla;
pub mod templates;
pub mod transcript;

use std::sync::Arc;

use serenity::all::{
    CommandInteraction, ComponentInteraction, Context, CreateCommand, ModalInteraction,
};
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use tracing::{error, info, warn};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::discord_helpers::{
    is_module_enabled, is_module_enabled_or_reply_command, is_module_enabled_or_reply_component,
    is_module_enabled_or_reply_modal,
};
use sentinel_shared::embeds::neutral_embed;
use sentinel_shared::grpc_client::SentinelGrpcClient;
use sentinel_shared::heartbeat::ApiClientKey;

use api_client::ApiClient;
use commands::ticket;
use config::ConfigKey;
use sla::SlaTracker;

// ── TypeMapKeys ──

pub struct SlaTrackerKey;
impl TypeMapKey for SlaTrackerKey {
    type Value = SlaTracker;
}

// Re-export pour l'insertion dans le TypeMap depuis main.rs
pub use config::TicketsConfig;

// ── Init TypeMapKeys ──

/// Insere les TypeMapKeys du module tickets.
pub fn init_typemap(data: &mut serenity::prelude::TypeMap) {
    data.insert::<config::ConfigKey>(TicketsConfig::from_env());
    data.insert::<SlaTrackerKey>(sla::SlaTracker::new());
}

// ── Slash commands ──

pub fn register_commands() -> Vec<CreateCommand> {
    commands::all()
}

pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    let name = command.data.name.as_str();
    if name == "ticket" || name == "ticket-admin" {
        if !is_module_enabled_or_reply_command(ctx, command, MODULE_BOT_NAME).await {
            return;
        }
        ticket::handle(ctx, command).await;
    }
}

// ── Component interactions ──

pub fn handles_component(cid: &str) -> bool {
    matches!(
        cid,
        ticket::PANEL_BUTTON_ID
            | ticket::TYPE_SELECT_ID
            | ticket::CLOSE_BUTTON_ID
            | ticket::INVITE_BUTTON_ID
            | ticket::INVITE_SELECT_ID
            | ticket::VOCAL_BUTTON_ID
            | ticket::VOCAL_USER_ACCEPT_ID
            | ticket::VOCAL_USER_DECLINE_ID
            | ticket::CLOSE_CONFIRM_ID
            | ticket::CLOSE_CANCEL_ID
    ) || cid == templates::TEMPLATE_BUTTON_ID
        || cid == templates::TEMPLATE_SELECT_ID
        || cid == faq::FAQ_CONTINUE_ID
        || cid.starts_with(satisfaction::SATISFACTION_PREFIX)
}

pub async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    let cid = component.data.custom_id.as_str();

    if !is_module_enabled_or_reply_component(ctx, component, MODULE_BOT_NAME).await {
        return;
    }

    match cid {
        ticket::PANEL_BUTTON_ID => ticket::handle_panel_click_with_faq(ctx, component).await,
        ticket::TYPE_SELECT_ID => ticket::handle_type_select(ctx, component).await,
        ticket::CLOSE_BUTTON_ID => ticket::handle_close_button(ctx, component).await,
        ticket::INVITE_BUTTON_ID => ticket::handle_invite_button(ctx, component).await,
        ticket::INVITE_SELECT_ID => ticket::handle_invite_select(ctx, component).await,
        ticket::VOCAL_BUTTON_ID => ticket::handle_vocal_button(ctx, component).await,
        ticket::VOCAL_USER_ACCEPT_ID => ticket::handle_vocal_user_accept(ctx, component).await,
        ticket::VOCAL_USER_DECLINE_ID => ticket::handle_vocal_user_decline(ctx, component).await,
        ticket::CLOSE_CONFIRM_ID => ticket::handle_close_confirm(ctx, component).await,
        ticket::CLOSE_CANCEL_ID => ticket::handle_close_cancel(ctx, component).await,
        _ => {
            if cid == templates::TEMPLATE_BUTTON_ID {
                ticket::handle_template_button(ctx, component).await;
            } else if cid == templates::TEMPLATE_SELECT_ID {
                ticket::handle_template_select(ctx, component).await;
            } else if cid == faq::FAQ_CONTINUE_ID {
                ticket::handle_faq_continue(ctx, component).await;
            } else if cid.starts_with(satisfaction::SATISFACTION_PREFIX) {
                ticket::handle_satisfaction_click(ctx, component).await;
            }
        }
    }
}

// ── Modal interactions ──

pub fn handles_modal(cid: &str) -> bool {
    ticket::is_ticket_modal(cid)
}

pub async fn on_modal(ctx: &Context, modal: &ModalInteraction) {
    if !is_module_enabled_or_reply_modal(ctx, modal, MODULE_BOT_NAME).await {
        return;
    }

    if ticket::is_ticket_modal(&modal.data.custom_id) {
        ticket::handle_modal_submit(ctx, modal).await;
    }
}

// ── Message handler (sync ticket messages -> API + SLA tracking) ──

pub async fn on_message(ctx: &Context, msg: &Message) {
    if let Some(guild_id) = msg.guild_id {
        if !is_module_enabled(ctx, &guild_id.to_string(), MODULE_BOT_NAME).await {
            return;
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

    let ticket_id = match ticket::get_ticket_id_from_channel(ctx, msg.channel_id).await {
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
    let grpc = match data.get::<sentinel_shared::grpc_client::GrpcClientKey>() {
        Some(g) => g.clone(),
        None => return,
    };
    let api = ApiClient::new(base.clone(), grpc);

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

    // SLA tracking : premiere reponse staff
    if author_role == "moderator" {
        if let Some(sla) = data.get::<SlaTrackerKey>() {
            if let Some(duration) = sla.record_staff_response(&ticket_id) {
                let formatted = sla::format_sla_duration(duration);
                info!(ticket_id = %ticket_id, first_response = %formatted, "SLA premiere reponse enregistree");

                let now = chrono::Utc::now().to_rfc3339();
                api.update_ticket_sla(&ticket_id, Some(&now), None, None).await;

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

// ── ready / on_ready : deploiement du panel + consumer Redis ──

pub async fn on_ready(ctx: &Context, _ready: &Ready) {
    deploy_panel_if_needed(ctx).await;
}

// ── Background tasks ──

/// Spawn les background tasks du module tickets : consumer Redis pour
/// relay staff + ticket_auto_closed + ticket_sla_escalated.
///
/// Phase 5 : ferme tickets inactifs (5E) + SLA escalation (5I) sont
/// deplaces dans sentinel-worker. Le bot ne fait plus que consumer
/// les events pour les actions Discord (post message dans channel).
pub fn spawn_background(ctx: Context) {
    // Redis consumer (relay staff + ticket_auto_closed + ticket_sla_escalated)
    let ctx_redis = ctx.clone();
    tokio::spawn(async move {
        let consumer = sentinel_shared::event_bus::default_consumer_name();
        sentinel_shared::event_bus::listen_stream_group(
            "ticket-bot".to_string(),
            consumer,
            move |payload| {
                let ctx = ctx_redis.clone();
                async move {
                    handle_redis_event(&ctx, &payload).await;
                }
            },
        )
        .await;
    });
}

// ── Private helpers (migres depuis l'ex-handler.rs/main.rs du ticket-bot) ──

async fn deploy_panel_if_needed(ctx: &Context) {
    let data = ctx.data.read().await;

    let channel_id = {
        let config = data.get::<ConfigKey>();
        config.and_then(|c| c.ticket_channel_id)
    };

    let channel_ids: Vec<u64> = if let Some(id) = channel_id {
        vec![id]
    } else if let Some(base) = data.get::<ApiClientKey>() {
        let mut ids = Vec::new();
        for guild in ctx.cache.guilds() {
            let guild_config = match base.get_guild_config_for(&guild.to_string(), MODULE_BOT_NAME).await {
                Ok(cfg) => cfg,
                Err(e) => {
                    warn!(error = %e, guild_id = %guild, "Echec chargement config guild");
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

        match channel_id.send_message(&ctx.http, ticket::build_panel_message()).await {
            Ok(_) => info!(channel_id = %ch_id, "Panel de tickets deploye"),
            Err(e) => error!(error = %e, channel_id = %ch_id, "Impossible de deployer le panel de tickets"),
        }
    }
}

async fn close_inactive_tickets(ctx: &Context) {
    let data = ctx.data.read().await;
    let base = match data.get::<ApiClientKey>() {
        Some(b) => b.clone(),
        None => return,
    };
    let grpc = match data.get::<sentinel_shared::grpc_client::GrpcClientKey>() {
        Some(g) => g.clone(),
        None => return,
    };
    drop(data);

    let api = ApiClient::new(Arc::clone(&base), grpc);

    let tickets = match api.list_tickets().await {
        Ok(t) => t,
        Err(_) => return,
    };

    let now = chrono::Utc::now();

    let mut guild_timeouts: std::collections::HashMap<String, i64> =
        std::collections::HashMap::new();

    for ticket in &tickets {
        if ticket.status == "closed" {
            continue;
        }

        let timeout_days = if let Some(t) = guild_timeouts.get(&ticket.server) {
            *t
        } else {
            let guild_config = match base.get_guild_config_for(&ticket.server, MODULE_BOT_NAME).await {
                Ok(cfg) => cfg,
                Err(e) => {
                    warn!(error = %e, guild_id = %ticket.server, "Echec chargement config guild");
                    std::collections::HashMap::new()
                }
            };
            let configured = BaseApiClient::config_u64(
                &guild_config,
                "inactive_close_days",
                7,
            ) as i64;
            guild_timeouts.insert(ticket.server.clone(), configured);
            configured
        };

        if timeout_days <= 0 {
            continue;
        }

        let updated_at = match chrono::DateTime::parse_from_rfc3339(&ticket.updated_at) {
            Ok(dt) => dt.with_timezone(&chrono::Utc),
            Err(_) => continue,
        };

        let inactive_days = (now - updated_at).num_days();
        if inactive_days < timeout_days {
            continue;
        }

        if let Err(e) = api.close_ticket(&ticket.id).await {
            warn!(error = %e, ticket_id = %ticket.id, "Erreur fermeture ticket inactif");
            continue;
        }

        if let Some(ref channel_id_str) = ticket.channel_id {
            if let Ok(ch_id) = channel_id_str.parse::<u64>() {
                let channel_id = ChannelId::new(ch_id);

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

async fn check_escalations(ctx: &Context) {
    let data = ctx.data.read().await;
    let base = match data.get::<ApiClientKey>() {
        Some(b) => b.clone(),
        None => return,
    };
    let grpc: Arc<SentinelGrpcClient> = match data.get::<sentinel_shared::grpc_client::GrpcClientKey>() {
        Some(g) => g.clone(),
        None => return,
    };
    // Extraire le tracker ici sous read lock — on en a besoin ensuite
    let sla_tracker_present = data.get::<SlaTrackerKey>().is_some();
    drop(data);

    if !sla_tracker_present {
        return;
    }

    // Cleanup stale
    {
        let data = ctx.data.read().await;
        if let Some(sla_tracker) = data.get::<SlaTrackerKey>() {
            sla_tracker.cleanup_stale();
        }
    }

    let api = ApiClient::new(Arc::clone(&base), grpc);
    let tickets = match api.list_tickets().await {
        Ok(t) => t,
        Err(_) => return,
    };

    for ticket in &tickets {
        if ticket.status == "closed" {
            continue;
        }

        let guild_config = match base.get_guild_config_for(&ticket.server, MODULE_BOT_NAME).await {
            Ok(cfg) => cfg,
            Err(e) => {
                warn!(error = %e, guild_id = %ticket.server, "Echec chargement config guild");
                std::collections::HashMap::new()
            }
        };
        let escalation_minutes = BaseApiClient::config_u64(&guild_config, "sla_escalation_minutes", 60);
        if escalation_minutes == 0 {
            continue;
        }

        let (breached, already_esc) = {
            let data = ctx.data.read().await;
            match data.get::<SlaTrackerKey>() {
                Some(sla) => {
                    let br = sla.breached_tickets(escalation_minutes);
                    let esc = sla.is_escalated(&ticket.id);
                    (br, esc)
                }
                None => return,
            }
        };

        if !breached.contains(&ticket.id) {
            continue;
        }

        if already_esc {
            continue;
        }

        if let Err(e) = api.update_ticket_priority(&ticket.id, "high").await {
            warn!(error = %e, ticket_id = %ticket.id, "Erreur escalade ticket");
            continue;
        }

        {
            let data = ctx.data.read().await;
            if let Some(sla) = data.get::<SlaTrackerKey>() {
                sla.mark_escalated(&ticket.id);
            }
        }

        if let Some(ref channel_id_str) = ticket.channel_id {
            if let Ok(ch_id) = channel_id_str.parse::<u64>() {
                let channel = ChannelId::new(ch_id);
                let msg = format!(
                    "**\u{26a0}\u{fe0f} Escalade automatique** — Ce ticket n'a pas recu de reponse depuis {}min. La priorite a ete augmentee.",
                    escalation_minutes
                );
                if let Err(e) = channel.say(&ctx.http, &msg).await {
                    warn!(error = %e, "Failed to send escalation message in channel");
                }
            }
        }

        info!(ticket_id = %ticket.id, "Ticket escalade automatiquement (SLA breach)");
    }
}

/// Phase 2 sync : un ticket a ete ferme depuis la web admin. On va
/// verrouiller le channel Discord (deny SendMessages au @everyone) et
/// editer le message de bienvenue pour signaler la fermeture.
async fn handle_ticket_closed_from_web(ctx: &Context, action_id: &str) {
    use serenity::all::{ChannelId, EditChannel, GetMessages, MessageId, PermissionOverwrite,
                        PermissionOverwriteType, Permissions, RoleId};

    // 1. Recupere le mapping discord_action_messages pour cet action_id.
    let data = ctx.data.read().await;
    let api = match data.get::<ApiClientKey>() {
        Some(a) => a.clone(),
        None => return,
    };
    drop(data);

    #[derive(serde::Deserialize)]
    struct Mapping {
        kind: String,
        guild_id: String,
        channel_id: String,
        message_id: String,
    }
    let mappings: Vec<Mapping> = match api
        .get_json(&format!("/api/discord-messages/{action_id}"))
        .await
    {
        Ok(list) => list,
        Err(e) => {
            warn!(error = %e, action_id, "Echec fetch mapping discord_action_messages");
            return;
        }
    };

    let ticket_mapping = match mappings.into_iter().find(|m| m.kind == "ticket") {
        Some(m) => m,
        None => {
            // Pas de mapping enregistre — ticket cree avant la phase 2 sync.
            return;
        }
    };

    let channel_id = match ticket_mapping.channel_id.parse::<u64>() {
        Ok(v) => ChannelId::new(v),
        Err(_) => return,
    };
    let guild_id = match ticket_mapping.guild_id.parse::<u64>() {
        Ok(v) => serenity::all::GuildId::new(v),
        Err(_) => return,
    };

    // 2. Edit le message de bienvenue (action_id) pour signaler la fermeture.
    if let Ok(msg_id_u64) = ticket_mapping.message_id.parse::<u64>() {
        let msg_id = MessageId::new(msg_id_u64);
        // Recupere l embed existant pour ne pas perdre l info, on ajoute
        // juste un footer "Ferme via web".
        if let Ok(messages) = channel_id
            .messages(&ctx.http, GetMessages::new().limit(1).around(msg_id))
            .await
        {
            if let Some(original) = messages.into_iter().find(|m| m.id == msg_id) {
                if let Some(existing_embed) = original.embeds.first() {
                    let new_embed = serenity::builder::CreateEmbed::from(existing_embed.clone())
                        .color(0x95A5A6) // gris
                        .footer(serenity::builder::CreateEmbedFooter::new(
                            "\u{1f512} Ticket ferme depuis la web admin",
                        ));
                    if let Err(e) = channel_id
                        .edit_message(
                            &ctx.http,
                            msg_id,
                            serenity::builder::EditMessage::new().embed(new_embed),
                        )
                        .await
                    {
                        warn!(error = %e, %channel_id, %msg_id, "Echec edit welcome ticket");
                    }
                }
            }
        }
    }

    // 3. Lock le channel : deny SendMessages au @everyone (role = guild_id).
    let everyone_role = RoleId::new(guild_id.get());
    let overwrite = PermissionOverwrite {
        allow: Permissions::empty(),
        deny: Permissions::SEND_MESSAGES,
        kind: PermissionOverwriteType::Role(everyone_role),
    };
    if let Err(e) = channel_id
        .create_permission(&ctx.http, overwrite)
        .await
    {
        warn!(error = %e, %channel_id, "Echec lock channel ticket apres close web");
    }

    // 4. Optionnel : rename le channel pour signaler la fermeture
    // (closed-ticket-XXX). Best-effort.
    if let Ok(channel_obj) = channel_id.to_channel(&ctx.http).await {
        if let Some(guild_channel) = channel_obj.guild() {
            if !guild_channel.name.starts_with("closed-") {
                let new_name = format!("closed-{}", guild_channel.name);
                let _ = channel_id
                    .edit(&ctx.http, EditChannel::new().name(&new_name))
                    .await;
            }
        }
    }

    info!(
        action_id,
        channel = %channel_id,
        "Ticket ferme depuis la web : channel Discord locke + welcome edite"
    );
}

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

    // SLA warning (avant escalation) : declenchee par worker quand un
    // ticket non-repondu depasse sla_first_response_minutes. Bot poste
    // un rappel dans le channel pour ping le staff.
    if event_type == "ticket_sla_warned" {
        let channel_id_str = data.get("channel_id").and_then(|v| v.as_str()).unwrap_or("");
        let warn_minutes = data.get("warn_minutes").and_then(|v| v.as_i64()).unwrap_or(30);
        if channel_id_str.is_empty() {
            return;
        }
        let ch_id = match channel_id_str.parse::<u64>() {
            Ok(v) => v,
            Err(_) => return,
        };
        let channel_id = ChannelId::new(ch_id);
        let msg = format!(
            "**\u{23f0} Rappel SLA** — Aucune reponse depuis {}min sur ce ticket. Merci d y repondre rapidement avant l escalation automatique.",
            warn_minutes
        );
        if let Err(e) = channel_id.say(&ctx.http, &msg).await {
            warn!(error = %e, "Failed to send SLA warning message");
        }
        return;
    }

    // Phase 5I : SLA escalation declenchee par worker. Bot poste le
    // message d'avertissement dans le channel.
    if event_type == "ticket_sla_escalated" {
        let channel_id_str = data
            .get("channel_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let escalation_minutes = data
            .get("escalation_minutes")
            .and_then(|v| v.as_i64())
            .unwrap_or(60);
        if channel_id_str.is_empty() {
            return;
        }
        let ch_id = match channel_id_str.parse::<u64>() {
            Ok(v) => v,
            Err(_) => return,
        };
        let channel_id = ChannelId::new(ch_id);
        let msg = format!(
            "**\u{26a0}\u{fe0f} Escalade automatique** — Ce ticket n'a pas recu de reponse depuis {}min. La priorite a ete augmentee.",
            escalation_minutes
        );
        if let Err(e) = channel_id.say(&ctx.http, &msg).await {
            warn!(error = %e, "Failed to send SLA escalation message");
        }
        return;
    }

    // Phase 5 : ticket ferme automatiquement par sentinel-worker (job
    // close_inactive_tickets). Le bot fait le menage Discord :
    // notification + delete channel apres 3s.
    if event_type == "ticket_auto_closed" {
        let channel_id_str = data
            .get("channel_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let timeout_days = data
            .get("timeout_days")
            .and_then(|v| v.as_i64())
            .unwrap_or(7);
        if channel_id_str.is_empty() {
            return;
        }
        let ch_id = match channel_id_str.parse::<u64>() {
            Ok(v) => v,
            Err(_) => return,
        };
        let channel_id = ChannelId::new(ch_id);
        let embed = sentinel_shared::embeds::neutral_embed(
            "\u{1f550} Ticket ferme automatiquement",
        )
        .description(format!(
            "Ce ticket a ete ferme apres {} jours d'inactivite.",
            timeout_days
        ));
        if let Err(e) = channel_id
            .send_message(
                &ctx.http,
                serenity::builder::CreateMessage::new().embed(embed),
            )
            .await
        {
            warn!(error = %e, "Failed to send auto-close notification");
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        if let Err(e) = channel_id.delete(&ctx.http).await {
            warn!(error = %e, "Failed to delete inactive ticket channel");
        }
        return;
    }

    // Phase 2 sync : ticket ferme depuis le web -> on lock le channel
    // Discord pour eviter les nouveaux messages. Si actor.source != "web",
    // c est notre propre fermeture (boucle), on skip.
    if event_type == "ticket_closed" {
        let source = data
            .get("actor")
            .and_then(|a| a.get("source"))
            .and_then(|s| s.as_str())
            .unwrap_or("");
        if source != "web" {
            return;
        }
        let action_id = data.get("action_id").and_then(|v| v.as_str()).unwrap_or("");
        if action_id.is_empty() {
            return;
        }
        handle_ticket_closed_from_web(ctx, action_id).await;
        return;
    }

    if event_type != "ticket_message" {
        return;
    }

    let ticket_id = match data.get("ticket_id").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => return,
    };
    let author_name = data.get("author_name").and_then(|v| v.as_str()).unwrap_or("Staff");

    let bot_id = ctx.cache.current_user().id;
    for guild_id in ctx.cache.guilds() {
        let channels = match guild_id.channels(&ctx.http).await {
            Ok(c) => c,
            Err(_) => continue,
        };

        for channel in channels.values() {
            if !channel.name.starts_with("ticket-") {
                continue;
            }

            let topic = channel.topic.as_deref().unwrap_or("");
            if let Some(id) = ticket::extract_ticket_id_from_topic(topic) {
                if id == ticket_id {
                    let data_lock = ctx.data.read().await;
                    if let (Some(base), Some(grpc)) = (
                        data_lock.get::<ApiClientKey>(),
                        data_lock.get::<sentinel_shared::grpc_client::GrpcClientKey>(),
                    ) {
                        let api = ApiClient::new(base.clone(), grpc.clone());
                        if let Ok(detail) = api.get_ticket(ticket_id).await {
                            if let Some(last_msg) = detail.messages.last() {
                                if last_msg.author_role == "moderator" {
                                    let already_in_channel = channel.id
                                        .messages(&ctx.http, serenity::all::GetMessages::new().limit(5))
                                        .await
                                        .ok()
                                        .map(|msgs| msgs.iter().any(|m| {
                                            (!m.author.bot && m.content == last_msg.content)
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
