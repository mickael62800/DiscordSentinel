use axum::middleware;
use axum::routing::{delete, get, patch, post};
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use super::handlers;
use super::middleware::auth::auth_middleware;
use super::state::AppState;
use crate::adapters::inbound::ws::handler::ws_handler;

pub fn build(state: AppState) -> Router {
    // Routes protégées par auth
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
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    // WebSocket (auth via query param ?token=)
    let ws_state = (state.broadcaster.clone(), state.api_key.clone());
    let ws_route = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(ws_state);

    // Routes publiques
    let public = Router::new().route("/health", get(handlers::health::health));

    Router::new()
        .merge(protected)
        .merge(ws_route)
        .merge(public)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
