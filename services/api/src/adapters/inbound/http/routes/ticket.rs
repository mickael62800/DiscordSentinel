//! Routes tickets (montees sous `/api/tickets`).

use axum::routing::{delete, get, patch, post};
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn ticket_inner() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::tickets::list_tickets).post(handlers::tickets::create_ticket))
        .route("/bulk", delete(handlers::tickets::bulk_delete_tickets))
        .route("/{id}", get(handlers::tickets::get_ticket_detail))
        .route("/{id}/messages", post(handlers::tickets::reply_ticket))
        .route("/{id}/close", patch(handlers::tickets::close_ticket))
        .route("/{id}/assign", patch(handlers::tickets::assign_ticket))
        .route("/{id}/status", patch(handlers::tickets::update_status))
        .route("/{id}/channels", patch(handlers::tickets::update_ticket_channel))
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/tickets", ticket_inner())
}
