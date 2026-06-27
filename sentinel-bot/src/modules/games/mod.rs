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

use crate::shared::discord_helpers::{
    is_module_enabled_or_reply_command, is_module_enabled_or_reply_component,
};
use crate::shared::heartbeat::ApiClientKey;

use commands::{PANEL_BUTTON_PREFIX, PANEL_SELECT_PREFIX};

pub fn register_commands() -> Vec<CreateCommand> {
    commands::all()
}

/// Spawn le consumer durable (Redis stream) : ecoute `games_panel_deploy`
/// (bouton "Deployer" du dashboard) et pose/rafraichit le panneau de jeux.
pub fn spawn(ctx: Context) {
    tokio::spawn(async move {
        let consumer = crate::shared::event_bus::default_consumer_name();
        crate::shared::event_bus::listen_stream_group(
            "sentinel-bot-games".to_string(),
            consumer,
            move |payload_json| {
                let ctx = ctx.clone();
                async move { handle_deploy_event(&ctx, &payload_json).await }
            },
        )
        .await;
    });
}

async fn handle_deploy_event(ctx: &Context, payload_json: &str) {
    use serenity::all::{ChannelId, GuildId};

    let envelope: serde_json::Value = match serde_json::from_str(payload_json) {
        Ok(v) => v,
        Err(_) => return,
    };
    if envelope.get("event").and_then(|v| v.as_str()) != Some("games_panel_deploy") {
        return;
    }
    let data = match envelope.get("data") {
        Some(d) => d,
        None => return,
    };
    let guild_id = data
        .get("guild_id")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u64>().ok());
    let channel_id = data
        .get("channel_id")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u64>().ok());
    let (Some(g), Some(c)) = (guild_id, channel_id) else { return };
    // category vide / absente => jeux sans categorie (None).
    let category = data
        .get("category")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    match commands::deploy_or_refresh_panel(ctx, GuildId::new(g), category.as_deref(), ChannelId::new(c)).await {
        Ok(status) => tracing::info!(guild = g, %status, "Panneau jeux deploye (web)"),
        Err(e) => tracing::warn!(guild = g, error = %e, "Echec deploiement panneau jeux (web)"),
    }
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
    cid.starts_with(PANEL_SELECT_PREFIX) || cid.starts_with(PANEL_BUTTON_PREFIX)
}

pub async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    let cid = component.data.custom_id.as_str();
    let is_select = cid.starts_with(PANEL_SELECT_PREFIX);
    let is_button = cid.starts_with(PANEL_BUTTON_PREFIX);
    if !is_select && !is_button {
        return;
    }

    if !is_module_enabled_or_reply_component(ctx, component, MODULE_BOT_NAME).await {
        return;
    }

    if is_button {
        handle_panel_button(ctx, component).await;
    } else {
        handle_panel_select(ctx, component).await;
    }
}

