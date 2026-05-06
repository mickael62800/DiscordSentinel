//! Module moderation — 21 commandes slash + boutons + autocomplete + consumer Redis
//! (ex moderation-bot).

pub const MODULE_BOT_NAME: &str = "moderation-bot";

pub mod api_client;
pub mod commands;
mod pending_actions;
pub mod reason_templates;
mod redis_events;
pub mod risk_check;
mod risky_buttons;

use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;
use serenity::all::{
    AutocompleteChoice, CommandDataOptionValue, CommandInteraction, ComponentInteraction,
    Context, CreateAutocompleteResponse, CreateInteractionResponse,
};
use serenity::builder::CreateCommand;
use serenity::prelude::*;
use tracing::warn;

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::discord_helpers::{
    is_module_enabled_or_reply_command, is_module_enabled_or_reply_component,
};
use sentinel_shared::heartbeat::ApiClientKey;

use api_client::{ApiClient, ModerationAction};

// ── TypeMapKeys ──

pub struct ModerationApiKey;
impl TypeMapKey for ModerationApiKey {
    type Value = Arc<ApiClient>;
}

#[allow(dead_code)]
pub struct PendingAction {
    pub action: ModerationAction,
    pub moderator_id: String,
    pub proposed_at: Instant,
}

pub struct PendingActionsKey;
impl TypeMapKey for PendingActionsKey {
    type Value = DashMap<String, PendingAction>;
}

pub const APPROVE_PREFIX: &str = "sentinel_mod_approve_";
pub const REJECT_PREFIX: &str = "sentinel_mod_reject_";

// ── Init TypeMapKeys ──

pub fn init_typemap(
    data: &mut serenity::prelude::TypeMap,
    api: &Arc<sentinel_shared::api_client::BaseApiClient>,
    grpc: &Arc<sentinel_shared::grpc_client::SentinelGrpcClient>,
) {
    data.insert::<ModerationApiKey>(Arc::new(ApiClient::new(
        Arc::clone(api),
        Arc::clone(grpc),
    )));
    data.insert::<PendingActionsKey>(DashMap::new());
    data.insert::<risk_check::RiskyPendingKey>(DashMap::new());
}

// ── Slash commands ──

pub fn register_commands() -> Vec<CreateCommand> {
    commands::all()
}

pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    let cmd_name = command.data.name.clone();
    let moderator = command.user.name.clone();
    let guild_id = command.guild_id.map(|g| g.to_string()).unwrap_or_default();

    if !is_module_enabled_or_reply_command(ctx, command, MODULE_BOT_NAME).await {
        return;
    }

    match cmd_name.as_str() {
        "warn" => commands::warn::handle(ctx, command).await,
        "unwarn" => commands::unwarn::handle(ctx, command).await,
        "mute" => commands::mute::handle(ctx, command).await,
        "unmute" => commands::mute::handle_unmute(ctx, command).await,
        "ban" => commands::ban::handle(ctx, command).await,
        "unban" => commands::ban::handle_unban(ctx, command).await,
        "history" => commands::history::handle(ctx, command).await,
        "note" => commands::notes::handle(ctx, command).await,
        "call" => commands::call::handle(ctx, command).await,
        "context" => commands::context::handle(ctx, command).await,
        "appeal" => commands::appeal::handle(ctx, command).await,
        "expirations" => commands::expirations::handle(ctx, command).await,
        "compare" => commands::compare::handle(ctx, command).await,
        "modstats" => commands::modstats::handle(ctx, command).await,
        "evidence" => commands::evidence::handle(ctx, command).await,
        "review" => commands::review::handle(ctx, command).await,
        "template" => commands::template::handle(ctx, command).await,
        "transcript" => commands::transcript::handle(ctx, command).await,
        "export" => commands::export::handle(ctx, command).await,
        "massmute" => commands::mass::handle_massmute(ctx, command).await,
        "massban" => commands::mass::handle_massban(ctx, command).await,
        _ => {}
    }

    let data = ctx.data.read().await;
    if let Some(api) = data.get::<ApiClientKey>() {
        api.send_log(
            "info",
            &guild_id,
            &format!("Commande /{} executee par {}", cmd_name, moderator),
        );
    }
}

// ── Component interactions ──

pub fn handles_component(cid: &str) -> bool {
    cid.starts_with(commands::unwarn::UNWARN_PREFIX)
        || cid.starts_with(commands::call::CALL_CLOSE_PREFIX)
        || cid.starts_with(commands::appeal::APPEAL_PREFIX)
        || cid.starts_with(APPROVE_PREFIX)
        || cid.starts_with(REJECT_PREFIX)
        || cid.starts_with(risk_check::CONFIRM_PREFIX)
        || cid.starts_with(risk_check::CANCEL_PREFIX)
}

