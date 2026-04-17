//! Module coude — Coup de Coude : mini-jeu social (combats, casino, primes,
//! vol, assurance, braquage + prison).
//!
//! Migre depuis coude-bot standalone vers sentinel-bot unifie.

pub mod api_client;
pub mod catalog;
pub mod channel_check;
pub mod commands;
pub mod guild_config;
pub mod interaction_helper;
pub mod prison_check;
pub mod taunts_dispatch;

use serenity::all::{CommandInteraction, ComponentInteraction, Context, CreateCommand, GuildId};
use serenity::prelude::*;

use sentinel_shared::heartbeat::ApiClientKey;

use api_client::ApiClient;
use guild_config::CoudeConfig;

// ── TypeMapKeys ──

/// Cle TypeMap pour le client API du jeu Coude.
pub struct GameApiKey;
impl TypeMapKey for GameApiKey {
    type Value = ApiClient;
}

/// Cle TypeMap pour stocker les guild IDs connus (pour le daily chaos / broadcasts).
pub struct GuildIdsKey;
impl TypeMapKey for GuildIdsKey {
    type Value = Vec<GuildId>;
}

// Re-exports utiles pour handler.rs
pub use catalog::CatalogCacheKey;
pub use prison_check::{check_and_reply_if_in_prison, check_component_in_prison};

// ── Helpers ──

/// Charge la config guild Coude depuis l'API (avec cache Redis cote API, TTL 15min).
pub async fn load_guild_config(ctx: &Context, guild_id: &str) -> CoudeConfig {
    let data = ctx.data.read().await;
    let api = data.get::<ApiClientKey>().expect("ApiClientKey non initialise");
    CoudeConfig::load(api, guild_id).await
}

// ── Slash commands ──

pub fn register_commands() -> Vec<CreateCommand> {
    commands::all()
}

pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    match command.data.name.as_str() {
        "coude" => commands::coude::handle(ctx, command).await,
        "profil" => commands::profil::handle(ctx, command).await,
        "shop" => commands::shop_cmd::handle(ctx, command).await,
        "prime" => commands::prime::handle(ctx, command).await,
        "leaderboard" => commands::leaderboard::handle(ctx, command).await,
        "pari" => commands::pari::handle(ctx, command).await,
        "voler" => commands::voler::handle(ctx, command).await,
        "assurance" => commands::assurance::handle(ctx, command).await,
        "train" => commands::train::handle(ctx, command).await,
        "classe" => commands::classe::handle(ctx, command).await,
        "donner" => commands::donner::handle(ctx, command).await,
        "hp" => commands::hp::handle(ctx, command).await,
        "repos" => commands::repos::handle(ctx, command).await,
        "potion" => commands::potion::handle(ctx, command).await,
        "saison" => commands::saison::handle(ctx, command).await,
        "reset-stats" => commands::reset_stats::handle(ctx, command).await,
        "resume" => commands::resume::handle(ctx, command).await,
        "cagnotte" => commands::cagnotte::handle(ctx, command).await,
        "protection" => commands::protection::handle(ctx, command).await,
        "boost-voleur" => commands::boost_voleur::handle(ctx, command).await,
        "no-taunts" => commands::no_taunts::handle(ctx, command).await,
        "taunts-channel" => commands::taunts_channel::handle(ctx, command).await,
        "braquage" => commands::braquage::handle(ctx, command).await,
        _ => {}
    }
}

/// Retourne `true` si la commande (par nom) est geree par le module coude.
pub fn handles_command(name: &str) -> bool {
    matches!(
        name,
        "coude"
            | "profil"
            | "shop"
            | "prime"
            | "leaderboard"
            | "pari"
            | "voler"
            | "assurance"
            | "train"
            | "classe"
            | "donner"
            | "hp"
            | "repos"
            | "potion"
            | "saison"
            | "reset-stats"
            | "resume"
            | "cagnotte"
            | "protection"
            | "boost-voleur"
            | "no-taunts"
            | "taunts-channel"
            | "braquage"
    )
}

// ── Component interactions ──

/// Retourne true si ce custom_id est gere par le module coude.
pub fn handles_component(cid: &str) -> bool {
    cid.starts_with(commands::coude::PRECONFIRM_OK_PREFIX)
        || cid.starts_with(commands::coude::PRECONFIRM_NO_PREFIX)
        || cid.starts_with(commands::accepter::ACCEPT_PREFIX)
        || cid.starts_with(commands::defend_item::DEFEND_SELECT_PREFIX)
        || cid.starts_with(commands::defend_item::DEFEND_PREFIX)
        || cid.starts_with(commands::refuser::REFUSE_PREFIX)
        || cid.starts_with(commands::annuler::CANCEL_PREFIX)
        || cid.starts_with(commands::classe::CLASS_SELECT_PREFIX)
        || cid.starts_with(commands::voler::STEAL_DEFEND_PREFIX)
}

pub async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    let custom_id = &component.data.custom_id;

    if custom_id.starts_with(commands::coude::PRECONFIRM_OK_PREFIX) {
        commands::coude::handle_preconfirm_ok(ctx, component).await;
    } else if custom_id.starts_with(commands::coude::PRECONFIRM_NO_PREFIX) {
        commands::coude::handle_preconfirm_no(ctx, component).await;
    } else if custom_id.starts_with(commands::accepter::ACCEPT_PREFIX) {
        commands::accepter::handle(ctx, component).await;
    } else if custom_id.starts_with(commands::defend_item::DEFEND_SELECT_PREFIX) {
        commands::defend_item::handle_defend_select(ctx, component).await;
    } else if custom_id.starts_with(commands::defend_item::DEFEND_PREFIX) {
        commands::defend_item::handle_defend_button(ctx, component).await;
    } else if custom_id.starts_with(commands::refuser::REFUSE_PREFIX) {
        commands::refuser::handle(ctx, component).await;
    } else if custom_id.starts_with(commands::annuler::CANCEL_PREFIX) {
        commands::annuler::handle(ctx, component).await;
    } else if custom_id.starts_with(commands::classe::CLASS_SELECT_PREFIX) {
        commands::classe::handle_select(ctx, component).await;
    } else if custom_id.starts_with(commands::voler::STEAL_DEFEND_PREFIX) {
        commands::voler::handle_defend(ctx, component).await;
    }
}

/// Bootstrap appele dans `ready` : stocke les guild IDs dans la TypeMap.
pub async fn on_ready(ctx: &Context, guild_ids: Vec<GuildId>) {
    let mut data = ctx.data.write().await;
    data.insert::<GuildIdsKey>(guild_ids);
}

/// Spawn les background tasks du module coude (actuellement: aucun — le
/// daily chaos a ete migre dans coude-worker + API).
pub fn spawn_background(_ctx: Context) {
    // Reserve pour de futures taches (ex : reapprovisionnement cache catalogue).
}
