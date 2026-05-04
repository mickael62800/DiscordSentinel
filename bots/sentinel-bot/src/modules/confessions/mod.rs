//! Module confessions : slash command /confess + panel persistant +
//! gestion modales (Submit, Reply, Report) + boutons + edit/delete par
//! admin via slash. API source de verite : tout passe par sentinel-api.

use std::sync::Arc;

use serenity::all::{
    ButtonStyle, CommandDataOptionValue, CommandInteraction, ComponentInteraction, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage, CreateModal,
    ModalInteraction,
};
use serenity::builder::{
    CreateActionRow, CreateButton, CreateEmbed, CreateInputText, CreateMessage,
};
use serenity::model::application::{CommandOptionType, InputTextStyle};
use serenity::model::id::{ChannelId, GuildId, MessageId};
use serenity::prelude::*;
use tracing::{info, warn};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::heartbeat::ApiClientKey;

pub const MODULE_BOT_NAME: &str = "confessions";

// ── Custom IDs ──────────────────────────────────────────────────────────

pub const CID_SUBMIT_BUTTON: &str = "conf_submit";          // bouton panel
pub const CID_REPLY_BUTTON_PREFIX: &str = "conf_reply:";    // conf_reply:<conf_id>
pub const CID_REPORT_BUTTON_PREFIX: &str = "conf_report:";  // conf_report:<conf_id>
pub const CID_SUBMIT_MODAL: &str = "conf_modal_submit";
pub const CID_REPLY_MODAL_PREFIX: &str = "conf_modal_reply:";
pub const CID_REPORT_MODAL_PREFIX: &str = "conf_modal_report:";

pub fn handles_component(cid: &str) -> bool {
    cid == CID_SUBMIT_BUTTON
        || cid.starts_with(CID_REPLY_BUTTON_PREFIX)
        || cid.starts_with(CID_REPORT_BUTTON_PREFIX)
}

pub fn handles_modal(cid: &str) -> bool {
    cid == CID_SUBMIT_MODAL
        || cid.starts_with(CID_REPLY_MODAL_PREFIX)
        || cid.starts_with(CID_REPORT_MODAL_PREFIX)
}

#[allow(dead_code)]
pub fn handles_command(name: &str) -> bool {
    matches!(name, "confess" | "confess-admin")
}

#[allow(dead_code)]
pub fn register_commands() -> Vec<CreateCommand> {
    vec![
        CreateCommand::new("confess")
            .description("Poste une confession anonyme dans le canal configure"),
        CreateCommand::new("confess-admin")
            .description("Administration des confessions (admin only)")
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "deploy-panel",
                    "Poste le bouton 'Poster une confession' dans ce canal")
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "delete",
                    "Supprime une confession par numero")
                    .add_sub_option(
                        CreateCommandOption::new(CommandOptionType::Integer, "number",
                            "Numero de confession (ex: 350)").required(true)
                    )
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "reveal",
                    "Revele l'auteur d'une confession (owner only)")
                    .add_sub_option(
                        CreateCommandOption::new(CommandOptionType::Integer, "number",
                            "Numero de confession").required(true)
                    )
            ),
    ]
}

// ── Slash command dispatcher ────────────────────────────────────────────

pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    let name = command.data.name.as_str();
    if name == "confess" {
        // Ouvre la modale de submission directement
        open_submit_modal(ctx, command).await;
        return;
    }
    if name != "confess-admin" {
        return;
    }
    // Sub-command
    let sub = command.data.options.first();
    let sub_name = sub.map(|o| o.name.as_str()).unwrap_or("");
    match sub_name {
        "deploy-panel" => admin_deploy_panel(ctx, command).await,
        "delete" => admin_delete(ctx, command).await,
        "reveal" => admin_reveal(ctx, command).await,
        _ => reply_ephemeral(ctx, command, "Sous-commande inconnue").await,
    }
}