pub async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    if !is_module_enabled_or_reply_component(ctx, component, MODULE_BOT_NAME).await {
        return;
    }
    let custom_id = &component.data.custom_id;

    if custom_id.starts_with(commands::unwarn::UNWARN_PREFIX) {
        commands::unwarn::handle_button(ctx, component).await;
    } else if custom_id.starts_with(commands::call::CALL_CLOSE_PREFIX) {
        commands::call::handle_close(ctx, component).await;
    } else if custom_id.starts_with(commands::appeal::APPEAL_PREFIX) {
        commands::appeal::handle_appeal_button(ctx, component).await;
    } else if custom_id.starts_with(APPROVE_PREFIX) {
        pending_actions::handle_approve(ctx, component).await;
    } else if custom_id.starts_with(REJECT_PREFIX) {
        pending_actions::handle_reject(ctx, component).await;
    } else if custom_id.starts_with(risk_check::CONFIRM_PREFIX) {
        risky_buttons::handle_risky_confirm(ctx, component).await;
    } else if custom_id.starts_with(risk_check::CANCEL_PREFIX) {
        risky_buttons::handle_risky_cancel(ctx, component).await;
    }
}

// ── Autocomplete (reason templates) ──

pub fn handles_autocomplete(cmd_name: &str) -> bool {
    matches!(cmd_name, "warn" | "mute" | "ban")
}

pub async fn handle_autocomplete(ctx: &Context, autocomplete: &CommandInteraction) {
    let guild_id = autocomplete.guild_id.map(|g| g.to_string()).unwrap_or_default();

    let current_input = autocomplete
        .data
        .options
        .iter()
        .find(|o| o.name == "reason")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("");

    let templates_raw = {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            let gc = match base.get_guild_config_for(&guild_id, MODULE_BOT_NAME).await {
                Ok(c) => c,
                Err(e) => {
                    warn!(error = %e, "Failed to fetch guild config for reason templates");
                    std::collections::HashMap::new()
                }
            };
            BaseApiClient::config_or(&gc, "reason_templates", "")
        } else {
            String::new()
        }
    };

    let templates = reason_templates::parse_templates(&templates_raw);
    let filtered = reason_templates::filter_templates(&templates, current_input);

    let choices: Vec<AutocompleteChoice> = filtered
        .iter()
        .map(|t| AutocompleteChoice::new(&t.label, serde_json::Value::String(t.reason.clone())))
        .collect();

    let response = CreateAutocompleteResponse::new().set_choices(choices);

    if let Err(e) = autocomplete
        .create_response(&ctx.http, CreateInteractionResponse::Autocomplete(response))
        .await
    {
        warn!(error = %e, "Failed to send autocomplete response");
    }
}

// ── Helpers : appeal channel (log_channel_id deja gere par log_to_channel) ──

/// Poste un embed dans `appeal_channel_id` configure pour la guild.
/// Utilise pour notifier les mods quand un appel de sanction est cree.
/// Best-effort : si la cle est vide ou l'envoi echoue, log warn.
pub async fn post_to_appeal_channel(
    ctx: &Context,
    guild_id: &str,
    embed: serenity::builder::CreateEmbed,
) {
    let cfg = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(api) => api.get_guild_config_for(guild_id, MODULE_BOT_NAME).await.unwrap_or_default(),
            None => return,
        }
    };
    let channel_id = match cfg.get("appeal_channel_id").and_then(|s| s.parse::<u64>().ok()) {
        Some(n) if n > 0 => n,
        _ => return,
    };
    let ch = serenity::model::id::ChannelId::new(channel_id);
    if let Err(e) = ch.send_message(
        &ctx.http,
        serenity::builder::CreateMessage::new().embed(embed),
    ).await {
        warn!(error = %e, channel_id, "Echec post appeal channel");
    }
}

// ── Background tasks ──

/// Spawn le consumer Redis des events moderation (appele depuis ready).
pub fn spawn_background(ctx: Context) {
    tokio::spawn(async move {
        let consumer = sentinel_shared::event_bus::default_consumer_name();
        sentinel_shared::event_bus::listen_stream_group(
            "moderation-bot".to_string(),
            consumer,
            move |payload| {
                let ctx = ctx.clone();
                async move {
                    redis_events::handle_redis_moderation_event(&ctx, &payload).await;
                }
            },
        )
        .await;
    });
}
