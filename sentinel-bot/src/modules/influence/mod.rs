//! Module bot du jeu « Influence » (cf. docs/Nouveau jeux/ARCHITECTURE.md).
//!
//! Phases 1-3 : profil/capitaux, organisations, votes d'org, conversions, lois.

use serenity::all::{
    ChannelId, CommandInteraction, ComponentInteraction, Context, CreateCommand, EditMessage,
    CreateInteractionResponse, CreateInteractionResponseMessage, MessageId,
};

use crate::shared::discord_helpers::is_module_enabled_or_reply_command;
use crate::shared::heartbeat::ApiClientKey;

pub mod api_client;
pub mod commands;

pub const MODULE_BOT_NAME: &str = "influence-bot";

/// Commandes slash exposees par le module.
pub fn register_commands() -> Vec<CreateCommand> {
    vec![
        commands::profil::register(),
        commands::org::register(),
        commands::vote::register(),
        commands::capital::register(),
        commands::transfert::register(),
        commands::loi::register(),
        commands::information::register_enquete(),
        commands::information::register_dossier(),
        commands::information::register_reveler(),
    ]
}

/// `true` si la commande appartient a ce module.
pub fn handles_command(name: &str) -> bool {
    matches!(
        name,
        "influence-profil"
            | "org"
            | "vote"
            | "capital"
            | "transfert"
            | "loi"
            | "enquete"
            | "dossier"
            | "reveler"
    )
}

/// Dispatch d'une commande du module.
pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    if !is_module_enabled_or_reply_command(ctx, command, MODULE_BOT_NAME).await {
        return;
    }
    match command.data.name.as_str() {
        "influence-profil" => commands::profil::handle(ctx, command).await,
        "org" => commands::org::handle(ctx, command).await,
        "vote" => commands::vote::handle(ctx, command).await,
        "capital" => commands::capital::handle(ctx, command).await,
        "transfert" => commands::transfert::handle(ctx, command).await,
        "loi" => commands::loi::handle(ctx, command).await,
        "enquete" => commands::information::handle_enquete(ctx, command).await,
        "dossier" => commands::information::handle_dossier(ctx, command).await,
        "reveler" => commands::information::handle_reveler(ctx, command).await,
        _ => {}
    }
}

/// `true` si le composant (bouton) appartient a ce module.
pub fn handles_component(cid: &str) -> bool {
    cid.starts_with(commands::vote::PREFIX) || cid.starts_with(commands::loi::PREFIX)
}

/// Dispatch d'un composant (boutons de vote d'org ou de loi).
pub async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    let cid = component.data.custom_id.clone();
    if cid.starts_with(commands::loi::PREFIX) {
        on_law_component(ctx, component, &cid).await;
    } else {
        on_motion_component(ctx, component, &cid).await;
    }
}

async fn on_motion_component(ctx: &Context, component: &ComponentInteraction, cid: &str) {
    let rest = match cid.strip_prefix(commands::vote::PREFIX) {
        Some(r) => r,
        None => return,
    };
    let Some((motion_id, action)) = rest.rsplit_once(':') else {
        return;
    };
    let Some(guild_id) = component.guild_id.map(|g| g.to_string()) else {
        return;
    };
    let api = match api(ctx).await {
        Some(a) => a,
        None => return,
    };
    let user_id = component.user.id.to_string();
    let result = if action == "close" {
        api_client::close_motion(&api, &guild_id, motion_id, &user_id).await
    } else {
        api_client::cast_vote(&api, &guild_id, motion_id, &user_id, &component.user.name, action)
            .await
    };
    match result {
        Ok(state) => {
            let closed = state.status != "ouverte";
            let resp = CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(commands::vote::build_embed(&state))
                    .components(commands::vote::vote_rows(motion_id, closed)),
            );
            let _ = component.create_response(&ctx.http, resp).await;
        }
        Err(e) => reply_error(ctx, component, &e).await,
    }
}

