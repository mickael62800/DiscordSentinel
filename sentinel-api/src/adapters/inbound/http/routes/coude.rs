//! Routes Coup de Coude (jeu de combat).

use axum::routing::delete;
use axum::routing::get;
use axum::routing::patch;
use axum::routing::post;
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn coude_inner() -> Router<AppState> {
    Router::new()
        // Existing
        .route("/{guild_id}/combats", get(handlers::coude::combats::list_combats))
        .route("/{guild_id}/players", get(handlers::coude::players::list_players))
        .route("/{guild_id}/purge", delete(handlers::coude::combats::purge_all))
        .route("/combats/{combat_id}", delete(handlers::coude::combats::cancel_combat))
        .route("/players/{guild_id}/{user_id}/coins", patch(handlers::coude::players::adjust_coins))
        // Player CRUD
        .route("/{guild_id}/players/get-or-create", post(handlers::coude::players::get_or_create_player))
        .route("/{guild_id}/players/{user_id}", get(handlers::coude::players::get_player))
        .route("/{guild_id}/players/{user_id}/class", patch(handlers::coude::players::update_player_class))
        .route("/{guild_id}/players/{user_id}/xp", post(handlers::coude::players::add_xp))
        .route("/{guild_id}/players/{user_id}/spend-stat", post(handlers::coude::players::spend_stat_point))
        .route("/{guild_id}/players/{user_id}/reset-stats", post(handlers::coude::players::reset_stats))
        // Seasons
        .route("/{guild_id}/season/current", get(handlers::coude::social::get_current_season))
        // Stats recording
        .route("/{guild_id}/players/{user_id}/record-win", post(handlers::coude::players::record_win))
        .route("/{guild_id}/players/{user_id}/record-loss", post(handlers::coude::players::record_loss))
        .route("/{guild_id}/players/{user_id}/record-draw", post(handlers::coude::players::record_draw))
        .route("/{guild_id}/players/{user_id}/increment-cowardice", post(handlers::coude::players::increment_cowardice))
        .route("/{guild_id}/players/{user_id}/increment-chaos", post(handlers::coude::players::increment_chaos))
        .route("/{guild_id}/players/{user_id}/coins-earned", post(handlers::coude::players::record_coins_earned))
        .route("/{guild_id}/players/{user_id}/coins-lost", post(handlers::coude::players::record_coins_lost))
        // Casino
        .route("/{guild_id}/players/{user_id}/casino-win", post(handlers::coude::economy::record_casino_win))
        .route("/{guild_id}/players/{user_id}/casino-loss", post(handlers::coude::economy::record_casino_loss))
        .route("/{guild_id}/players/{user_id}/casino-faillite", post(handlers::coude::economy::record_casino_faillite))
        .route("/{guild_id}/players/{user_id}/casino-today", get(handlers::coude::economy::count_casino_today))
        .route("/{guild_id}/players/{user_id}/casino-gains-today", get(handlers::coude::economy::sum_casino_gains_today))
        .route("/{guild_id}/players/{user_id}/steal-today", get(handlers::coude::economy::count_steal_today))
        // Combat lifecycle
        .route("/{guild_id}/combats/create", post(handlers::coude::combats::create_combat))
        .route("/combats/{combat_id}/detail", get(handlers::coude::combats::get_combat))
        .route("/{guild_id}/combats/pending/attacker/{user_id}", get(handlers::coude::combats::get_pending_for_attacker))
        .route("/{guild_id}/combats/pending/defender/{user_id}", get(handlers::coude::combats::get_pending_for_defender))
        .route("/combats/{combat_id}/resolve", post(handlers::coude::combats::resolve_combat))
        .route("/combats/{combat_id}/betting", post(handlers::coude::combats::set_combat_betting))
        .route("/combats/{combat_id}/expire", post(handlers::coude::combats::expire_combat))
        .route("/combats/{combat_id}/defender-special", post(handlers::coude::combats::set_defender_special))
        .route("/combats/expired", get(handlers::coude::combats::get_expired_combats))
        // Bets
        .route("/{guild_id}/bets", post(handlers::coude::bets::place_bet))
        .route("/combats/{combat_id}/bets", get(handlers::coude::bets::get_combat_bets))
        .route("/{guild_id}/combats/betting/{user_id}", get(handlers::coude::bets::get_betting_combat))
        .route("/combats/{combat_id}/resolve-bets", post(handlers::coude::bets::resolve_bets))
        .route("/combats/{combat_id}/refund-bets", post(handlers::coude::bets::refund_bets))
        // Cooldowns
        .route("/{guild_id}/cooldown/{user_id}/{action}", get(handlers::coude::social::check_cooldown))
        .route("/{guild_id}/cooldown/{user_id}/{action}", post(handlers::coude::social::set_cooldown))
        // Economy
        .route("/{guild_id}/transfer", post(handlers::coude::economy::transfer_coins))
        .route("/{guild_id}/steal", post(handlers::coude::economy::record_steal))
        .route("/{guild_id}/steal-fail-penalty", post(handlers::coude::economy::steal_fail_penalty))
        // Primes
        .route("/{guild_id}/primes", post(handlers::coude::inventory::create_prime))
        .route("/{guild_id}/primes/{target_id}/active", get(handlers::coude::inventory::get_active_primes))
        .route("/{guild_id}/primes/claim", post(handlers::coude::inventory::claim_primes))
        // Insurance
        .route("/{guild_id}/friendly-duels", post(handlers::coude::friendly_duel::resolve_friendly_duel))
        .route("/{guild_id}/insurance/buy", post(handlers::coude::inventory::buy_insurance))
        // Phase 2 #3 audit : RNG scam migre cote API.
        .route(
            "/{guild_id}/insurance/buy-with-roll",
            post(handlers::coude::inventory::buy_insurance_with_roll),
        )
        .route("/{guild_id}/insurance/{user_id}", get(handlers::coude::inventory::get_active_insurance))
        .route("/insurance/{insurance_id}/expire", post(handlers::coude::inventory::expire_insurance))
        // Leaderboard
        .route("/{guild_id}/leaderboard/{category}", get(handlers::coude::social::leaderboard))
        // Utility
        .route("/guilds", get(handlers::coude::players::get_all_guild_ids))
        .route("/{guild_id}/players/random", get(handlers::coude::players::get_random_players))
        .route("/{guild_id}/daily-chaos", post(handlers::coude::social::log_daily_chaos))
        .route("/{guild_id}/events", get(handlers::coude::social::get_active_events))
        // Inventory
        .route("/{guild_id}/inventory/{user_id}", get(handlers::coude::inventory::get_inventory))
        .route("/{guild_id}/inventory/{user_id}/add", post(handlers::coude::inventory::add_item))
        .route("/{guild_id}/inventory/{user_id}/use", post(handlers::coude::inventory::use_item))
        .route("/{guild_id}/inventory/{user_id}/has/{item_key}", get(handlers::coude::inventory::has_item))
        // HP
        .route("/{guild_id}/players/{user_id}/hp", post(handlers::coude::players::update_hp))
        .route("/{guild_id}/players/{user_id}/repos", post(handlers::coude::players::repos))
        // Phase 9 Part E : config railleries (admin web UI)
        .route(
            "/{guild_id}/config/taunts",
            get(handlers::coude::taunts::get_taunts_config).put(handlers::coude::taunts::update_taunts_config),
        )
        .route(
            "/{guild_id}/config/taunts/opt-outs/{user_id}",
            delete(handlers::coude::taunts::remove_taunts_opt_out),
        )
        // Migration 139 : hooks taunts blackjack + eco (appeles par les bots).
        .route(
            "/{guild_id}/taunts/bj/natural/{user_id}",
            post(handlers::coude::taunts::track_bj_natural),
        )
        .route(
            "/{guild_id}/taunts/bj/won/{user_id}",
            post(handlers::coude::taunts::track_bj_won),
        )
        .route(
            "/{guild_id}/taunts/bj/bust/{user_id}",
            post(handlers::coude::taunts::track_bj_bust),
        )
        .route(
            "/{guild_id}/taunts/eco/bankruptcy/{user_id}",
            post(handlers::coude::taunts::track_bankruptcy),
        )
        .route(
            "/{guild_id}/taunts/eco/jackpot/{user_id}",
            post(handlers::coude::taunts::track_jackpot),
        )
        .route(
            "/{guild_id}/taunts/eco/donor/{user_id}",
            post(handlers::coude::taunts::track_generous_donor),
        )
        // Migration 139 : tournoi hebdo
        .route(
            "/{guild_id}/tournaments/current",
            get(handlers::coude::tournaments::get_current_tournament),
        )
        .route(
            "/{guild_id}/tournaments/history",
            get(handlers::coude::tournaments::get_tournament_history),
        )
        // Migration 159 : maledictions (cf. COUPE_AMELIORATIONS 5.1)
        .route(
            "/{guild_id}/curses",
            post(handlers::coude::curses::cast_curse),
        )
        .route(
            "/{guild_id}/curses/{target_id}",
            get(handlers::coude::curses::get_active_curse),
        )
        .route(
            "/{guild_id}/curses/{target_id}/lift",
            post(handlers::coude::curses::lift_curse),
        )
        // Migration 162 : Memorial des clodos (cf. COUPE_AMELIORATIONS 6.1)
        .route(
            "/{guild_id}/tout-ou-rien/record",
            post(handlers::coude::tout_ou_rien::record_tout_ou_rien),
        )
        .route(
            "/{guild_id}/tout-ou-rien/memorial",
            get(handlers::coude::tout_ou_rien::get_memorial),
        )
        .route(
            "/{guild_id}/tout-ou-rien/by-user/{user_id}",
            get(handlers::coude::tout_ou_rien::get_user_stats),
        )
        // Phase 2 #1 audit : RNG /tout-ou-rien migre cote API.
        .route(
            "/{guild_id}/tout-ou-rien/play",
            post(handlers::coude::tout_ou_rien::play_tout_ou_rien),
        )
        // Phase 2 #4 audit : RNG d20 + % de /voler migre cote API.
        .route(
            "/{guild_id}/steal/roll",
            post(handlers::coude::steal_roll::roll_steal),
        )
        // Phase 5 — persistance des tentatives /voler (timer 60s deplace
        // dans sentinel-worker).
        .route("/steals", post(handlers::coude::steal_attempts::create_steal_attempt))
        .route(
            "/steals/{id}/defend",
            patch(handlers::coude::steal_attempts::mark_defended),
        )
        .route(
            "/steals/{id}/resolved",
            patch(handlers::coude::steal_attempts::mark_resolved),
        )
        // Phase 3 #9 audit : catalogue de templates flavor.
        .route(
            "/flavor/{key}/random",
            get(handlers::coude::flavor::get_random_flavor),
        )
        // Phase 3 finalisation : RNG fake_amount /prank cote API.
        .route(
            "/{guild_id}/prank/braquage/roll",
            post(handlers::coude::prank::roll_prank_braquage_amount),
        )
        // Migration 165 : refusals / dette d honneur (cf. roadmap 5.3)
        .route(
            "/{guild_id}/refusals/{requester_id}/{refuser_id}/increment",
            post(handlers::coude::refusal::increment_refusal),
        )
        .route(
            "/{guild_id}/refusals/{requester_id}/{refuser_id}",
            get(handlers::coude::refusal::get_refusal),
        )
        .route(
            "/{guild_id}/refusals/{requester_id}/{refuser_id}/reset",
            post(handlers::coude::refusal::reset_refusal),
        )
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/coude", coude_inner())
}
