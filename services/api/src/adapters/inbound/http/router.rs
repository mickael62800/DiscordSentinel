use axum::http::{header, HeaderValue, Method};
use axum::middleware;
use axum::routing::{delete, get, patch, post};
use axum::Router;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;
use tracing::Span;

use super::handlers;
use super::middleware::api_logger::api_logger_middleware;
use super::middleware::auth::auth_middleware;
use super::middleware::rate_limit::{rate_limit_middleware, RateLimiter};
use super::state::AppState;

fn build_cors(allowed_origins: &str) -> CorsLayer {
    let allow_origin = if allowed_origins == "*" {
        AllowOrigin::any()
    } else if allowed_origins.is_empty() {
        // Default securise : uniquement les origines Tauri + localhost dev
        tracing::info!("ALLOWED_ORIGINS non configure — utilisation des origines par defaut (Tauri + localhost)");
        AllowOrigin::list([
            "https://tauri.localhost".parse::<HeaderValue>().unwrap(),
            "http://tauri.localhost".parse::<HeaderValue>().unwrap(),
            "http://localhost:1420".parse::<HeaderValue>().unwrap(),
            "http://localhost:3000".parse::<HeaderValue>().unwrap(),
        ])
    } else {
        let origins: Vec<HeaderValue> = allowed_origins
            .split(',')
            .filter_map(|o| o.trim().parse().ok())
            .collect();
        AllowOrigin::list(origins)
    };

    CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::HeaderName::from_static("x-request-id"),
        ])
        .max_age(std::time::Duration::from_secs(3600))
}

// ── Route groups ──

/// Routes bot standard (sans les endpoints lourds d'inference deplacés dans heavy_routes).
fn bot_routes_standard() -> Router<AppState> {
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
        // Blackjack
        .route("/api/blackjack/start", post(handlers::blackjack::start_game))
        .route("/api/blackjack/{game_id}/hit", post(handlers::blackjack::hit))
        .route("/api/blackjack/{game_id}/stand", post(handlers::blackjack::stand))
        .route("/api/blackjack/{game_id}/double", post(handlers::blackjack::double_down))
        .route("/api/blackjack/{guild_id}/{user_id}/active", get(handlers::blackjack::get_active))
}

fn ticket_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::tickets::list_tickets).post(handlers::tickets::create_ticket))
        .route("/{id}", get(handlers::tickets::get_ticket_detail))
        .route("/{id}/messages", post(handlers::tickets::reply_ticket))
        .route("/{id}/close", patch(handlers::tickets::close_ticket))
        .route("/{id}/assign", patch(handlers::tickets::assign_ticket))
        .route("/{id}/status", patch(handlers::tickets::update_status))
        .route("/{id}/channels", patch(handlers::tickets::update_ticket_channel))
}

fn security_routes() -> Router<AppState> {
    Router::new()
        .route("/events", post(handlers::security::report_event).get(handlers::security::list_events))
}

fn reminders_routes() -> Router<AppState> {
    Router::new()
        .route("/", post(handlers::reminders::create_reminder))
        .route("/pending", get(handlers::reminders::get_pending))
        .route("/{guild_id}", get(handlers::reminders::list_by_guild))
}

fn notes_routes() -> Router<AppState> {
    Router::new()
        .route("/", post(handlers::notes::add_note))
        .route("/{guild_id}/{user_id}", get(handlers::notes::get_notes))
        .route("/{id}", delete(handlers::notes::delete_note))
}

fn strikes_routes() -> Router<AppState> {
    Router::new()
        .route("/config/{guild_id}", get(handlers::strikes::get_config).put(handlers::strikes::save_config))
        .route("/{guild_id}/{user_id}", get(handlers::strikes::get_active_strikes).delete(handlers::strikes::reset_strikes))
        .route("/", post(handlers::strikes::add_strike))
}

fn moderation_routes() -> Router<AppState> {
    Router::new()
        .route("/actions", post(handlers::moderation::log_action))
        .route("/bans", get(handlers::moderation::list_bans))
        .route("/execute-ban", post(handlers::moderation::execute_ban))
        .route("/execute-unban", post(handlers::moderation::execute_unban))
        .route("/history/{guild_id}/{user_id}", get(handlers::moderation::get_history))
}

