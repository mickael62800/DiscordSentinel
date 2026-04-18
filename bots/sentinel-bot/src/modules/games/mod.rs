//! Module games — /game et mentions #Jeu (ex game-bot).

pub const MODULE_BOT_NAME: &str = "game-bot";

pub mod api_client;
pub mod commands;
pub mod detector;

use std::sync::Arc;

use serenity::all::{CommandInteraction, Context, CreateCommand, Message};
use tracing::warn;

use sentinel_shared::discord_helpers::{is_module_enabled, is_module_enabled_or_reply_command};
use sentinel_shared::heartbeat::ApiClientKey;

pub fn register_commands() -> Vec<CreateCommand> {
    commands::all()
}

pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    if !is_module_enabled_or_reply_command(ctx, command, MODULE_BOT_NAME).await {
        return;
    }
    if command.data.name == "game" {
        commands::handle(ctx, command).await;
    }
}

/// Detection des mentions #Jeu (extraite du game-bot EventHandler).
pub async fn on_message(ctx: &Context, msg: &Message) {
    let guild_id = match msg.guild_id {
        Some(g) => g,
        None => return,
    };

    if !is_module_enabled(ctx, &guild_id.to_string(), MODULE_BOT_NAME).await {
        return;
    }

    let mut mentions = detector::extract_game_mentions(&msg.content);
    mentions.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    mentions.dedup_by(|a, b| a.to_lowercase() == b.to_lowercase());
    if mentions.is_empty() {
        return;
    }

    let data = ctx.data.read().await;
    let base = match data.get::<ApiClientKey>() {
        Some(b) => Arc::clone(b),
        None => return,
    };
    drop(data);

    let api = api_client::GameApiClient::new(base);
    let guild_id_str = guild_id.to_string();

    for mention in &mentions {
        let game = match api.get_game_by_name(&guild_id_str, mention).await {
            Ok(Some(g)) => g,
            _ => continue,
        };

        let subs = match api.get_subscribers(&guild_id_str, &game.id).await {
            Ok(s) => s,
            Err(e) => { warn!(error = %e, "Erreur get_subscribers"); continue; }
        };

        if subs.is_empty() {
            continue;
        }

        let pings: Vec<String> = subs.iter()
            .filter(|s| s.user_id.as_str() != msg.author.id.to_string())
            .map(|s| format!("<@{}>", s.user_id))
            .collect();

        if !pings.is_empty() {
            let text = format!("**{}** mentionne ! {}", game.game_name, pings.join(" "));
            if let Err(e) = msg.channel_id.say(&ctx.http, &text).await {
                warn!(error = %e, "Echec envoi notification jeu");
            }
        }
    }
}
