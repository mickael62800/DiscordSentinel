//! Module bot du jeu « Influence » (cf. docs/Nouveau jeux/ARCHITECTURE.md).
//!
//! Phase 1 (MVP) : `/influence-profil`, `/org` (organisations), `/vote`.

use serenity::all::{
    CommandInteraction, ComponentInteraction, Context, CreateCommand, CreateInteractionResponse,
    CreateInteractionResponseMessage,
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
    ]
}

/// `true` si la commande appartient a ce module.
pub fn handles_command(name: &str) -> bool {
    matches!(
        name,
        "influence-profil" | "org" | "vote" | "capital" | "transfert"
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
        _ => {}
    }
}

/// `true` si le composant (bouton) appartient a ce module.
pub fn handles_component(cid: &str) -> bool {
    cid.starts_with(commands::vote::PREFIX)
}

/// Dispatch d'un composant (boutons de vote).
pub async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    let cid = component.data.custom_id.clone();
    // Format : inf_vote:<motion_id>:<action>
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

    let api = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(a) => a.clone(),
            None => return,
        }
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
            // Met a jour le message : embed + boutons (retires si clos).
            let closed = state.status != "ouverte";
            let resp = CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(commands::vote::build_embed(&state))
                    .components(commands::vote::vote_rows(motion_id, closed)),
            );
            if let Err(e) = component.create_response(&ctx.http, resp).await {
                tracing::warn!(error = %e, "Echec maj message de vote");
            }
        }
        Err(e) => {
            // Erreur (non-membre, deja clos...) -> reponse ephemere, message inchange.
            let resp = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(format!("⚠️ {e}"))
                    .ephemeral(true),
            );
            let _ = component.create_response(&ctx.http, resp).await;
        }
    }
}