async fn on_law_component(ctx: &Context, component: &ComponentInteraction, cid: &str) {
    let rest = match cid.strip_prefix(commands::loi::PREFIX) {
        Some(r) => r,
        None => return,
    };
    let Some((law_id, action)) = rest.rsplit_once(':') else {
        return;
    };
    let Some(guild_id) = component.guild_id.map(|g| g.to_string()) else {
        return;
    };
    let api = match api(ctx).await {
        Some(a) => a,
        None => return,
    };
    match api_client::law_vote(
        &api,
        &guild_id,
        law_id,
        &component.user.id.to_string(),
        &component.user.name,
        action,
    )
    .await
    {
        Ok(state) => {
            let closed = state.status != "vote";
            let resp = CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(commands::loi::build_embed(&state))
                    .components(commands::loi::vote_rows(law_id, closed)),
            );
            let _ = component.create_response(&ctx.http, resp).await;
        }
        Err(e) => reply_error(ctx, component, &e).await,
    }
}

async fn reply_error(ctx: &Context, component: &ComponentInteraction, msg: &str) {
    let resp = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(format!("⚠️ {msg}"))
            .ephemeral(true),
    );
    let _ = component.create_response(&ctx.http, resp).await;
}

async fn api(ctx: &Context) -> Option<std::sync::Arc<crate::shared::api_client::BaseApiClient>> {
    ctx.data.read().await.get::<ApiClientKey>().cloned()
}

// ── Consumer d'evenements : cloture de loi par le worker ──

/// Spawn le consumer Redis (cf. game_portal). A appeler dans `ready`.
pub fn spawn(ctx: Context) {
    tokio::spawn(async move {
        let consumer = crate::shared::event_bus::default_consumer_name();
        crate::shared::event_bus::listen_stream_group(
            "sentinel-bot-influence".to_string(),
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
    let env: serde_json::Value = match serde_json::from_str(payload_json) {
        Ok(v) => v,
        Err(_) => return,
    };
    let event = env.get("event").and_then(|v| v.as_str());
    let data = match env.get("data") {
        Some(d) => d,
        None => return,
    };
    match event {
        Some("influence_law_closed") => on_law_closed(ctx, data).await,
        Some("influence_investigation_done") => on_investigation_done(ctx, data).await,
        _ => {}
    }
}

/// Enquete resolue : notifie l'initiateur en message prive.
async fn on_investigation_done(ctx: &Context, data: &serde_json::Value) {
    let (Some(user_id), Some(target), Some(subject), Some(success)) = (
        data.get("initiator_user_id").and_then(|v| v.as_str()),
        data.get("target_username").and_then(|v| v.as_str()),
        data.get("subject").and_then(|v| v.as_str()),
        data.get("success").and_then(|v| v.as_bool()),
    ) else {
        return;
    };
    let Ok(uid) = user_id.parse::<u64>() else { return };
    let msg = if success {
        format!(
            "🔎 Ton enquete sur **{target}** (« {subject} ») a **abouti** ! Consulte ton `/dossier` — tu peux la `/reveler` quand tu veux."
        )
    } else {
        format!("🔎 Ton enquete sur **{target}** (« {subject} ») n'a **rien donne** cette fois.")
    };
    if let Ok(channel) = serenity::model::id::UserId::new(uid).create_dm_channel(&ctx.http).await {
        let _ = channel
            .send_message(
                &ctx.http,
                serenity::all::CreateMessage::new().content(msg),
            )
            .await;
    }
}

async fn on_law_closed(ctx: &Context, data: &serde_json::Value) {
    let (Some(guild_id), Some(law_id), Some(channel_id), Some(message_id)) = (
        data.get("guild_id").and_then(|v| v.as_str()),
        data.get("law_id").and_then(|v| v.as_str()),
        data.get("channel_id").and_then(|v| v.as_str()),
        data.get("message_id").and_then(|v| v.as_str()),
    ) else {
        return;
    };

    let Some(api) = api(ctx).await else { return };
    let Ok(state) = api_client::law_state(&api, guild_id, law_id).await else {
        return;
    };
    let (Ok(chan), Ok(mid)) = (channel_id.parse::<u64>(), message_id.parse::<u64>()) else {
        return;
    };

    // Edite le message : embed final + retrait des boutons.
    let edit = EditMessage::new()
        .embed(commands::loi::build_embed(&state))
        .components(vec![]);
    let _ = ChannelId::new(chan)
        .edit_message(&ctx.http, MessageId::new(mid), edit)
        .await;
}
