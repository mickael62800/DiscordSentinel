//! Module Roue du Destin (wheel-bot).
//!
//! Rituel quotidien : 1 spin par jour par joueur, resultat broadcast
//! publiquement dans le salon courant. Animation suspense 4s.
//!
//! Pas de salon prive (contrairement a slot) — la roue est la SIGNATURE
//! du serveur, tout le monde voit chaque spin.

pub const MODULE_BOT_NAME: &str = "wheel-bot";

pub mod api_client;
mod buttons;
mod embeds;
pub mod setup;

use serenity::all::{CommandInteraction, ComponentInteraction, Context, CreateCommand};

use sentinel_shared::discord_helpers::{
    is_module_enabled_or_reply_command, is_module_enabled_or_reply_component,
};

pub fn register_commands() -> Vec<CreateCommand> {
    vec![setup::register()]
}

pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    if !is_module_enabled_or_reply_command(ctx, command, MODULE_BOT_NAME).await {
        return;
    }
    if command.data.name == "wheel-setup" {
        setup::handle(ctx, command).await;
    }
}

pub fn handles_component(cid: &str) -> bool {
    cid == setup::PANEL_SPIN_ID
}

pub async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    if !is_module_enabled_or_reply_component(ctx, component, MODULE_BOT_NAME).await {
        return;
    }
    let cid = component.data.custom_id.as_str();
    if cid == setup::PANEL_SPIN_ID {
        buttons::handle_spin(ctx, component).await;
    }
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
