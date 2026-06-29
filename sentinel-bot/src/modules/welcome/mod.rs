//! Module welcome — bienvenue/depart (ex welcome-bot).

pub const MODULE_BOT_NAME: &str = "welcome-bot";

pub mod api_client;
pub mod handler;
pub mod template;

use serenity::all::{ComponentInteraction, Context, Member};
use serenity::model::id::GuildId;

pub async fn on_member_add(ctx: &Context, member: &Member) {
    handler::on_member_add(ctx, member).await;
}

pub async fn on_member_remove(
    ctx: &Context,
    guild_id: GuildId,
    user: &serenity::model::user::User,
) {
    handler::on_member_remove(ctx, guild_id, user).await;
}

pub async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    handler::on_component(ctx, component).await;
}

/// Fin du filtrage d'adhesion natif Discord (membership screening) : on
/// attribue le(s) role(s) du reglement, comme via le bouton du bot.
pub async fn on_screening_complete(
    ctx: &Context,
    guild_id: GuildId,
    user_id: serenity::model::id::UserId,
) {
    handler::on_screening_complete(ctx, guild_id, user_id).await;
}

pub async fn on_voice_state_update(
    ctx: &Context,
    old: &Option<serenity::model::voice::VoiceState>,
    new: &serenity::model::voice::VoiceState,
) {
    handler::on_voice_state_update(ctx, old, new).await;
}

pub fn handles_component(custom_id: &str) -> bool {
    custom_id == handler::RULES_ACCEPT_ID
}

/// Spawn le consumer durable (Redis stream). Appele une fois au `ready`.
/// Ecoute `welcome_rules_publish` (bouton "Publier le reglement" du dashboard)
/// et poste le panneau de reglement avec le bouton d'acceptation.
pub fn spawn(ctx: Context) {
    tokio::spawn(async move {
        let consumer = crate::shared::event_bus::default_consumer_name();
        crate::shared::event_bus::listen_stream_group(
            "sentinel-bot-welcome".to_string(),
            consumer,
            move |payload_json| {
                let ctx = ctx.clone();
                async move { handle_event(&ctx, &payload_json).await }
            },
        )
        .await;
    });
}

async fn handle_event(ctx: &Context, payload_json: &str) {
    let envelope: serde_json::Value = match serde_json::from_str(payload_json) {
        Ok(v) => v,
        Err(_) => return,
    };
    if envelope.get("event").and_then(|v| v.as_str()) != Some("welcome_rules_publish") {
        return;
    }
    let guild_id = envelope
        .get("data")
        .and_then(|d| d.get("guild_id"))
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u64>().ok());
    if let Some(g) = guild_id {
        if let Err(e) = handler::publish_rules_panel(ctx, GuildId::new(g)).await {
            tracing::warn!(error = %e, guild = g, "Echec publication panneau reglement");
        }
    }
}
