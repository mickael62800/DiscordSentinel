//! Module games — /game, /game-admin, panels de select-menu.
//!
//! Le ping des joueurs est fait nativement via un role Discord par jeu :
//! chaque jeu cree par `/game-admin create` genere un role Discord mentionnable
//! (`<@&role_id>`). S'abonner = recevoir le role, se desabonner = perdre le role.

pub const MODULE_BOT_NAME: &str = "game-bot";

pub mod api_client;
pub mod commands;
pub mod emoji;

use std::collections::HashSet;
use std::sync::Arc;

use serenity::all::{
    CommandInteraction, ComponentInteraction, ComponentInteractionDataKind, Context,
    CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage, RoleId,
};
use tracing::warn;

use sentinel_shared::discord_helpers::{
    is_module_enabled_or_reply_command, is_module_enabled_or_reply_component,
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

    // Extrait panel_id du custom_id : `game_panel_select_{panel_id}_{chunk_idx}`.
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

    // Chaque menu couvre un chunk des jeux (25 options max). On ne synchronise
    // que les jeux de ce chunk.
    const CHUNK_SIZE: usize = 25;
    let chunk_idx: usize = component
        .data
        .custom_id
        .rsplit_once('_')
        .and_then(|(_, n)| n.parse::<usize>().ok())
        .unwrap_or(0);

    let chunk_games: Vec<&api_client::Game> = games_in_category
        .chunks(CHUNK_SIZE)
        .nth(chunk_idx)
        .map(|c| c.iter().collect())
        .unwrap_or_default();

    if chunk_games.is_empty() {
        reply_ephemeral(ctx, component, "Ce panel est vide ou obsolete.").await;
        return;
    }

    let chunk_game_ids: HashSet<String> = chunk_games.iter().map(|g| g.id.clone()).collect();
    let selected_set: HashSet<String> = selected_values
        .into_iter()
        .filter(|id| chunk_game_ids.contains(id))
        .collect();

    // Recupere le membre pour lire/muter ses roles.
    let member = match guild_id.member(&ctx.http, user_id).await {
        Ok(m) => m,
        Err(e) => {
            warn!(error = %e, "Erreur fetch member depuis select");
            reply_ephemeral(ctx, component, "Erreur : impossible de lire ton profil.").await;
            return;
        }
    };
    let current_role_ids: HashSet<RoleId> = member.roles.iter().copied().collect();

    let mut added_names: Vec<String> = Vec::new();
    let mut removed_names: Vec<String> = Vec::new();
    let mut skipped_legacy = 0usize;

    for g in &chunk_games {
        let role_id = match g.role_id.as_deref().and_then(|s| s.parse::<u64>().ok()) {
            Some(id) => RoleId::new(id),
            None => {
                skipped_legacy += 1;
                warn!(game = %g.game_name, "Jeu sans role_id : skip (legacy)");
                continue;
            }
        };
        let wants = selected_set.contains(&g.id);
        let has = current_role_ids.contains(&role_id);

        if wants && !has {
            match member.add_role(&ctx.http, role_id).await {
                Ok(()) => added_names.push(g.game_name.clone()),
                Err(e) => warn!(error = %e, game = %g.game_name, "Erreur add_role"),
            }
        } else if !wants && has {
            match member.remove_role(&ctx.http, role_id).await {
                Ok(()) => removed_names.push(g.game_name.clone()),
                Err(e) => warn!(error = %e, game = %g.game_name, "Erreur remove_role"),
            }
        }
    }

    let response = build_sync_response(&added_names, &removed_names, skipped_legacy);
    reply_ephemeral(ctx, component, &response).await;
}

fn build_sync_response(added: &[String], removed: &[String], skipped_legacy: usize) -> String {
    if added.is_empty() && removed.is_empty() && skipped_legacy == 0 {
        return "Aucun changement.".to_string();
    }
    let mut lines = vec!["**Abonnements mis a jour :**".to_string()];
    if !added.is_empty() {
        let shown: Vec<&String> = added.iter().take(10).collect();
        let extra = added.len().saturating_sub(shown.len());
        let names = shown.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
        if extra > 0 {
            lines.push(format!("+ {} (+{} autres)", names, extra));
        } else {
            lines.push(format!("+ {}", names));
        }
    }
    if !removed.is_empty() {
        let shown: Vec<&String> = removed.iter().take(10).collect();
        let extra = removed.len().saturating_sub(shown.len());
        let names = shown.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
        if extra > 0 {
            lines.push(format!("- {} (+{} autres)", names, extra));
        } else {
            lines.push(format!("- {}", names));
        }
    }
    if skipped_legacy > 0 {
        lines.push(format!(
            "*{} jeu(x) ignore(s) : pas encore de role Discord associe (recree-les via `/game-admin create`).*",
            skipped_legacy
        ));
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
