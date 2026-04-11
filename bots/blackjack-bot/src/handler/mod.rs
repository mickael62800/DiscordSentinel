//! EventHandler du bot blackjack, éclaté par responsabilité.
//!
//! - `table` : cycle de vie d'une table (création, invitation, join, fermeture).
//! - `game` : sélection de mise, détection de fin de partie, boutons rejouer.
//! - `afk_cleanup` : background task qui ferme les tables inactives.

use std::sync::Arc;

use serenity::async_trait;
use serenity::model::application::Interaction;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use tracing::{error, info};

use sentinel_shared::heartbeat::{register_guilds, ApiClientKey};

use crate::channel_manager::ChannelManager;
use crate::commands;

mod afk_cleanup;
mod game;
mod table;

pub struct ChannelManagerKey;
impl TypeMapKey for ChannelManagerKey {
    type Value = Arc<ChannelManager>;
}

/// Custom IDs pour les boutons — partagés entre `table` et `game`.
pub(super) const BET_PREFIX: &str = "bj_bet:";
pub(super) const CLOSE_TABLE_ID: &str = "bj_close_table";
pub(super) const INVITE_BUTTON_ID: &str = "bj_invite";
pub(super) const JOIN_BUTTON_ID: &str = "bj_join_table";

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, "Blackjack bot connecte");

        register_guilds(&ctx, &ready).await;

        if let Err(e) = serenity::model::application::Command::set_global_commands(
            &ctx.http,
            commands::all(),
        )
        .await
        {
            error!(error = %e, "Erreur enregistrement commandes");
        } else {
            info!("Slash commands enregistrees : blackjack-setup");
        }

        // Background task : cleanup des tables AFK toutes les 60s
        afk_cleanup::spawn(ctx.clone());
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(command) => {
                if let Some(guild_id) = command.guild_id {
                    let data = ctx.data.read().await;
                    if let Some(api) = data.get::<ApiClientKey>() {
                        if !sentinel_shared::discord_helpers::is_bot_enabled(
                            api,
                            &guild_id.to_string(),
                        )
                        .await
                        {
                            return;
                        }
                    }
                }

                if command.data.name.as_str() == "blackjack-setup" { commands::setup::handle(&ctx, &command).await }
            }
            Interaction::Component(component) => {
                let custom_id = component.data.custom_id.clone();

                if custom_id == commands::setup::PANEL_BUTTON_ID {
                    table::handle_panel_click(&ctx, &component).await;
                } else if custom_id.starts_with(BET_PREFIX) {
                    game::handle_bet_select(&ctx, &component).await;
                } else if custom_id == CLOSE_TABLE_ID {
                    table::handle_close_table(&ctx, &component).await;
                } else if custom_id == INVITE_BUTTON_ID {
                    table::handle_invite(&ctx, &component).await;
                } else if custom_id == JOIN_BUTTON_ID {
                    table::handle_join(&ctx, &component).await;
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
                    commands::blackjack::handle_component(&ctx, &component).await;
                    game::check_game_over(&ctx, &component).await;
                }
            }
            _ => {}
        }
    }
}
