//! Routes Coup de Coude (jeu de combat).

use axum::routing::{delete, get, patch, post};
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn coude_inner() -> Router<AppState> {
    Router::new()
        // Existing
        .route("/{guild_id}/combats", get(handlers::coude::list_combats))
        .route("/{guild_id}/players", get(handlers::coude::list_players))
        .route("/{guild_id}/purge", delete(handlers::coude::purge_all))
        .route("/combats/{combat_id}", delete(handlers::coude::cancel_combat))
        .route("/players/{guild_id}/{user_id}/coins", patch(handlers::coude::adjust_coins))
        // Player CRUD
        .route("/{guild_id}/players/get-or-create", post(handlers::coude::get_or_create_player))
        .route("/{guild_id}/players/{user_id}", get(handlers::coude::get_player))
        .route("/{guild_id}/players/{user_id}/class", patch(handlers::coude::update_player_class))
        .route("/{guild_id}/players/{user_id}/xp", post(handlers::coude::add_xp))
        .route("/{guild_id}/players/{user_id}/spend-stat", post(handlers::coude::spend_stat_point))
        .route("/{guild_id}/players/{user_id}/reset-stats", post(handlers::coude::reset_stats))
        // Seasons
        .route("/{guild_id}/season/current", get(handlers::coude::get_current_season))
        // Stats recording
        .route("/{guild_id}/players/{user_id}/record-win", post(handlers::coude::record_win))
        .route("/{guild_id}/players/{user_id}/record-loss", post(handlers::coude::record_loss))
        .route("/{guild_id}/players/{user_id}/record-draw", post(handlers::coude::record_draw))
        .route("/{guild_id}/players/{user_id}/increment-cowardice", post(handlers::coude::increment_cowardice))
        .route("/{guild_id}/players/{user_id}/increment-chaos", post(handlers::coude::increment_chaos))
        .route("/{guild_id}/players/{user_id}/coins-earned", post(handlers::coude::record_coins_earned))
        .route("/{guild_id}/players/{user_id}/coins-lost", post(handlers::coude::record_coins_lost))
        // Casino
        .route("/{guild_id}/players/{user_id}/casino-win", post(handlers::coude::record_casino_win))
        .route("/{guild_id}/players/{user_id}/casino-loss", post(handlers::coude::record_casino_loss))
        .route("/{guild_id}/players/{user_id}/casino-faillite", post(handlers::coude::record_casino_faillite))
        .route("/{guild_id}/players/{user_id}/casino-today", get(handlers::coude::count_casino_today))
        .route("/{guild_id}/players/{user_id}/casino-gains-today", get(handlers::coude::sum_casino_gains_today))
        .route("/{guild_id}/players/{user_id}/steal-today", get(handlers::coude::count_steal_today))
        // Combat lifecycle
        .route("/{guild_id}/combats/create", post(handlers::coude::create_combat))
        .route("/combats/{combat_id}/detail", get(handlers::coude::get_combat))
        .route("/{guild_id}/combats/pending/attacker/{user_id}", get(handlers::coude::get_pending_for_attacker))
        .route("/{guild_id}/combats/pending/defender/{user_id}", get(handlers::coude::get_pending_for_defender))
        .route("/combats/{combat_id}/resolve", post(handlers::coude::resolve_combat))
        .route("/combats/{combat_id}/betting", post(handlers::coude::set_combat_betting))
        .route("/combats/{combat_id}/expire", post(handlers::coude::expire_combat))
        .route("/combats/{combat_id}/defender-special", post(handlers::coude::set_defender_special))
        .route("/combats/expired", get(handlers::coude::get_expired_combats))
        // Bets
        .route("/{guild_id}/bets", post(handlers::coude::place_bet))
        .route("/combats/{combat_id}/bets", get(handlers::coude::get_combat_bets))
        .route("/{guild_id}/combats/betting/{user_id}", get(handlers::coude::get_betting_combat))
        .route("/combats/{combat_id}/resolve-bets", post(handlers::coude::resolve_bets))
        .route("/combats/{combat_id}/refund-bets", post(handlers::coude::refund_bets))
        // Cooldowns
        .route("/{guild_id}/cooldown/{user_id}/{action}", get(handlers::coude::check_cooldown))
        .route("/{guild_id}/cooldown/{user_id}/{action}", post(handlers::coude::set_cooldown))
        // Economy
        .route("/{guild_id}/transfer", post(handlers::coude::transfer_coins))
        .route("/{guild_id}/steal", post(handlers::coude::record_steal))
        .route("/{guild_id}/steal-fail-penalty", post(handlers::coude::steal_fail_penalty))
        // Primes
        .route("/{guild_id}/primes", post(handlers::coude::create_prime))
        .route("/{guild_id}/primes/{target_id}/active", get(handlers::coude::get_active_primes))
        .route("/{guild_id}/primes/claim", post(handlers::coude::claim_primes))
        // Insurance
        .route("/{guild_id}/friendly-duels", post(handlers::coude::resolve_friendly_duel))
        .route("/{guild_id}/insurance/buy", post(handlers::coude::buy_insurance))
        .route("/{guild_id}/insurance/{user_id}", get(handlers::coude::get_active_insurance))
        .route("/insurance/{insurance_id}/expire", post(handlers::coude::expire_insurance))
        // Leaderboard
        .route("/{guild_id}/leaderboard/{category}", get(handlers::coude::leaderboard))
        // Utility
        .route("/guilds", get(handlers::coude::get_all_guild_ids))
        .route("/{guild_id}/players/random", get(handlers::coude::get_random_players))
        .route("/{guild_id}/daily-chaos", post(handlers::coude::log_daily_chaos))
        .route("/{guild_id}/events", get(handlers::coude::get_active_events))
        // Inventory
        .route("/{guild_id}/inventory/{user_id}", get(handlers::coude::get_inventory))
        .route("/{guild_id}/inventory/{user_id}/add", post(handlers::coude::add_item))
        .route("/{guild_id}/inventory/{user_id}/use", post(handlers::coude::use_item))
        .route("/{guild_id}/inventory/{user_id}/has/{item_key}", get(handlers::coude::has_item))
        // HP
        .route("/{guild_id}/players/{user_id}/hp", post(handlers::coude::update_hp))
        .route("/{guild_id}/players/{user_id}/repos", post(handlers::coude::repos))
        // Phase 9 Part E : config railleries (admin web UI)
        .route(
            "/{guild_id}/config/taunts",
            get(handlers::coude::get_taunts_config).put(handlers::coude::update_taunts_config),
        )
        .route(
            "/{guild_id}/config/taunts/opt-outs/{user_id}",
            delete(handlers::coude::remove_taunts_opt_out),
        )
        // Migration 139 : hooks taunts blackjack + eco (appeles par les bots).
        .route(
            "/{guild_id}/taunts/bj/natural/{user_id}",
            post(handlers::coude::track_bj_natural),
        )
        .route(
            "/{guild_id}/taunts/bj/won/{user_id}",
            post(handlers::coude::track_bj_won),
        )
        .route(
            "/{guild_id}/taunts/bj/bust/{user_id}",
            post(handlers::coude::track_bj_bust),
        )
        .route(
            "/{guild_id}/taunts/eco/bankruptcy/{user_id}",
            post(handlers::coude::track_bankruptcy),
        )
        .route(
            "/{guild_id}/taunts/eco/jackpot/{user_id}",
            post(handlers::coude::track_jackpot),
        )
        .route(
            "/{guild_id}/taunts/eco/donor/{user_id}",
            post(handlers::coude::track_generous_donor),
        )
        // Migration 139 : tournoi hebdo
        .route(
            "/{guild_id}/tournaments/current",
            get(handlers::coude::get_current_tournament),
        )
        .route(
            "/{guild_id}/tournaments/history",
            get(handlers::coude::get_tournament_history),
        )
        // Migration 159 : maledictions (cf. COUPE_AMELIORATIONS 5.1)
        .route(
            "/{guild_id}/curses",
            post(handlers::coude::cast_curse),
        )
        .route(
            "/{guild_id}/curses/{target_id}",
            get(handlers::coude::get_active_curse),
        )
        .route(
            "/{guild_id}/curses/{target_id}/lift",
            post(handlers::coude::lift_curse),
        )
        // Migration 161 : vendettas (cf. COUPE_AMELIORATIONS 5.3)
        .route(
            "/{guild_id}/vendettas",
            post(handlers::coude::declare_vendetta),
        )
        .route(
            "/{guild_id}/vendettas/by-challenger/{challenger_id}",
            get(handlers::coude::list_vendettas_by_challenger),
        )
        // Migration 162 : Memorial des clodos (cf. COUPE_AMELIORATIONS 6.1)
        .route(
            "/{guild_id}/tout-ou-rien/record",
            post(handlers::coude::record_tout_ou_rien),
        )
        .route(
            "/{guild_id}/tout-ou-rien/memorial",
            get(handlers::coude::get_memorial),
        )
        .route(
            "/{guild_id}/tout-ou-rien/by-user/{user_id}",
            get(handlers::coude::get_user_stats),
        )
        // Migration 164 : primes collectives (cf. COUPE_AMELIORATIONS 5.3)
        .route(
            "/{guild_id}/bounties/by-target/{target_id}",
            get(handlers::coude::get_bounty_by_target),
        )
        .route(
            "/{guild_id}/bounties/by-target/{target_id}/contribute",
            post(handlers::coude::contribute_to_target),
        )
        // Migration 165 : refusals / dette d honneur (cf. roadmap 5.3)
        .route(
            "/{guild_id}/refusals/{requester_id}/{refuser_id}/increment",
            post(handlers::coude::increment_refusal),
        )
        .route(
            "/{guild_id}/refusals/{requester_id}/{refuser_id}",
            get(handlers::coude::get_refusal),
        )
        .route(
            "/{guild_id}/refusals/{requester_id}/{refuser_id}/reset",
            post(handlers::coude::reset_refusal),
        )
        // Migration 166 : coalitions (cf. roadmap 5.3)
        .route(
            "/{guild_id}/coalitions/join",
            post(handlers::coude::join_coalition),
        )
        .route(
            "/{guild_id}/coalitions/by-target/{target_id}",
            get(handlers::coude::get_coalition_by_target),
        )
        // Migration 167 : ultimates par classe (cf. roadmap 3.1)
        .route(
            "/{guild_id}/ultimates/{user_id}/activate",
            post(handlers::coude::activate_ultimate),
        )
        .route(
            "/{guild_id}/ultimates/{user_id}",
            get(handlers::coude::get_ultimate_state),
        )
        // Migration 168 : prestige (cf. roadmap 3.3)
        .route(
            "/{guild_id}/players/{user_id}/prestige",
            post(handlers::coude::prestige_player),
        )
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/coude", coude_inner())
}