fn voice_channel_routes() -> Router<AppState> {
    Router::new()
        .route("/_all", get(handlers::voice_channels::list_all_channels))
        .route("/{guild_id}", get(handlers::voice_channels::list_channels))
        .route("/", post(handlers::voice_channels::create_channel))
        .route(
            "/by-channel/{channel_id}",
            get(handlers::voice_channels::get_channel_detail)
                .patch(handlers::voice_channels::update_channel)
                .delete(handlers::voice_channels::delete_channel),
        )
        .route("/by-channel/{channel_id}/close", patch(handlers::voice_channels::close_channel))
        .route("/by-channel/{channel_id}/transfer", patch(handlers::voice_channels::transfer_ownership))
        .route("/by-channel/{channel_id}/co-admins", post(handlers::voice_channels::add_co_admin))
        .route("/by-channel/{channel_id}/co-admins/{user_id}", delete(handlers::voice_channels::remove_co_admin))
        .route("/whitelist/{guild_id}/{owner_id}", get(handlers::voice_channels::get_whitelist))
        .route("/whitelist", post(handlers::voice_channels::add_to_whitelist))
        .route("/whitelist/{guild_id}/{owner_id}/{target_id}", delete(handlers::voice_channels::remove_from_whitelist))
        .route("/by-channel/{channel_id}/bans", post(handlers::voice_channels::ban_from_channel))
        .route(
            "/by-channel/{channel_id}/bans/{user_id}",
            delete(handlers::voice_channels::unban_from_channel)
                .get(handlers::voice_channels::check_ban),
        )
        // Invite Links
        .route(
            "/by-channel/{channel_id}/invites",
            get(handlers::voice_channels::list_invite_links)
                .post(handlers::voice_channels::create_invite_link),
        )
        .route("/by-channel/{channel_id}/invites/{link_id}", delete(handlers::voice_channels::revoke_invite_link))
        .route("/invites/{code}/use", post(handlers::voice_channels::use_invite_link))
        // Themes
        .route("/themes/{guild_id}", get(handlers::voice_channels::list_themes).post(handlers::voice_channels::create_theme))
        .route("/themes/{guild_id}/{theme_id}", patch(handlers::voice_channels::update_theme).delete(handlers::voice_channels::delete_theme))
}

fn conduct_routes() -> Router<AppState> {
    Router::new()
        .route("/config/{guild_id}", get(handlers::conduct::get_config))
        .route("/config", post(handlers::conduct::save_config))
        .route("/{guild_id}/{user_id}", get(handlers::conduct::get_points))
        .route("/{guild_id}/leaderboard", get(handlers::conduct::get_leaderboard))
        .route("/{guild_id}/{user_id}/log", get(handlers::conduct::get_points_log))
        .route("/{guild_id}/{user_id}/add", post(handlers::conduct::add_points))
}

fn level_routes() -> Router<AppState> {
    Router::new()
        .route("/config/{guild_id}", get(handlers::levels::get_config))
        .route("/config", post(handlers::levels::save_config))
        .route("/xp", post(handlers::levels::add_xp))
        .route("/{guild_id}/{user_id}", get(handlers::levels::get_user_level))
        .route("/{guild_id}/leaderboard", get(handlers::levels::get_leaderboard))
        .route("/rewards/{guild_id}", get(handlers::levels::get_rewards))
        .route("/rewards", post(handlers::levels::set_reward))
        .route("/rewards/{guild_id}/{level}", delete(handlers::levels::delete_reward))
}

fn role_panel_routes() -> Router<AppState> {
    Router::new()
        .route("/", post(handlers::role_panels::create_panel))
        .route("/{guild_id}", get(handlers::role_panels::list_panels))
        .route("/detail/{panel_id}", get(handlers::role_panels::get_panel).delete(handlers::role_panels::delete_panel))
        .route("/by-message/{message_id}", get(handlers::role_panels::get_panel_by_message))
        .route("/set-message", patch(handlers::role_panels::set_message_id))
}

fn auto_role_routes() -> Router<AppState> {
    Router::new()
        .route("/{guild_id}", get(handlers::role_panels::list_auto_roles))
        .route("/", post(handlers::role_panels::add_auto_role))
        .route("/{guild_id}/{role_id}", delete(handlers::role_panels::delete_auto_role))
}

