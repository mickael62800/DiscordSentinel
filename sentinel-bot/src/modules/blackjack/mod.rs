//! Module blackjack — tables multijoueur, partie solo, images de cartes.
//!
//! Migre depuis blackjack-bot standalone vers sentinel-bot unifie.

pub const MODULE_BOT_NAME: &str = "blackjack-bot";

pub mod afk_cleanup;
pub mod api_client;
mod buttons;
pub mod card_image;
pub mod channel_manager;
mod embeds;
pub mod game;
pub mod game_logic;
// `messages` (templates BJ_*) migre dans `coude_flavor_templates`
// (migration 174) — bot consomme via `api.random_flavor`.
pub mod setup;
pub mod table;

use std::sync::Arc;

use serenity::all::{CommandInteraction, ComponentInteraction, Context, CreateCommand};
use serenity::prelude::*;

use crate::shared::discord_helpers::{
    is_module_enabled_or_reply_command, is_module_enabled_or_reply_component,
};

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

const BET_PREFIX: &str = "bj_bet:";
const CLOSE_TABLE_ID: &str = "bj_close_table";
const INVITE_BUTTON_ID: &str = "bj_invite";
const JOIN_BUTTON_ID: &str = "bj_join_table";

// ── Slash commands ──

// ── Init TypeMapKeys ──

pub fn init_typemap(
    data: &mut serenity::prelude::TypeMap,
    api: &Arc<crate::shared::api_client::BaseApiClient>,
    grpc: &Arc<crate::shared::grpc_client::SentinelGrpcClient>,
) {
    data.insert::<GameApiKey>(ApiClient::new(Arc::clone(api), Arc::clone(grpc)));
    data.insert::<ChannelManagerKey>(Arc::new(ChannelManager::new()));
}

pub fn register_commands() -> Vec<CreateCommand> {
    vec![setup::register()]
}

pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    if !is_module_enabled_or_reply_command(ctx, command, MODULE_BOT_NAME).await {
        return;
    }
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
    if !is_module_enabled_or_reply_component(ctx, component, MODULE_BOT_NAME).await {
        return;
    }
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
    afk_cleanup::spawn(ctx.clone());
    spawn_redis_listener(ctx);
}

/// Listener Redis Stream : `blackjack_table_closed` depuis web -> edit
/// l'embed Discord (gris + retire les boutons + footer "ferme via web").
fn spawn_redis_listener(ctx: Context) {
    tokio::spawn(async move {
        let consumer = crate::shared::event_bus::default_consumer_name();
        crate::shared::event_bus::listen_stream_group(
            "blackjack-bot-sync".to_string(),
            consumer,
            move |payload| {
                let ctx = ctx.clone();
                async move {
                    table::handle_redis_event(&ctx, &payload).await;
                }
            },
        )
        .await;
    });
}
