//! Module Tamagotchi (compagnon virtuel).
//!
//! Jeu independant (coins partages). Panel global -> salon prive par joueur
//! (comme la machine a sous). Dans le salon : choix d'espece a la naissance,
//! puis carte du compagnon avec boutons de soin (Nourrir/Jouer/Dormir/Caliner).
//! Le combat, l'entrainement, la boutique, la tenue et la visite viendront en
//! jalons ulterieurs.

pub const MODULE_BOT_NAME: &str = "tamagotchi-bot";

mod api_client;
mod card_render;
mod lifecycle_events;
mod panel;
mod refresh;
mod setup;

use serenity::all::{CommandInteraction, ComponentInteraction, Context, CreateCommand};

/// Spawn les taches de fond du module tamagotchi (appele au `ready`) :
/// rafraichissement horaire des cartes + consumer des transitions de vie.
pub fn spawn_background(ctx: Context) {
    refresh::spawn(ctx.clone());
    lifecycle_events::spawn(ctx);
}

use crate::shared::discord_helpers::{
    is_module_enabled_or_reply_command, is_module_enabled_or_reply_component,
};

pub fn register_commands() -> Vec<CreateCommand> {
    vec![setup::register()]
}

pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    if !is_module_enabled_or_reply_command(ctx, command, MODULE_BOT_NAME).await {
        return;
    }
    if command.data.name == "tama-setup" {
        setup::handle(ctx, command).await;
    }
}

pub fn handles_component(cid: &str) -> bool {
    cid == setup::PANEL_OPEN_ID
        || cid == panel::CLOSE_ID
        || cid == panel::HIST_ID
        || cid == panel::SHOP_OPEN_ID
        || cid == panel::VISIT_OPEN_ID
        || cid == panel::VISIT_SELECT_ID
        || cid == panel::COMBAT_OPEN_ID
        || cid == panel::COMBAT_SELECT_ID
        || cid.starts_with(panel::PICK_PREFIX)
        || cid.starts_with(panel::ACT_PREFIX)
        || cid.starts_with(panel::TRAIN_PREFIX)
        || cid.starts_with(panel::BUY_PREFIX)
}

pub async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    if !is_module_enabled_or_reply_component(ctx, component, MODULE_BOT_NAME).await {
        return;
    }
    let cid = component.data.custom_id.as_str();
    if cid == setup::PANEL_OPEN_ID {
        panel::handle_open(ctx, component).await;
    } else if cid.starts_with(panel::PICK_PREFIX) {
        panel::handle_pick(ctx, component).await;
    } else if cid.starts_with(panel::ACT_PREFIX) {
        panel::handle_action(ctx, component).await;
    } else if cid.starts_with(panel::TRAIN_PREFIX) {
        panel::handle_train(ctx, component).await;
    } else if cid == panel::SHOP_OPEN_ID {
        panel::handle_shop_open(ctx, component).await;
    } else if cid.starts_with(panel::BUY_PREFIX) {
        panel::handle_buy(ctx, component).await;
    } else if cid == panel::VISIT_OPEN_ID {
        panel::handle_visit_open(ctx, component).await;
    } else if cid == panel::VISIT_SELECT_ID {
        panel::handle_visit_select(ctx, component).await;
    } else if cid == panel::COMBAT_OPEN_ID {
        panel::handle_combat_open(ctx, component).await;
    } else if cid == panel::COMBAT_SELECT_ID {
        panel::handle_combat_select(ctx, component).await;
    } else if cid == panel::HIST_ID {
        panel::handle_history(ctx, component).await;
    } else if cid == panel::CLOSE_ID {
        panel::handle_close(ctx, component).await;
    }
}
