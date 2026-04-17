//! Module blackjack — tables multijoueur, partie solo, images de cartes.
//!
//! Migre depuis blackjack-bot standalone vers sentinel-bot unifie.

pub mod afk_cleanup;
pub mod api_client;
mod buttons;
pub mod card_image;
pub mod channel_manager;
mod embeds;
pub mod game;
pub mod game_logic;
mod messages;
pub mod setup;
pub mod table;

use std::sync::Arc;

use serenity::all::{CommandInteraction, ComponentInteraction, Context, CreateCommand};
use serenity::prelude::*;

use api_client::ApiClient;
use channel_manager::ChannelManager;

// ── TypeMapKeys ──

pub struct GameApiKey;
impl TypeMapKey for GameApiKey {
    type Value = ApiClient;
}

pub struct ChannelManagerKey;
impl TypeMapKey for ChannelManagerKey {
    type Value = Arc<ChannelManager>;
}

// ── Constants (custom IDs pour les boutons) ──

pub(self) const BET_PREFIX: &str = "bj_bet:";
pub(self) const CLOSE_TABLE_ID: &str = "bj_close_table";
pub(self) const INVITE_BUTTON_ID: &str = "bj_invite";
pub(self) const JOIN_BUTTON_ID: &str = "bj_join_table";

// ── Slash commands ──

pub fn register_commands() -> Vec<CreateCommand> {
    vec![setup::register()]
}

pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    if command.data.name == "blackjack-setup" {
        setup::handle(ctx, command).await;
    }
}

// ── Component interactions ──

/// Retourne true si ce custom_id est gere par le module blackjack.
pub fn handles_component(cid: &str) -> bool {
    cid == setup::PANEL_BUTTON_ID
        || cid.starts_with(BET_PREFIX)
        || cid == CLOSE_TABLE_ID
        || cid == INVITE_BUTTON_ID
        || cid == JOIN_BUTTON_ID
        || cid.starts_with("bj_hit:")
        || cid.starts_with("bj_stand:")
        || cid.starts_with("bj_double:")
}

pub async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    let custom_id = component.data.custom_id.as_str();

    if custom_id == setup::PANEL_BUTTON_ID {
        table::handle_panel_click(ctx, component).await;
    } else if custom_id.starts_with(BET_PREFIX) {
        game::handle_bet_select(ctx, component).await;
    } else if custom_id == CLOSE_TABLE_ID {
        table::handle_close_table(ctx, component).await;
    } else if custom_id == INVITE_BUTTON_ID {
        table::handle_invite(ctx, component).await;
    } else if custom_id == JOIN_BUTTON_ID {
        table::handle_join(ctx, component).await;
    } else if custom_id.starts_with("bj_hit:")
        || custom_id.starts_with("bj_stand:")
        || custom_id.starts_with("bj_double:")
    {
        // Touch activity
        {
            let data = ctx.data.read().await;
            if let Some(mgr) = data.get::<ChannelManagerKey>() {
                mgr.touch(component.user.id);
            }
        }
        game_logic::handle_component(ctx, component).await;
        game::check_game_over(ctx, component).await;
    }
}

/// Spawn les background tasks du module blackjack (appele au `ready`).
pub fn spawn_background(ctx: Context) {
    afk_cleanup::spawn(ctx);
}