async fn open_submit_modal(ctx: &Context, command: &CommandInteraction) {
    let modal = CreateModal::new(CID_SUBMIT_MODAL, "Confession anonyme")
        .components(vec![CreateActionRow::InputText(
            CreateInputText::new(InputTextStyle::Paragraph, "Ton message", "content")
                .min_length(1)
                .max_length(2000)
                .required(true),
        )]);
    let resp = CreateInteractionResponse::Modal(modal);
    if let Err(e) = command.create_response(&ctx.http, resp).await {
        warn!(error = %e, "Echec ouverture modale confess");
    }
}

async fn admin_deploy_panel(ctx: &Context, command: &CommandInteraction) {
    let channel = command.channel_id;
    let embed = CreateEmbed::new()
        .title("📝 Confessions anonymes")
        .description(
            "Clique sur le bouton ci-dessous pour poster une confession **anonyme**.\n\
             Personne (sauf le bot) ne saura qui a écrit. Sois respectueux et lis les règles."
        )
        .color(0x5865f2);
    let row = CreateActionRow::Buttons(vec![
        CreateButton::new(CID_SUBMIT_BUTTON)
            .label("Poster une confession")
            .style(ButtonStyle::Primary)
            .emoji('📝'),
    ]);
    let msg = CreateMessage::new().embed(embed).components(vec![row]);
    match channel.send_message(&ctx.http, msg).await {
        Ok(message) => {
            let guild_id = command.guild_id.map(|g| g.to_string()).unwrap_or_default();
            // Sauvegarde le panel_message_id et channel_id en config
            if let Some(api) = api_client(ctx).await {
                let body = serde_json::json!({
                    "guild_id": guild_id,
                    "enabled": true,
                    "channel_id": channel.to_string(),
                    "panel_message_id": message.id.to_string(),
                    "cooldown_secs": 60,
                    "max_per_day": 20,
                    "min_chars": 5,
                    "max_chars": 2000,
                    "automod_enabled": true,
                    "banned_user_ids": Vec::<String>::new(),
                });
                let _: Result<serde_json::Value, _> = api.post_json("/api/confessions/config", &body).await;
            }
            reply_ephemeral(ctx, command, "✅ Panel deploye dans ce canal.").await;
        }
        Err(e) => {
            warn!(error = %e, "Echec deploy panel");
            reply_ephemeral(ctx, command, &format!("Erreur : {e}")).await;
        }
    }
}

async fn admin_delete(ctx: &Context, command: &CommandInteraction) {
    let number = sub_int_option(command, "number").unwrap_or(0);
    if number <= 0 {
        reply_ephemeral(ctx, command, "Numero invalide").await;
        return;
    }
    let guild_id = command.guild_id.map(|g| g.to_string()).unwrap_or_default();
    let api = match api_client(ctx).await {
        Some(a) => a,
        None => return,
    };
    // Trouve la confession par numero
    let path = format!("/api/confessions/{}/list?limit=500&include_deleted=false", guild_id);
    let list: Result<Vec<serde_json::Value>, String> = api.get_json(&path).await;
    let list = match list {
        Ok(l) => l,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur : {e}")).await;
            return;
        }
    };
    let target = list
        .iter()
        .find(|c| c.get("public_number").and_then(|v| v.as_i64()) == Some(number));
    let target = match target {
        Some(t) => t,
        None => {
            reply_ephemeral(ctx, command, &format!("Confession #{} introuvable", number)).await;
            return;
        }
    };
    let id = target.get("id").and_then(|v| v.as_str()).unwrap_or("");
    // Pas de delete_json avec body, on poste le payload via post_json sur
    // un endpoint patch-like. Notre route DELETE accepte un body : on fait
    // un appel HTTP direct via reqwest.
    let url = format!("{}/api/confessions/by-id/{}", api.base_url(), id);
    let body = serde_json::json!({
        "deleted_by": command.user.id.to_string(),
        "reason": "Supprimee par admin via slash command",
    });
    let req = api
        .client()
        .request(reqwest::Method::DELETE, &url)
        .json(&body);
    let req = api.auth(req);
    let resp: Result<(), String> = req.send().await.map(|_| ()).map_err(|e| e.to_string());
    match resp {
        Ok(_) => {
            // Supprime aussi le message Discord (best-effort)
            if let (Some(ch), Some(msg)) = (
                target.get("channel_id").and_then(|v| v.as_str()),
                target.get("message_id").and_then(|v| v.as_str()),
            ) {
                if let (Ok(c), Ok(m)) = (ch.parse::<u64>(), msg.parse::<u64>()) {
                    let _ = ChannelId::new(c)
                        .delete_message(&ctx.http, MessageId::new(m))
                        .await;
                }
            }
            reply_ephemeral(ctx, command, &format!("✅ Confession #{} supprimee", number)).await;
        }
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur : {e}")).await;
        }
    }
}

