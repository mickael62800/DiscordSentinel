//! Routes bot-facing (wallet, blackjack, rules, infractions) sans les
//! endpoints lourds d'inference deplacés dans `heavy`.

use axum::routing::{delete, get, post};
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

/// Routes bot standard (sans les endpoints lourds d'inference deplacés dans heavy_routes).
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/rules/{guild_id}", get(handlers::rules::get_rules))
        .route("/rules", post(handlers::rules::create_rule))
        .route("/rules/{guild_id}/{rule_id}", delete(handlers::rules::delete_rule))
        .route("/infractions/{guild_id}", get(handlers::infractions::list_infractions))
        .route("/infractions/delete/{id}", delete(handlers::infractions::delete_infraction))
        // Wallet (shared coin system)
        .route("/api/wallet/{guild_id}/{user_id}", get(handlers::wallet::get_wallet))
        .route("/api/wallet/{guild_id}/{user_id}/credit", post(handlers::wallet::credit))
        .route("/api/wallet/{guild_id}/{user_id}/debit", post(handlers::wallet::debit))
        .route("/api/wallet/transfer", post(handlers::wallet::transfer))
        .route("/api/wallet/{guild_id}/leaderboard", get(handlers::wallet::leaderboard))
        .route("/api/wallet/{guild_id}/{user_id}/transactions", get(handlers::wallet::transactions))
        // Phase 8 — administration wallets (pluriel pour eviter collision
        // avec les routes dynamiques /api/wallet/{guild}/{user_id}).
        .route("/api/wallets/{guild_id}", get(handlers::wallet::list_wallets))
        .route("/api/wallets/{guild_id}/reset-all", post(handlers::wallet::reset_all_wallets))
        .route("/api/wallets/{guild_id}/{user_id}/reset", post(handlers::wallet::reset_wallet))
        // Blackjack
        .route("/api/blackjack/start", post(handlers::blackjack::start_game))
        .route("/api/blackjack/{game_id}/hit", post(handlers::blackjack::hit))
        .route("/api/blackjack/{game_id}/stand", post(handlers::blackjack::stand))
        .route("/api/blackjack/{game_id}/double", post(handlers::blackjack::double_down))
        .route("/api/blackjack/{guild_id}/{user_id}/active", get(handlers::blackjack::get_active))
        // Phase 8 — administration blackjack (prefixe "admin" pour eviter
        // collision avec les routes dynamiques /api/blackjack/{game_id}/*).
        .route("/api/blackjack/admin/{guild_id}/games", get(handlers::blackjack::list_games))
        .route("/api/blackjack/admin/games/{game_id}", delete(handlers::blackjack::cancel_game))
        .route("/api/blackjack/admin/{guild_id}/purge", delete(handlers::blackjack::purge_all))
        // Blackjack tables (multijoueur)
        .route("/api/blackjack/tables", post(handlers::blackjack::create_table))
        .route("/api/blackjack/tables/{table_id}/join", post(handlers::blackjack::join_table))
        .route("/api/blackjack/tables/{table_id}/players", get(handlers::blackjack::list_table_players))
        .route("/api/blackjack/tables/{table_id}/games", get(handlers::blackjack::list_table_games))
        .route("/api/blackjack/tables/{table_id}", delete(handlers::blackjack::close_table))
        .route("/api/blackjack/tables/by-channel/{channel_id}", get(handlers::blackjack::get_table_by_channel))
        // Slot machine (migration 157)
        .route("/api/slot/{guild_id}/spin", post(handlers::slot::spin))
        .route("/api/slot/{guild_id}/daily", post(handlers::slot::daily))
        .route("/api/slot/{guild_id}/jackpot", get(handlers::slot::get_jackpot))
        .route("/api/slot/{guild_id}/recent", get(handlers::slot::recent_spins))
        .route("/api/slot/{guild_id}/leaderboard", get(handlers::slot::leaderboard))
}
