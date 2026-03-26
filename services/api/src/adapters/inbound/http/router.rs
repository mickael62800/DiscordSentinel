use axum::http::{header, HeaderValue, Method};
use axum::middleware;
use axum::routing::{delete, get, patch, post};
use axum::Router;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;
use tracing::Span;

use super::handlers;
use super::middleware::auth::auth_middleware;
use super::middleware::rate_limit::{rate_limit_middleware, RateLimiter};
use super::state::AppState;
use crate::adapters::inbound::ws::handler::ws_handler;

fn build_cors(allowed_origins: &str) -> CorsLayer {
    let allow_origin = if allowed_origins.is_empty() || allowed_origins == "*" {
        AllowOrigin::any()
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

pub fn build(state: AppState, max_body_size: usize, rate_limit_per_sec: u64, allowed_origins: &str) -> Router {
    let limiter = RateLimiter::new(rate_limit_per_sec);

    // Spawn cleanup task (purge stale IP buckets every 60s)
    let limiter_cleanup = limiter.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            limiter_cleanup.cleanup().await;
        }
    });

    // Routes protégées par auth + rate limit
    let protected = Router::new()
        // Bots
        .route("/analyze", post(handlers::analyze::analyze))
        // Rules (scoring — format technique)
        .route("/rules/{guild_id}", get(handlers::rules::get_rules))
        .route("/rules", post(handlers::rules::create_rule))
        .route(
            "/rules/{guild_id}/{rule_id}",
            delete(handlers::rules::delete_rule),
        )
        // Infractions (par guild)
        .route(
            "/infractions/{guild_id}",
            get(handlers::infractions::list_infractions),
        )
        // App — Tickets
        .route(
            "/api/tickets",
            get(handlers::tickets::list_tickets).post(handlers::tickets::create_ticket),
        )
        .route("/api/tickets/{id}", get(handlers::tickets::get_ticket_detail))
        .route(
            "/api/tickets/{id}/messages",
            post(handlers::tickets::reply_ticket),
        )
        .route(
            "/api/tickets/{id}/close",
            patch(handlers::tickets::close_ticket),
        )
        .route(
            "/api/tickets/{id}/assign",
            patch(handlers::tickets::assign_ticket),
        )
        // Security events
        .route(
            "/api/security/events",
            post(handlers::security::report_event).get(handlers::security::list_events),
        )
        // Moderation actions
        .route(
            "/api/moderation/actions",
            post(handlers::moderation::log_action),
        )
        .route(
            "/api/moderation/history/{guild_id}/{user_id}",
            get(handlers::moderation::get_history),
        )
        // Stats
        .route(
            "/api/stats/messages",
            post(handlers::stats::record_messages),
        )
        .route(
            "/api/stats/voice",
            post(handlers::stats::record_voice),
        )
        .route(
            "/api/stats/{guild_id}/user/{user_id}",
            get(handlers::stats::get_user_stats),
        )
        .route(
            "/api/stats/{guild_id}/overview",
            get(handlers::stats::get_guild_overview),
        )
        .route(
            "/api/stats/{guild_id}/leaderboard",
            get(handlers::stats::get_leaderboard),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .route_layer(middleware::from_fn_with_state(
            limiter,
            rate_limit_middleware,
        ));

    // WebSocket (auth via query param ?token=)
    let ws_state = (state.broadcaster.clone(), state.api_key.clone());
    let ws_route = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(ws_state);

    // Routes publiques
    let public = Router::new().route("/health", get(handlers::health::health));

    // TraceLayer configuré pour inclure le request_id dans chaque span
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

    Router::new()
        .merge(protected)
        .merge(ws_route)
        .merge(public)
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(RequestBodyLimitLayer::new(max_body_size))
        .layer(trace_layer)
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(build_cors(allowed_origins))
        .with_state(state)
}