async fn admin_reveal(ctx: &Context, command: &CommandInteraction) {
    let number = sub_int_option(command, "number").unwrap_or(0);
    if number <= 0 {
        reply_ephemeral(ctx, command, "Numero invalide").await;
        return;
    }
    let guild_id = command.guild_id.map(|g| g.to_string()).unwrap_or_default();
    let api = match api_client(ctx).await {
        Some(a) => a,
        None => return,
    };
    let path = format!("/api/confessions/{}/list?limit=500&include_deleted=true", guild_id);
    let list: Result<Vec<serde_json::Value>, String> = api.get_json(&path).await;
    let list = match list {
        Ok(l) => l,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur : {e}")).await;
            return;
        }
    };
    let target = list
        .iter()
        .find(|c| c.get("public_number").and_then(|v| v.as_i64()) == Some(number));
    match target {
        Some(t) => {
            let author = t.get("author_user_id").and_then(|v| v.as_str()).unwrap_or("?");
            reply_ephemeral(
                ctx,
                command,
                &format!("Confession #{} → auteur : <@{}> (`{}`)", number, author, author),
            )
            .await;
        }
        None => {
            reply_ephemeral(ctx, command, &format!("Confession #{} introuvable", number)).await;
        }
    }
}

fn sub_int_option(command: &CommandInteraction, name: &str) -> Option<i64> {
    let sub = command.data.options.first()?;
    if let CommandDataOptionValue::SubCommand(opts) = &sub.value {
        for o in opts {
            if o.name == name {
                if let CommandDataOptionValue::Integer(v) = &o.value {
                    return Some(*v);
                }
            }
        }
    }
    None
}

// ── Component (boutons) ─────────────────────────────────────────────────

pub async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    let cid = component.data.custom_id.as_str();
    if cid == CID_SUBMIT_BUTTON {
        open_submit_modal_from_component(ctx, component).await;
        return;
    }
    if let Some(conf_id) = cid.strip_prefix(CID_REPLY_BUTTON_PREFIX) {
        open_reply_modal(ctx, component, conf_id).await;
        return;
    }
    if let Some(conf_id) = cid.strip_prefix(CID_REPORT_BUTTON_PREFIX) {
        open_report_modal(ctx, component, conf_id).await;
        return;
    }
}

async fn open_submit_modal_from_component(ctx: &Context, component: &ComponentInteraction) {
    let modal = CreateModal::new(CID_SUBMIT_MODAL, "Confession anonyme")
        .components(vec![CreateActionRow::InputText(
            CreateInputText::new(InputTextStyle::Paragraph, "Ton message", "content")
                .min_length(1)
                .max_length(2000)
                .required(true),
        )]);
    let resp = CreateInteractionResponse::Modal(modal);
    if let Err(e) = component.create_response(&ctx.http, resp).await {
        warn!(error = %e, "Echec ouverture modale submit");
    }
}

async fn open_reply_modal(ctx: &Context, component: &ComponentInteraction, conf_id: &str) {
    let modal = CreateModal::new(
        format!("{}{}", CID_REPLY_MODAL_PREFIX, conf_id),
        "Reponse anonyme",
    )
    .components(vec![CreateActionRow::InputText(
        CreateInputText::new(InputTextStyle::Paragraph, "Ta reponse", "content")
            .min_length(1)
            .max_length(2000)
            .required(true),
    )]);
    let resp = CreateInteractionResponse::Modal(modal);
    if let Err(e) = component.create_response(&ctx.http, resp).await {
        warn!(error = %e, "Echec ouverture modale reply");
    }
}

