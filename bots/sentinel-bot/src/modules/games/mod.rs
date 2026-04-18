//! Module games — /game, /game-admin, panels de reactions, mentions #Jeu.

pub const MODULE_BOT_NAME: &str = "game-bot";

pub mod api_client;
pub mod commands;
pub mod detector;
pub mod emoji;

use std::sync::Arc;

use serenity::all::{CommandInteraction, Context, CreateCommand, Message, Reaction};
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
    if command.data.name == "game" || command.data.name == "game-admin" {
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

/// User a ajoute une reaction sur un message : si c'est un panel, subscribe.
pub async fn on_reaction_add(ctx: &Context, reaction: &Reaction) {
    handle_reaction(ctx, reaction, true).await;
}

/// User a retire une reaction : si c'est un panel, unsubscribe.
pub async fn on_reaction_remove(ctx: &Context, reaction: &Reaction) {
    handle_reaction(ctx, reaction, false).await;
}

async fn handle_reaction(ctx: &Context, reaction: &Reaction, add: bool) {
    // Skip bot
    let user_id = match reaction.user_id {
        Some(u) => u,
        None => return,
    };
    if user_id == ctx.cache.current_user().id {
        return;
    }

    let guild_id = match reaction.guild_id {
        Some(g) => g,
        None => return,
    };

    if !is_module_enabled(ctx, &guild_id.to_string(), MODULE_BOT_NAME).await {
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
    let message_id_str = reaction.message_id.to_string();

    // Est-ce un panel ?
    let panel = match api.find_panel_by_message(&guild_id_str, &message_id_str).await {
        Ok(Some(p)) => p,
        Ok(None) => return,
        Err(e) => { warn!(error = %e, "Erreur find_panel_by_message"); return; }
    };

    // Liste les jeux de la categorie du panel
    let games = match api
        .list_games_by_category(&guild_id_str, panel.category.as_deref())
        .await
    {
        Ok(g) => g,
        Err(e) => { warn!(error = %e, "Erreur list_games_by_category"); return; }
    };

    // Trouve le jeu dont l'emoji matche
    let game = games.iter().find(|g| {
        g.emoji
            .as_deref()
            .map(|em| emoji::emoji_matches(em, &reaction.emoji))
            .unwrap_or(false)
    });
    let game = match game {
        Some(g) => g,
        None => return,
    };

    let res = if add {
        api.subscribe(&guild_id_str, &game.id, &user_id.to_string()).await
    } else {
        api.unsubscribe(&guild_id_str, &game.id, &user_id.to_string()).await
    };
    if let Err(e) = res {
        warn!(error = %e, add, game = %game.game_name, "Erreur (un)subscribe via reaction");
    }
}