fn coude_routes() -> Router<AppState> {
    Router::new()
        // Existing
        .route("/{guild_id}/combats", get(handlers::coude::list_combats))
        .route("/{guild_id}/players", get(handlers::coude::list_players))
        .route("/combats/{combat_id}", delete(handlers::coude::cancel_combat))
        .route("/players/{guild_id}/{user_id}/coins", patch(handlers::coude::adjust_coins))
        // Player CRUD
        .route("/{guild_id}/players/get-or-create", post(handlers::coude::get_or_create_player))
        .route("/{guild_id}/players/{user_id}", get(handlers::coude::get_player))
        .route("/{guild_id}/players/{user_id}/class", patch(handlers::coude::update_player_class))
        .route("/{guild_id}/players/{user_id}/xp", post(handlers::coude::add_xp))
        .route("/{guild_id}/players/{user_id}/spend-stat", post(handlers::coude::spend_stat_point))
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
        // Primes
        .route("/{guild_id}/primes", post(handlers::coude::create_prime))
        .route("/{guild_id}/primes/{target_id}/active", get(handlers::coude::get_active_primes))
        .route("/{guild_id}/primes/claim", post(handlers::coude::claim_primes))
        // Insurance
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
}

fn member_routes() -> Router<AppState> {
    Router::new()
        .route("/{guild_id}", get(handlers::guild_members::list_members_db))
        .route("/{guild_id}/{user_id}", get(handlers::guild_members::get_member).patch(handlers::guild_members::update_member).delete(handlers::guild_members::remove_member))
        .route("/{guild_id}/{user_id}/summary", get(handlers::guild_members::get_member_summary))
        .route("/sync", post(handlers::guild_members::sync_members))
        .route("/register", post(handlers::guild_members::register_member))
}

fn analytics_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::analytics::get_full_analytics))
        .route("/heatmap", get(handlers::analytics::get_heatmap))
        .route("/actions", get(handlers::analytics::get_action_distribution))
        .route("/top-infractors", get(handlers::analytics::get_top_infractors))
        .route("/moderation-trend", get(handlers::analytics::get_moderation_trend))
        .route("/peak-hours", get(handlers::analytics::get_peak_hours))
}

fn stats_routes() -> Router<AppState> {
    Router::new()
        .route("/messages", post(handlers::stats::record_messages))
        .route("/voice", post(handlers::stats::record_voice))
        .route("/{guild_id}/user/{user_id}", get(handlers::stats::get_user_stats))
        .route("/{guild_id}/overview", get(handlers::stats::get_guild_overview))
        .route("/{guild_id}/leaderboard", get(handlers::stats::get_leaderboard))
        .route("/{guild_id}/voice-stats", get(handlers::stats::get_guild_voice_stats))
}

fn dashboard_routes() -> Router<AppState> {
    Router::new()
        .route("/guilds", get(handlers::dashboard::list_guilds))
        .route("/guilds/register", post(handlers::dashboard::register_guild))
        .route("/logs", get(handlers::dashboard::get_logs).post(handlers::dashboard::create_log))
        .route("/logs/{category}", delete(handlers::dashboard::delete_logs_by_category))
        .route("/infractions", get(handlers::dashboard::get_all_infractions))
        .route("/infractions/{id}", delete(handlers::infractions::delete_infraction))
        .route("/rules", get(handlers::dashboard::get_all_rules))
        .route("/rules/{id}", patch(handlers::dashboard::toggle_rule))
        .route("/bots/heartbeat", post(handlers::dashboard::bot_heartbeat))
        .route("/bots/definitions", get(handlers::bot_config::get_definitions))
        .route("/bots/config/{guild_id}", get(handlers::bot_config::get_guild_config))
        .route("/bots/config/{guild_id}/{bot_name}", get(handlers::bot_config::get_bot_config))
        .route("/bots/config", post(handlers::bot_config::set_config).delete(handlers::bot_config::delete_config))
        .route("/ia-config/{guild_id}", get(handlers::ia_config::get_ia_config).put(handlers::ia_config::save_ia_config))
        .route("/purge/infractions", delete(handlers::purge::purge_infractions))
        .route("/purge/audit-logs", delete(handlers::purge::purge_audit_logs))
        .route("/purge/logs", delete(handlers::purge::purge_logs))
}