async fn open_report_modal(ctx: &Context, component: &ComponentInteraction, conf_id: &str) {
    let modal = CreateModal::new(
        format!("{}{}", CID_REPORT_MODAL_PREFIX, conf_id),
        "Signaler cette confession",
    )
    .components(vec![CreateActionRow::InputText(
        CreateInputText::new(InputTextStyle::Paragraph, "Raison du signalement", "reason")
            .min_length(3)
            .max_length(500)
            .required(true),
    )]);
    let resp = CreateInteractionResponse::Modal(modal);
    if let Err(e) = component.create_response(&ctx.http, resp).await {
        warn!(error = %e, "Echec ouverture modale report");
    }
}

// ── Modal (submit / reply / report) ──────────────────────────────────────

pub async fn on_modal(ctx: &Context, modal: &ModalInteraction) {
    let cid = modal.data.custom_id.as_str();
    if cid == CID_SUBMIT_MODAL {
        handle_submit(ctx, modal).await;
        return;
    }
    if let Some(conf_id) = cid.strip_prefix(CID_REPLY_MODAL_PREFIX) {
        handle_reply(ctx, modal, conf_id).await;
        return;
    }
    if let Some(conf_id) = cid.strip_prefix(CID_REPORT_MODAL_PREFIX) {
        handle_report(ctx, modal, conf_id).await;
        return;
    }
}

