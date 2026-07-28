//! Couche HTTP axum : router, auth Bearer, handlers.

pub mod handlers;

use axum::extract::Request;
use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware;
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::get;
use axum::routing::post;
use axum::Router;

use crate::bootstrap::AppState;

/// Construit le router complet (routes + auth Bearer NEXUS_API_KEY).
pub fn build_router(state: AppState) -> Router {
    let api = Router::new()
        .route("/api/wheel/{guild_id}/{user_id}/spin", post(handlers::wheel::spin))
        .route("/api/wallet/{guild_id}/{user_id}", get(handlers::wallet::get))
        .layer(middleware::from_fn_with_state(state.clone(), require_api_key));

    Router::new()
        .route("/health", get(|| async { "ok" }))
        .merge(api)
        .with_state(state)
}

/// Auth simple : si NEXUS_API_KEY est definie, exige `Authorization: Bearer <key>`
/// sur toutes les routes /api (comme sentinel-api). /health reste ouvert.
async fn require_api_key(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let Some(expected) = &state.api_key else {
        return Ok(next.run(req).await);
    };
    let authorized = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .is_some_and(|token| token == expected);
    if !authorized {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(next.run(req).await)
}