/// Construit le router sans rate limiter ni ConnectInfo — pour les tests d'integration.
pub fn build_for_test(state: AppState) -> Router {
    let protected = Router::new()
        // Endpoints lourds (sans rate limit en test)
        .route("/analyze", post(handlers::analyze::analyze))
        .route("/analyze/image", post(handlers::analyze_image::analyze_image))
        .nest("/api/analytics", analytics_routes())
        // Routes standard
        .merge(bot_routes_standard())
        .nest("/api/tickets", ticket_routes())
        .nest("/api/security", security_routes())
        .nest("/api/moderation", moderation_routes())
        .nest("/api/strikes", strikes_routes())
        .nest("/api/notes", notes_routes())
        .nest("/api/reminders", reminders_routes())
        .nest("/api/voice-channels", voice_channel_routes())
        .nest("/api/conduct", conduct_routes())
        .nest("/api/levels", level_routes())
        .nest("/api/role-panels", role_panel_routes())
        .nest("/api/auto-roles", auto_role_routes())
        .nest("/api/stats", stats_routes())
        .route("/api/stats", get(handlers::dashboard::get_dashboard_stats))
        .nest("/api", dashboard_routes())
        .route("/api/charts/activity", get(handlers::dashboard_charts::get_activity_trend))
        .route("/api/audit-logs", get(handlers::audit_logs::list_audit_logs).post(handlers::audit_logs::create_audit_log))
        .route("/api/watched-users", get(handlers::watched_users::list_watched_users).post(handlers::watched_users::add_watched_user))
        .route("/api/watched-users/{guild_id}/{user_id}", get(handlers::watched_users::get_user_dossier).delete(handlers::watched_users::remove_watched_user))
        .route("/api/discord-roles/{guild_id}", get(handlers::discord_roles::list_roles))
        .route("/api/discord-roles/{guild_id}/sync", post(handlers::discord_roles::sync_roles))
        .route("/api/discord-roles/{guild_id}/create", post(handlers::discord_roles::create_role))
        .route("/api/discord-roles/{guild_id}/{role_id}", patch(handlers::discord_roles::edit_role).delete(handlers::discord_roles::delete_role))
        .nest("/api/members", member_routes())
        .nest("/api/coude", coude_routes())
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    let public = Router::new().route("/health", get(handlers::health::health));

    Router::new()
        .merge(protected)
        .merge(public)
        .with_state(state)
}