async fn handle_submit(ctx: &Context, modal: &ModalInteraction) {
    let content = extract_input(modal, "content").unwrap_or_default();
    let guild_id = modal.guild_id.map(|g| g.to_string()).unwrap_or_default();
    let user_id = modal.user.id.to_string();

    let api = match api_client(ctx).await {
        Some(a) => a,
        None => return,
    };

    // 1. Cree la confession via API
    let create_body = serde_json::json!({
        "guild_id": guild_id,
        "author_user_id": user_id,
        "content": content,
    });
    let created: Result<serde_json::Value, String> =
        api.post_json("/api/confessions", &create_body).await;
    let created = match created {
        Ok(v) => v,
        Err(e) => {
            modal_reply_ephemeral(ctx, modal, &format!("❌ {}", e)).await;
            return;
        }
    };
    let id = created.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let public_number = created.get("public_number").and_then(|v| v.as_i64()).unwrap_or(0);

    // 2. Recupere la config pour le channel_id ou poster
    let cfg_path = format!("/api/confessions/config/{}", guild_id);
    let cfg: Result<serde_json::Value, String> = api.get_json(&cfg_path).await;
    let channel_id_str = cfg
        .ok()
        .and_then(|c| {
            c.get("channel_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_default();
    if channel_id_str.is_empty() {
        modal_reply_ephemeral(
            ctx,
            modal,
            "❌ Aucun salon de confession configure. Lance /confess-admin deploy-panel.",
        )
        .await;
        return;
    }
    let ch = match channel_id_str.parse::<u64>() {
        Ok(c) => ChannelId::new(c),
        Err(_) => return,
    };

    // 3. Poste l'embed sur Discord
    let embed = CreateEmbed::new()
        .author(serenity::builder::CreateEmbedAuthor::new(format!(
            "Confession anonyme (#{})",
            public_number
        )))
        .description(&content)
        .color(0xff5e5e);
    let row = CreateActionRow::Buttons(vec![
        CreateButton::new(format!("{}{}", CID_REPLY_BUTTON_PREFIX, id))
            .label("Répondre")
            .style(ButtonStyle::Secondary)
            .emoji('💬'),
        CreateButton::new(format!("{}{}", CID_REPORT_BUTTON_PREFIX, id))
            .label("Signaler")
            .style(ButtonStyle::Secondary)
            .emoji('🚩'),
    ]);
    let msg_payload = CreateMessage::new().embed(embed).components(vec![row]);
    let posted = ch.send_message(&ctx.http, msg_payload).await;
    let posted = match posted {
        Ok(m) => m,
        Err(e) => {
            modal_reply_ephemeral(ctx, modal, &format!("❌ Erreur post Discord : {e}")).await;
            return;
        }
    };

    // 4. Cree le thread "Confession Replies (#N)"
    let thread_name = format!("Confession Replies (#{})", public_number);
    let thread = ch
        .create_thread_from_message(
            &ctx.http,
            posted.id,
            serenity::builder::CreateThread::new(thread_name),
        )
        .await
        .ok();
    let thread_id = thread.as_ref().map(|t| t.id.to_string());

    // 5. Update message_refs cote API
    let refs_body = serde_json::json!({
        "message_id": posted.id.to_string(),
        "channel_id": ch.to_string(),
        "thread_id": thread_id,
    });
    let _: Result<serde_json::Value, String> = api
        .post_json(&format!("/api/confessions/by-id/{}/message-refs", id), &refs_body)
        .await;

    modal_reply_ephemeral(
        ctx,
        modal,
        &format!("✅ Confession #{} postee anonymement", public_number),
    )
    .await;
}

async fn handle_reply(ctx: &Context, modal: &ModalInteraction, conf_id: &str) {
    let content = extract_input(modal, "content").unwrap_or_default();
    let user_id = modal.user.id.to_string();
    let api = match api_client(ctx).await {
        Some(a) => a,
        None => return,
    };

    let body = serde_json::json!({
        "author_user_id": user_id,
        "content": content,
        "is_anonymous": true,
    });
    let created: Result<serde_json::Value, String> = api
        .post_json(
            &format!("/api/confessions/by-id/{}/replies", conf_id),
            &body,
        )
        .await;
    let created = match created {
        Ok(v) => v,
        Err(e) => {
            modal_reply_ephemeral(ctx, modal, &format!("❌ {}", e)).await;
            return;
        }
    };
    let reply_id = created.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let public_number = created.get("public_number").and_then(|v| v.as_i64()).unwrap_or(0);

    // Recupere la confession pour avoir le thread_id
    let conf: Result<serde_json::Value, String> = api
        .get_json(&format!("/api/confessions/by-id/{}", conf_id))
        .await;
    let thread_id_str = conf
        .ok()
        .and_then(|c| c.get("thread_id").and_then(|v| v.as_str()).map(|s| s.to_string()));
    let Some(thread_id) = thread_id_str else {
        modal_reply_ephemeral(ctx, modal, "❌ Thread introuvable").await;
        return;
    };
    let ch = match thread_id.parse::<u64>() {
        Ok(c) => ChannelId::new(c),
        Err(_) => return,
    };
    let embed = CreateEmbed::new()
        .author(serenity::builder::CreateEmbedAuthor::new(format!(
            "Réponse anonyme (#{})",
            public_number
        )))
        .description(&content)
        .color(0xff5e5e);
    let posted = ch
        .send_message(&ctx.http, CreateMessage::new().embed(embed))
        .await;
    if let Ok(m) = posted {
        let body = serde_json::json!({ "message_id": m.id.to_string() });
        let _: Result<serde_json::Value, String> = api
            .post_json(&format!("/api/confessions/replies/{}/message-id", reply_id), &body)
            .await;
    }
    modal_reply_ephemeral(ctx, modal, "✅ Reponse anonyme postee").await;
}

async fn handle_report(ctx: &Context, modal: &ModalInteraction, conf_id: &str) {
    let reason = extract_input(modal, "reason").unwrap_or_default();
    let guild_id = modal.guild_id.map(|g| g.to_string()).unwrap_or_default();
    let api = match api_client(ctx).await {
        Some(a) => a,
        None => return,
    };
    let body = serde_json::json!({
        "guild_id": guild_id,
        "confession_id": conf_id,
        "reply_id": null,
        "reporter_user_id": modal.user.id.to_string(),
        "reason": reason,
    });
    let resp: Result<serde_json::Value, String> = api.post_json("/api/confessions/reports", &body).await;
    match resp {
        Ok(_) => modal_reply_ephemeral(ctx, modal, "✅ Signalement transmis aux moderateurs").await,
        Err(e) => modal_reply_ephemeral(ctx, modal, &format!("❌ {}", e)).await,
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────

fn extract_input(modal: &ModalInteraction, field_id: &str) -> Option<String> {
    for row in &modal.data.components {
        for c in &row.components {
            if let serenity::all::ActionRowComponent::InputText(it) = c {
                if it.custom_id == field_id {
                    return it.value.clone();
                }
            }
        }
    }
    None
}

async fn api_client(ctx: &Context) -> Option<Arc<BaseApiClient>> {
    let data = ctx.data.read().await;
    data.get::<ApiClientKey>().cloned()
}

async fn reply_ephemeral(ctx: &Context, command: &CommandInteraction, content: &str) {
    let resp = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(content)
            .ephemeral(true),
    );
    if let Err(e) = command.create_response(&ctx.http, resp).await {
        warn!(error = %e, "Echec reply ephemere confess");
    }
}

async fn modal_reply_ephemeral(ctx: &Context, modal: &ModalInteraction, content: &str) {
    let resp = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(content)
            .ephemeral(true),
    );
    if let Err(e) = modal.create_response(&ctx.http, resp).await {
        warn!(error = %e, "Echec reply ephemere modal confess");
    }
}

// Used externally via consumer
#[allow(dead_code)]
pub fn module_name() -> &'static str {
    MODULE_BOT_NAME
}

#[allow(dead_code)]
pub fn ensure_used(_ctx: &Context, _g: GuildId) {
    info!("confessions module loaded");
}

// ── Consumer Redis stream pour sync bidirectionnelle Web -> Discord ─────

/// Spawn le consumer durable Redis stream sentinel:events filtre sur les
/// events "confession_deleted" et "confession_reply_deleted". Quand un
/// admin supprime une confession via la page web, l'API broadcast un event
/// avec message_id+channel_id, et ce consumer supprime le message Discord
/// pour garder la sync.
pub fn spawn_consumer(ctx: Context) {
    tokio::spawn(async move {
        let consumer = sentinel_shared::event_bus::default_consumer_name();
        sentinel_shared::event_bus::listen_stream_group(
            "sentinel-bot-confessions".to_string(),
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
    let event = envelope.get("event").and_then(|v| v.as_str()).unwrap_or("");
    let data = match envelope.get("data") {
        Some(d) => d.clone(),
        None => return,
    };
    match event {
        "confession_deleted" => {
            let channel_id_str = data
                .get("channel_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let message_id_str = data
                .get("message_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            if channel_id_str.is_empty() || message_id_str.is_empty() {
                return;
            }
            let (Ok(c), Ok(m)) = (channel_id_str.parse::<u64>(), message_id_str.parse::<u64>())
            else {
                return;
            };
            let ch = ChannelId::new(c);
            let mid = MessageId::new(m);
            // Idempotent : si deja supprime, on ignore l'erreur 404.
            match ch.delete_message(&ctx.http, mid).await {
                Ok(_) => info!(channel_id = c, message_id = m, "Confession message deleted (sync from web)"),
                Err(e) => {
                    let s = e.to_string();
                    if !s.contains("404") {
                        warn!(error = %e, "Echec delete message confession (sync web)");
                    }
                }
            }
        }
        "confession_reply_deleted" => {
            // Le reply est dans le thread - on doit retrouver le channel.
            // L'API broadcast n'envoie pas le channel_id du thread, donc on
            // recupere via la confession parent.
            let confession_id = data
                .get("confession_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let message_id_str = data
                .get("message_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            if confession_id.is_empty() || message_id_str.is_empty() {
                return;
            }
            let api = match api_client(ctx).await {
                Some(a) => a,
                None => return,
            };
            let conf: Result<serde_json::Value, String> = api
                .get_json(&format!("/api/confessions/by-id/{}", confession_id))
                .await;
            let thread_id_str = match conf {
                Ok(c) => c
                    .get("thread_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                Err(_) => return,
            };
            if thread_id_str.is_empty() {
                return;
            }
            let (Ok(c), Ok(m)) = (thread_id_str.parse::<u64>(), message_id_str.parse::<u64>())
            else {
                return;
            };
            let ch = ChannelId::new(c);
            let mid = MessageId::new(m);
            match ch.delete_message(&ctx.http, mid).await {
                Ok(_) => info!(thread_id = c, message_id = m, "Reply message deleted (sync web)"),
                Err(e) => {
                    let s = e.to_string();
                    if !s.contains("404") {
                        warn!(error = %e, "Echec delete reply message (sync web)");
                    }
                }
            }
        }
        _ => {}
    }
}
