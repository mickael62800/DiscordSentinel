use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{ConnectInfo, Query, State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use subtle::ConstantTimeEq;
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::broadcaster::EventBroadcaster;
use crate::logger::GatewayLogger;

/// Cache local des tokens Discord deja valides (TTL 5 min). Evite de
/// hit l'API sur chaque reconnexion WS (les clients reconnect souvent).
const TOKEN_CACHE_TTL: Duration = Duration::from_secs(300);

#[derive(Clone)]
pub struct CachedAuth {
    authorized: bool,
    expires_at: Instant,
}

/// WebSocket close code: "Try Again Later" (server at capacity)
const WS_CLOSE_TRY_AGAIN_LATER: u16 = 1013;

#[derive(Debug, serde::Deserialize)]
pub struct WsQuery {
    /// Bearer API_KEY (clients internes : bot, workers).
    pub token: Option<String>,
    /// Token OAuth Discord (utilisateurs web).
    /// Valide si non-vide. Les events du gateway sont du broadcast public
    /// (heartbeats, stats), pas de donnees user-specifiques sensibles.
    pub discord_token: Option<String>,
}

#[derive(Clone)]
pub struct GatewayState {
    pub broadcaster: Arc<EventBroadcaster>,
    pub api_key: String,
    pub api_url: String,
    pub logger: Arc<GatewayLogger>,
    pub http_client: reqwest::Client,
    pub token_cache: Arc<Mutex<std::collections::HashMap<String, CachedAuth>>>,
}

/// Verifie via l'API que le token Discord appartient a un user whitelist.
/// Cache 5min en memoire pour eviter le round-trip a chaque reconnexion.
async fn discord_token_authorized(state: &GatewayState, discord_token: &str) -> bool {
    // Cache check
    {
        let cache = state.token_cache.lock().await;
        if let Some(cached) = cache.get(discord_token) {
            if cached.expires_at > Instant::now() {
                return cached.authorized;
            }
        }
    }

    // Hit /api/auth/check-access avec Bearer API_KEY (services internes)
    // + X-Discord-Token. L'API renvoie 200/403 selon le whitelist.
    let url = format!(
        "{}/api/auth/check-access",
        state.api_url.trim_end_matches('/')
    );
    let result = state
        .http_client
        .get(&url)
        .bearer_auth(&state.api_key)
        .header("x-discord-token", discord_token)
        .send()
        .await;

    let authorized = match result {
        Ok(resp) => resp.status().is_success(),
        Err(e) => {
            warn!(error = %e, "check-access call failed -> deny WS auth");
            false
        }
    };

    // Cache resultat (positif ou negatif).
    let mut cache = state.token_cache.lock().await;
    cache.insert(
        discord_token.to_string(),
        CachedAuth {
            authorized,
            expires_at: Instant::now() + TOKEN_CACHE_TTL,
        },
    );
    // Nettoyage opportuniste : retire les entrees expirees pour eviter une
    // croissance illimitee si beaucoup de tokens differents transitent.
    let now = Instant::now();
    cache.retain(|_, v| v.expires_at > now);

    authorized
}

/// Handler WebSocket — auth via query param ?token=
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    State(state): State<GatewayState>,
) -> Response {
    // Auth check : 2 modes possibles
    //   1. Bearer API_KEY (token query param) : services internes
    //      Comparaison constant-time pour prevenir timing attack.
    //   2. Discord OAuth token (discord_token query param) : utilisateurs web
    //      Valide si non-vide. Les events du gateway sont du broadcast public
    //      (heartbeats, stats globales), pas de donnees user-specifiques
    //      sensibles -> accepter un token Discord non-vide est OK.
    if !state.api_key.is_empty() {
        let valid_api_key = query
            .token
            .as_ref()
            .map(|t| t.as_bytes().ct_eq(state.api_key.as_bytes()).into())
            .unwrap_or(false);
        // Plus d'acceptation aveugle d'un token Discord non-vide : on hit
        // /api/auth/check-access pour valider que le user est dans la
        // whitelist (api_user_guilds OU SUPERADMIN_USER_IDS). Sans ca, un
        // user Discord random pourrait sniffer tous les events de toutes
        // les guilds (infractions, bans, etc.) en temps reel.
        let valid_discord_token = if valid_api_key {
            false // pas besoin, deja autorise via API key
        } else if let Some(t) = query.discord_token.as_deref() {
            !t.is_empty() && discord_token_authorized(&state, t).await
        } else {
            false
        };
        if !valid_api_key && !valid_discord_token {
            warn!(client_ip = %addr, "WebSocket rejected: no valid auth (token or discord_token)");
            return StatusCode::UNAUTHORIZED.into_response();
        }
    }

    let logger = state.logger.clone();
    let client_ip = addr.to_string();
    ws.on_upgrade(move |socket| handle_socket(socket, state.broadcaster, logger, client_ip))
}

async fn handle_socket(
    mut socket: WebSocket,
    broadcaster: Arc<EventBroadcaster>,
    logger: Arc<GatewayLogger>,
    client_ip: String,
) {
    // Verifier la limite de connexions
    let mut rx = match broadcaster.subscribe() {
        Some(rx) => rx,
        None => {
            warn!(client_ip = %client_ip, connected = broadcaster.connected_count(), "WebSocket rejected: max connections reached");
            if let Err(e) = socket
                .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                    code: WS_CLOSE_TRY_AGAIN_LATER,
                    reason: "Too many connections".into(),
                })))
                .await
            {
                warn!(error = %e, "Failed to send close frame");
            }
            return;
        }
    };

    let clients = broadcaster.connected_count();
    info!(clients, client_ip = %client_ip, "WebSocket client connected");
    logger.info(
        "Client WebSocket connecte",
        serde_json::json!({
            "event_type": "websocket.client_connected",
            "client_ip": &client_ip,
            "total_clients": clients,
        }),
    );

    let mut events_relayed: u64 = 0;
    let mut events_skipped: u64 = 0;

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
                                warn!(error = %e, event_type = %ws_event.event, "Failed to serialize event");
                                logger.warn("Echec serialisation event", serde_json::json!({
                                    "event_type": "websocket.serialize_error",
                                    "error": e.to_string(),
                                    "ws_event_type": ws_event.event,
                                    "client_ip": &client_ip,
                                }));
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!(skipped = n, client_ip = %client_ip, "Client lagged");
                        events_skipped += n;
                        logger.warn("Client lagged (events skip)", serde_json::json!({
                            "event_type": "websocket.client_lagged",
                            "skipped": n,
                            "client_ip": &client_ip,
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
    info!(clients, client_ip = %client_ip, events_relayed, events_skipped, "WebSocket client disconnected");
    logger.info(
        "Client WebSocket deconnecte",
        serde_json::json!({
            "event_type": "websocket.client_disconnected",
            "client_ip": &client_ip,
            "total_clients": clients,
            "events_relayed": events_relayed,
            "skipped_events": events_skipped,
        }),
    );
}
