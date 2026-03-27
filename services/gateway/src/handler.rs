use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Query, State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use tracing::{info, warn};

use crate::broadcaster::EventBroadcaster;

#[derive(Debug, serde::Deserialize)]
pub struct WsQuery {
    pub token: Option<String>,
}

#[derive(Clone)]
pub struct GatewayState {
    pub broadcaster: Arc<EventBroadcaster>,
    pub api_key: String,
}

/// Handler WebSocket — auth via query param ?token=
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    State(state): State<GatewayState>,
) -> Response {
    // Auth check
    if !state.api_key.is_empty() {
        match query.token {
            Some(ref t) if t == &state.api_key => {}
            _ => {
                warn!("WebSocket rejected: invalid token");
                return StatusCode::UNAUTHORIZED.into_response();
            }
        }
    }

    ws.on_upgrade(move |socket| handle_socket(socket, state.broadcaster))
}

async fn handle_socket(mut socket: WebSocket, broadcaster: Arc<EventBroadcaster>) {
    // Verifier la limite de connexions
    let mut rx = match broadcaster.subscribe() {
        Some(rx) => rx,
        None => {
            warn!("WebSocket rejected: max connections reached");
            let _ = socket
                .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                    code: 1013, // Try Again Later
                    reason: "Too many connections".into(),
                })))
                .await;
            return;
        }
    };

    info!(
        clients = broadcaster.connected_count(),
        "WebSocket client connected"
    );

    loop {
        tokio::select! {
            event = rx.recv() => {
                match event {
                    Ok(ws_event) => {
                        match serde_json::to_string(&ws_event) {
                            Ok(json) => {
                                if socket.send(Message::Text(json.into())).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!(error = %e, "Failed to serialize event");
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!(skipped = n, "Client lagged");
                    }
                    Err(_) => break,
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(data))) => {
                        if socket.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    broadcaster.unsubscribe();
    info!(
        clients = broadcaster.connected_count(),
        "WebSocket client disconnected"
    );
}
