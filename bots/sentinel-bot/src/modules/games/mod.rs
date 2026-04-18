//! Module games — /game, /game-admin, panels de select-menu, mentions #Jeu.

pub const MODULE_BOT_NAME: &str = "game-bot";

pub mod api_client;
pub mod commands;
pub mod detector;
pub mod emoji;

use std::collections::HashSet;
use std::sync::Arc;

use serenity::all::{
    CommandInteraction, ComponentInteraction, ComponentInteractionDataKind, Context,
    CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage, Message,
};
use tracing::warn;

use sentinel_shared::discord_helpers::{
    is_module_enabled, is_module_enabled_or_reply_command, is_module_enabled_or_reply_component,
};
use sentinel_shared::heartbeat::ApiClientKey;

use commands::PANEL_SELECT_PREFIX;

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

// ── Component interactions (select menus des panels) ──

pub fn handles_component(cid: &str) -> bool {
    cid.starts_with(PANEL_SELECT_PREFIX)
}

pub async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    let cid = component.data.custom_id.as_str();
    if !cid.starts_with(PANEL_SELECT_PREFIX) {
        return;
    }

    if !is_module_enabled_or_reply_component(ctx, component, MODULE_BOT_NAME).await {
        return;
    }

    handle_panel_select(ctx, component).await;
}