pub fn build(state: AppState, max_body_size: usize, rate_limit_per_sec: u64, allowed_origins: &str) -> Router {
    let limiter = RateLimiter::new(rate_limit_per_sec);

    // Limiter strict pour les endpoints lourds (inference IA, analytics)
    // Par defaut : 5 req/s (burst 50) vs standard qui est typiquement 50-100 req/s
    let heavy_rate: u64 = std::env::var("HEAVY_RATE_LIMIT_PER_SEC")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);
    let heavy_limiter = RateLimiter::new(heavy_rate);

    // Spawn cleanup tasks (purge stale IP buckets every 60s)
    let limiter_cleanup = limiter.clone();
    let heavy_cleanup = heavy_limiter.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            limiter_cleanup.cleanup().await;
            heavy_cleanup.cleanup().await;
        }
    });

    // Routes lourdes avec rate limit strict (inference IA + analytics)
    let heavy_routes = Router::new()
        .route("/analyze", post(handlers::analyze::analyze))
        .route("/analyze/image", post(handlers::analyze_image::analyze_image))
        .nest("/api/analytics", analytics_routes())
        .route_layer(middleware::from_fn_with_state(
            heavy_limiter,
            rate_limit_middleware,
        ));

    // Routes protegees par auth + rate limit standard
    let protected = Router::new()
        // Routes lourdes (limiter strict)
        .merge(heavy_routes)
        // Bot-facing routes (scoring, rules, infractions) — sans /analyze (deplace dans heavy)
        .merge(bot_routes_standard())
        // App-facing routes (nested by domain)
        .nest("/api/tickets", ticket_routes())
        .nest("/api/security", security_routes())
        .nest("/api/moderation", moderation_routes())
        .nest("/api/strikes", strikes_routes())
        .nest("/api/notes", notes_routes())
        .nest("/api/reminders", reminders_routes())
        .nest("/api/voice-channels", voice_channel_routes())
        .nest("/api/conduct", conduct_routes())
        .nest("/api/levels", level_routes())
        .nest("/api/role-panels", role_panel_routes())
        .nest("/api/auto-roles", auto_role_routes())
        .nest("/api/stats", stats_routes())
        // Dashboard stats (hors du nest /api pour eviter le conflit avec /api/stats)
        .route("/api/stats", get(handlers::dashboard::get_dashboard_stats))
        // Dashboard & config routes
        .nest("/api", dashboard_routes())
        // Charts
        .route("/api/charts/activity", get(handlers::dashboard_charts::get_activity_trend))
        // Audit logs
        .route("/api/audit-logs", get(handlers::audit_logs::list_audit_logs).post(handlers::audit_logs::create_audit_log))
        // Watched users
        .route("/api/watched-users", get(handlers::watched_users::list_watched_users).post(handlers::watched_users::add_watched_user))
        .route("/api/watched-users/{guild_id}/{user_id}", get(handlers::watched_users::get_user_dossier).delete(handlers::watched_users::remove_watched_user))
        // Discord roles (CRUD + sync)
        .route("/api/discord-roles/{guild_id}", get(handlers::discord_roles::list_roles))
        .route("/api/discord-roles/{guild_id}/sync", post(handlers::discord_roles::sync_roles))
        .route("/api/discord-roles/{guild_id}/create", post(handlers::discord_roles::create_role))
        .route("/api/discord-roles/{guild_id}/{role_id}", patch(handlers::discord_roles::edit_role).delete(handlers::discord_roles::delete_role))
        // Bot persistence (fire-and-forget endpoints for bot data)
        .route("/api/name-history", post(handlers::bot_persistence::create_name_history))
        .route("/api/levels/{guild_id}/{user_id}/streak", patch(handlers::bot_persistence::update_streak))
        .route("/api/tickets/{id}/sla", patch(handlers::bot_persistence::update_ticket_sla))
        .route("/api/sponsorships", post(handlers::bot_persistence::create_sponsorship))
        .route("/api/sponsorships/{guild_id}", get(handlers::bot_persistence::list_sponsorships))
        .route("/api/temp-roles", post(handlers::bot_persistence::create_temp_role))
        .route("/api/temp-roles/{guild_id}", get(handlers::bot_persistence::list_temp_roles))
        .route("/api/temp-roles/{guild_id}/{user_id}/{role_id}", delete(handlers::bot_persistence::delete_temp_role))
        .route("/api/moderation/pending", post(handlers::bot_persistence::create_pending_action))
        .route("/api/moderation/pending/guild/{guild_id}", get(handlers::bot_persistence::list_pending_actions))
        .route("/api/moderation/pending/{id}/resolve", patch(handlers::bot_persistence::resolve_pending_action))
        // Members (DB-backed)
        .nest("/api/members", member_routes())
        // Guild members (direct Discord API)
        .route("/api/guilds/{guild_id}/members", get(handlers::guild_members::list_members))
        // User activity (surveillance)
        .route("/api/user-activity", post(handlers::user_activity::create_activity))
        .route("/api/user-activity/{guild_id}/{user_id}", get(handlers::user_activity::get_activity))
        // Models status (IA)
        .route("/api/models/status", get(handlers::models_status::get_models_status))
        .route("/api/models/reload", post(handlers::models_status::reload_model))
        // Cache monitoring
        .route("/api/cache/stats", get(handlers::cache_stats::get_cache_stats))
        // Coup de coude
        .nest("/api/coude", coude_routes())
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .route_layer(middleware::from_fn_with_state(
            limiter,
            rate_limit_middleware,
        ));

    // Routes publiques
    let public = Router::new().route("/health", get(handlers::health::health));

    // TraceLayer configure pour inclure le request_id dans chaque span
    let trace_layer = TraceLayer::new_for_http().make_span_with(|request: &axum::http::Request<_>| {
        let request_id = request
            .headers()
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("-");

        tracing::info_span!(
            "http_request",
            method = %request.method(),
            uri = %request.uri(),
            request_id = %request_id,
        )
    }).on_response(|response: &axum::http::Response<_>, latency: std::time::Duration, _span: &Span| {
        tracing::info!(
            status = response.status().as_u16(),
            latency_ms = latency.as_millis() as u64,
            "response"
        );
    });

    let log_repo = state.log_repo.clone();

    Router::new()
        .merge(protected)
        .merge(public)
        .layer(middleware::from_fn_with_state(log_repo, api_logger_middleware))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(RequestBodyLimitLayer::new(max_body_size))
        .layer(trace_layer)
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        // Security headers
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::HeaderName::from_static("x-xss-protection"),
            HeaderValue::from_static("1; mode=block"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::HeaderName::from_static("permissions-policy"),
            HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
        .layer(build_cors(allowed_origins))
        .with_state(state)
}
