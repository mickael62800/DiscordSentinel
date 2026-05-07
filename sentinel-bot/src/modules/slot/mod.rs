//! Module slot (machine a sous / tirette).
//!
//! Pattern aligne sur blackjack : panel persistant global avec un bouton
//! "Ouvrir ma machine" -> creation d un salon Discord prive par utilisateur,
//! ou se deroulent les spins avec animation suspense (3 frames de 2s),
//! resultat dans un message classique (non-ephemere) puis re-post des boutons
//! d action en bas.

pub const MODULE_BOT_NAME: &str = "slot-bot";

pub mod animation;
pub mod api_client;
mod buttons;
pub mod channel_manager;
mod embeds;
pub mod setup;

use std::sync::Arc;

use serenity::all::{CommandInteraction, ComponentInteraction, Context, CreateCommand};
use serenity::prelude::*;

use crate::shared::discord_helpers::{
    is_module_enabled_or_reply_command, is_module_enabled_or_reply_component,
};

pub use channel_manager::SlotChannelManager;

/// TypeMapKey pour stocker le SlotChannelManager dans le ctx.data partage.
pub struct SlotChannelManagerKey;
impl TypeMapKey for SlotChannelManagerKey {
    type Value = Arc<SlotChannelManager>;
}

/// Initialise les TypeMapKeys du module dans le TypeMap partage.
/// Appele au boot depuis `main.rs` comme les autres modules.
pub fn init_typemap(data: &mut TypeMap) {
    data.insert::<SlotChannelManagerKey>(Arc::new(SlotChannelManager::new()));
}

pub fn register_commands() -> Vec<CreateCommand> {
    vec![setup::register()]
}

pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    if !is_module_enabled_or_reply_command(ctx, command, MODULE_BOT_NAME).await {
        return;
    }
    if command.data.name == "slot-setup" {
        setup::handle(ctx, command).await;
    }
}

pub fn handles_component(cid: &str) -> bool {
    cid == setup::PANEL_OPEN_ID
        || cid == setup::CHANNEL_SPIN_ID
        || cid == setup::CHANNEL_DAILY_ID
        || cid == setup::CHANNEL_CLOSE_ID
}

pub async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    if !is_module_enabled_or_reply_component(ctx, component, MODULE_BOT_NAME).await {
        return;
    }
    let cid = component.data.custom_id.as_str();
    if cid == setup::PANEL_OPEN_ID {
        buttons::handle_open_machine(ctx, component).await;
    } else if cid == setup::CHANNEL_SPIN_ID {
        buttons::handle_spin_in_channel(ctx, component).await;
    } else if cid == setup::CHANNEL_DAILY_ID {
        buttons::handle_daily_in_channel(ctx, component).await;
    } else if cid == setup::CHANNEL_CLOSE_ID {
        buttons::handle_close_channel(ctx, component).await;
    }
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