async fn handle_panel_select(ctx: &Context, component: &ComponentInteraction) {
    let guild_id = match component.guild_id {
        Some(g) => g,
        None => return,
    };
    let guild_id_str = guild_id.to_string();
    let user_id = component.user.id;
    let user_id_str = user_id.to_string();

    // Extrait panel_id du custom_id : `game_panel_select_{panel_id}_{chunk_idx}`.
    // Le panel_id est un UUID (qui contient des `-`), suivi d'un `_{chunk_idx}`.
    let suffix = match component.data.custom_id.strip_prefix(PANEL_SELECT_PREFIX) {
        Some(s) => s,
        None => return,
    };
    let panel_id = match suffix.rsplit_once('_') {
        Some((pid, _chunk)) => pid.to_string(),
        None => suffix.to_string(),
    };

    // Valeurs selectionnees (game_id) dans ce select menu.
    let selected_values: Vec<String> = match &component.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => values.clone(),
        _ => return,
    };

    let data = ctx.data.read().await;
    let base = match data.get::<ApiClientKey>() {
        Some(b) => Arc::clone(b),
        None => return,
    };
    drop(data);

    let api = api_client::GameApiClient::new(base);

    // Retrouve le panel pour connaitre sa categorie.
    let panels = match api.list_panels(&guild_id_str).await {
        Ok(p) => p,
        Err(e) => {
            warn!(error = %e, "Erreur list_panels depuis select");
            reply_ephemeral(ctx, component, "Erreur : impossible de retrouver le panel.").await;
            return;
        }
    };
    let panel = match panels.into_iter().find(|p| p.id == panel_id) {
        Some(p) => p,
        None => {
            reply_ephemeral(ctx, component, "Ce panel n'existe plus. Demande a un admin de le redeployer.").await;
            return;
        }
    };

    // Liste tous les jeux de la categorie du panel.
    let games_in_category = match api
        .list_games_by_category(&guild_id_str, panel.category.as_deref())
        .await
    {
        Ok(g) => g,
        Err(e) => {
            warn!(error = %e, "Erreur list_games_by_category depuis select");
            reply_ephemeral(ctx, component, "Erreur : impossible de lister les jeux.").await;
            return;
        }
    };

    // Sur un message multi-select, chaque menu ne couvre qu'un chunk des jeux.
    // On ne synchronise donc que les jeux presents dans les options du menu actuel
    // (connus via leurs ids : = l'intersection des game_ids possibles pour ce chunk).
    //
    // Approche simple : on considere "les jeux du panel" comme l'ensemble complet,
    // et on applique la diff entre `selected_values` et `current_subscriptions`
    // limites a ce sous-ensemble. Si plusieurs menus, chaque interaction ne touche
    // que les jeux de son propre chunk.
    //
    // Pour savoir quels game_ids appartiennent a ce chunk, on redecoupe la liste
    // exactement comme dans commands::build_panel_components : chunks de 25 dans
    // l'ordre renvoye par l'API.
    const CHUNK_SIZE: usize = 25;
    let chunk_idx: usize = component
        .data
        .custom_id
        .rsplit_once('_')
        .and_then(|(_, n)| n.parse::<usize>().ok())
        .unwrap_or(0);

    let chunk_game_ids: HashSet<String> = games_in_category
        .chunks(CHUNK_SIZE)
        .nth(chunk_idx)
        .map(|c| c.iter().map(|g| g.id.clone()).collect())
        .unwrap_or_default();

    if chunk_game_ids.is_empty() {
        reply_ephemeral(ctx, component, "Ce panel est vide ou obsolete.").await;
        return;
    }

    // Abonnements actuels du user dans la guild, filtres sur les jeux du chunk.
    let user_games = match api.get_user_games(&guild_id_str, &user_id_str).await {
        Ok(g) => g,
        Err(e) => {
            warn!(error = %e, "Erreur get_user_games depuis select");
            reply_ephemeral(ctx, component, "Erreur : impossible de lire tes abonnements.").await;
            return;
        }
    };
    let current_in_chunk: HashSet<String> = user_games
        .iter()
        .map(|g| g.id.clone())
        .filter(|id| chunk_game_ids.contains(id))
        .collect();

    let selected_set: HashSet<String> = selected_values
        .into_iter()
        .filter(|id| chunk_game_ids.contains(id))
        .collect();

    let to_add: Vec<String> = selected_set.difference(&current_in_chunk).cloned().collect();
    let to_remove: Vec<String> = current_in_chunk.difference(&selected_set).cloned().collect();

    // Map id → name pour le rendu ephemeral.
    let id_to_name: std::collections::HashMap<String, String> = games_in_category
        .iter()
        .map(|g| (g.id.clone(), g.game_name.clone()))
        .collect();

    let mut added_names: Vec<String> = Vec::new();
    let mut removed_names: Vec<String> = Vec::new();

    for id in &to_add {
        match api.subscribe(&guild_id_str, id, &user_id_str).await {
            Ok(()) => {
                if let Some(name) = id_to_name.get(id) {
                    added_names.push(name.clone());
                }
            }
            Err(e) => warn!(error = %e, game_id = %id, "Erreur subscribe via select"),
        }
    }
    for id in &to_remove {
        match api.unsubscribe(&guild_id_str, id, &user_id_str).await {
            Ok(()) => {
                if let Some(name) = id_to_name.get(id) {
                    removed_names.push(name.clone());
                }
            }
            Err(e) => warn!(error = %e, game_id = %id, "Erreur unsubscribe via select"),
        }
    }

    let response = build_sync_response(&added_names, &removed_names);
    reply_ephemeral(ctx, component, &response).await;
}

fn build_sync_response(added: &[String], removed: &[String]) -> String {
    if added.is_empty() && removed.is_empty() {
        return "Aucun changement.".to_string();
    }
    let mut lines = vec!["**Abonnements mis a jour :**".to_string()];
    if !added.is_empty() {
        let shown: Vec<&String> = added.iter().take(10).collect();
        let extra = added.len().saturating_sub(shown.len());
        let names = shown.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
        if extra > 0 {
            lines.push(format!("➕ {} (+{} autres)", names, extra));
        } else {
            lines.push(format!("➕ {}", names));
        }
    }
    if !removed.is_empty() {
        let shown: Vec<&String> = removed.iter().take(10).collect();
        let extra = removed.len().saturating_sub(shown.len());
        let names = shown.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
        if extra > 0 {
            lines.push(format!("➖ {} (+{} autres)", names, extra));
        } else {
            lines.push(format!("➖ {}", names));
        }
    }
    lines.join("\n")
}

async fn reply_ephemeral(ctx: &Context, component: &ComponentInteraction, content: &str) {
    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(content)
            .ephemeral(true),
    );
    if let Err(e) = component.create_response(&ctx.http, response).await {
        warn!(error = %e, "Erreur reponse ephemeral games panel select");
    }
}
