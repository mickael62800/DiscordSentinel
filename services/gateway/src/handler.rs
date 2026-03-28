use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{ConnectInfo, Query, State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use tracing::{info, warn};

use crate::broadcaster::EventBroadcaster;
use crate::logger::GatewayLogger;

#[derive(Debug, serde::Deserialize)]
pub struct WsQuery {
    pub token: Option<String>,
}

#[derive(Clone)]
pub struct GatewayState {
    pub broadcaster: Arc<EventBroadcaster>,
    pub api_key: String,
    pub logger: Arc<GatewayLogger>,
}

/// Handler WebSocket — auth via query param ?token=
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    State(state): State<GatewayState>,
) -> Response {
    // Auth check
    if !state.api_key.is_empty() {
        match query.token {
            Some(ref t) if t == &state.api_key => {}
            _ => {
                warn!("WebSocket rejected: invalid token");
                state.logger.warn("Connexion WebSocket refusee : token invalide", serde_json::json!({
                    "event": "auth_rejected",
                    "client_ip": addr.to_string(),
                }));
                return StatusCode::UNAUTHORIZED.into_response();
            }
        }
    }

    let logger = state.logger.clone();
    let client_ip = addr.to_string();
    ws.on_upgrade(move |socket| handle_socket(socket, state.broadcaster, logger, client_ip))
}

async fn handle_socket(mut socket: WebSocket, broadcaster: Arc<EventBroadcaster>, logger: Arc<GatewayLogger>, client_ip: String) {
    // Verifier la limite de connexions
    let mut rx = match broadcaster.subscribe() {
        Some(rx) => rx,
        None => {
            warn!("WebSocket rejected: max connections reached");
            logger.error("Connexion WebSocket refusee : limite atteinte", serde_json::json!({
                "event": "max_connections",
                "client_ip": &client_ip,
                "connected": broadcaster.connected_count(),
            }));
            let _ = socket
                .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                    code: 1013,
                    reason: "Too many connections".into(),
                })))
                .await;
            return;
        }
    };

    let clients = broadcaster.connected_count();
    info!(clients, "WebSocket client connected");
    logger.info("Client WebSocket connecte", serde_json::json!({
        "event": "client_connected",
        "client_ip": &client_ip,
        "total_clients": clients,
    }));

    let mut events_relayed: u64 = 0;

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
                                events_relayed += 1;
                            }
                            Err(e) => {
                                warn!(error = %e, "Failed to serialize event");
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!(skipped = n, "Client lagged");
                        logger.warn("Client WebSocket en retard", serde_json::json!({
                            "event": "client_lagged",
                            "client_ip": &client_ip,
                            "skipped_events": n,
                        }));
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
    let clients = broadcaster.connected_count();
    info!(clients, "WebSocket client disconnected");
    logger.info("Client WebSocket deconnecte", serde_json::json!({
        "event": "client_disconnected",
        "client_ip": &client_ip,
        "total_clients": clients,
        "events_relayed": events_relayed,
    }));
}