/// Clic sur un bouton-icone de jeu : toggle le role (abonnement) puis met a
/// jour le panneau (compteurs). Confirmation ephemere a l'utilisateur.
async fn handle_panel_button(ctx: &Context, component: &ComponentInteraction) {
    let guild_id = match component.guild_id {
        Some(g) => g,
        None => return,
    };
    let guild_id_str = guild_id.to_string();

    // custom_id : `game_panel_btn|{panel_id}|{game_id}`.
    let rest = match component.data.custom_id.strip_prefix(PANEL_BUTTON_PREFIX) {
        Some(s) => s,
        None => return,
    };
    let (panel_id, game_id) = match rest.split_once('|') {
        Some((p, g)) => (p.to_string(), g.to_string()),
        None => return,
    };

    let base = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(b) => Arc::clone(b),
            None => return,
        }
    };
    let api = api_client::GameApiClient::new(base);

    // Retrouve le panel (pour sa categorie) et les jeux de la categorie.
    let panel = match api.list_panels(&guild_id_str).await {
        Ok(panels) => panels.into_iter().find(|p| p.id == panel_id),
        Err(e) => {
            warn!(error = %e, "Erreur list_panels (bouton jeu)");
            None
        }
    };
    let Some(panel) = panel else {
        reply_ephemeral(ctx, component, "Ce panneau n'existe plus. Demande a un admin de le redeployer.").await;
        return;
    };
    let games = match api.list_games_by_category(&guild_id_str, panel.category.as_deref()).await {
        Ok(g) => g,
        Err(e) => {
            warn!(error = %e, "Erreur list_games_by_category (bouton jeu)");
            reply_ephemeral(ctx, component, "Erreur : impossible de lister les jeux.").await;
            return;
        }
    };

    let game = match games.iter().find(|g| g.id == game_id) {
        Some(g) => g,
        None => {
            reply_ephemeral(ctx, component, "Ce jeu n'existe plus.").await;
            return;
        }
    };
    let role_id = match game.role_id.as_deref().and_then(|s| s.parse::<u64>().ok()) {
        Some(id) => RoleId::new(id),
        None => {
            reply_ephemeral(ctx, component, "Ce jeu n'a pas de role associe. Demande a un admin de le recreer.").await;
            return;
        }
    };

    // Toggle du role sur le membre.
    let member = match guild_id.member(&ctx.http, component.user.id).await {
        Ok(m) => m,
        Err(e) => {
            warn!(error = %e, "Erreur fetch member (bouton jeu)");
            reply_ephemeral(ctx, component, "Erreur : impossible de lire ton profil.").await;
            return;
        }
    };
    let has = member.roles.contains(&role_id);
    let confirm = if has {
        match member.remove_role(&ctx.http, role_id).await {
            Ok(()) => format!("\u{274e} Tu ne suis plus **{}**.", game.game_name),
            Err(e) => {
                warn!(error = %e, "Erreur remove_role (bouton jeu)");
                "Erreur lors du desabonnement (hierarchie des roles ?).".to_string()
            }
        }
    } else {
        match member.add_role(&ctx.http, role_id).await {
            Ok(()) => format!("\u{2705} Tu suis maintenant **{}** ! Tu seras notifie.", game.game_name),
            Err(e) => {
                warn!(error = %e, "Erreur add_role (bouton jeu)");
                "Erreur lors de l'abonnement (hierarchie des roles ?).".to_string()
            }
        }
    };

    reply_ephemeral(ctx, component, &confirm).await;

    // Re-render du panneau (compteurs a jour). Edition directe du message.
    let games_slice: Vec<&api_client::Game> = games.iter().take(commands::MAX_BUTTONS_PER_PANEL).collect();
    let embed = commands::build_panel_embed(panel.category.as_deref(), &games_slice);
    let components = commands::build_panel_button_components(ctx, guild_id, &panel.id, &games_slice);
    let mut msg = component.message.clone();
    if let Err(e) = msg
        .edit(
            &ctx.http,
            serenity::all::EditMessage::new().embed(embed).components(components),
        )
        .await
    {
        warn!(error = %e, "Erreur re-render panneau jeux apres toggle");
    }
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

    // On track aussi l'etat final attendu pour pouvoir afficher la liste
    // complete des jeux actifs apres l'operation (sans re-fetch member).
    let mut active_role_ids: HashSet<RoleId> = current_role_ids.clone();

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
                Ok(()) => {
                    added_names.push(g.game_name.clone());
                    active_role_ids.insert(role_id);
                }
                Err(e) => warn!(error = %e, game = %g.game_name, "Erreur add_role"),
            }
        } else if !wants && has {
            match member.remove_role(&ctx.http, role_id).await {
                Ok(()) => {
                    removed_names.push(g.game_name.clone());
                    active_role_ids.remove(&role_id);
                }
                Err(e) => warn!(error = %e, game = %g.game_name, "Erreur remove_role"),
            }
        }
    }

    // Calcule la liste complete des jeux actuellement actifs pour cet user
    // (toutes categories confondues, pas juste le chunk courant). Permet a
    // l'user de voir son etat a chaque interaction puisque Discord ne peut
    // pas pre-cocher les options du panel public.
    let all_games = api.list_games_by_category(&guild_id_str, None).await.unwrap_or_default();
    let active_games: Vec<String> = all_games
        .iter()
        .filter_map(|g| {
            let rid = g.role_id.as_deref().and_then(|s| s.parse::<u64>().ok())?;
            if active_role_ids.contains(&RoleId::new(rid)) {
                Some(g.game_name.clone())
            } else {
                None
            }
        })
        .collect();

    let response = build_sync_response(&added_names, &removed_names, skipped_legacy, &active_games);
    reply_ephemeral(ctx, component, &response).await;
}

fn build_sync_response(
    added: &[String],
    removed: &[String],
    skipped_legacy: usize,
    active_games: &[String],
) -> String {
    let mut lines = Vec::new();

    if !added.is_empty() || !removed.is_empty() {
        lines.push("**Abonnements mis a jour :**".to_string());
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
    } else if skipped_legacy == 0 {
        lines.push("Aucun changement.".to_string());
    }

    if skipped_legacy > 0 {
        lines.push(format!(
            "*{} jeu(x) ignore(s) : pas encore de role Discord associe (recree-les via `/game-admin create`).*",
            skipped_legacy
        ));
    }

    // Liste complete des jeux actuellement suivis pour que l'user voie son
    // etat (le panel Discord ne peut pas pre-cocher selon l'utilisateur).
    if active_games.is_empty() {
        lines.push("\n**Tu ne suis aucun jeu actuellement.**".to_string());
    } else {
        let shown: Vec<&String> = active_games.iter().take(20).collect();
        let extra = active_games.len().saturating_sub(shown.len());
        let names = shown.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
        let suffix = if extra > 0 { format!(" (+{} autres)", extra) } else { String::new() };
        lines.push(format!(
            "\n**Tu suis actuellement ({}) :** {}{}",
            active_games.len(),
            names,
            suffix
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
