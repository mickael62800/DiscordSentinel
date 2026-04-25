//! Module slot (machine a sous / tirette).
//!
//! Panel persistant + 2 boutons : Tirer (spin paye, mise par defaut) et
//! Daily Bonus (1 spin gratuit / jour). API HTTP via api_client.rs.

pub const MODULE_BOT_NAME: &str = "slot-bot";

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
    if command.data.name == "slot-setup" {
        setup::handle(ctx, command).await;
    }
}

pub fn handles_component(cid: &str) -> bool {
    cid == setup::PANEL_SPIN_ID || cid == setup::PANEL_DAILY_ID
}

pub async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    if !is_module_enabled_or_reply_component(ctx, component, MODULE_BOT_NAME).await {
        return;
    }
    let cid = component.data.custom_id.as_str();
    if cid == setup::PANEL_SPIN_ID {
        buttons::handle_spin(ctx, component).await;
    } else if cid == setup::PANEL_DAILY_ID {
        buttons::handle_daily(ctx, component).await;
    }
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
