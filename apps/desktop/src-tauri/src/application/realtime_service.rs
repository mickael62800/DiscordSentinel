use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures_util::StreamExt;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use tokio_tungstenite::connect_async;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WsEvent {
    pub event: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct WsStatus {
    pub connected: bool,
    pub url: String,
}

pub struct RealtimeService {
    connected: Arc<AtomicBool>,
    ws_url: Arc<Mutex<String>>,
    api_key: Arc<Mutex<String>>,
    shutdown: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
}

impl RealtimeService {
    pub fn new() -> Self {
        Self {
            connected: Arc::new(AtomicBool::new(false)),
            ws_url: Arc::new(Mutex::new(String::new())),
            api_key: Arc::new(Mutex::new(String::new())),
            shutdown: Arc::new(Mutex::new(None)),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    pub async fn get_status(&self) -> WsStatus {
        WsStatus {
            connected: self.is_connected(),
            url: self.ws_url.lock().await.clone(),
        }
    }

    pub async fn connect(&self, app: AppHandle, api_url: String, api_key: String) -> Result<(), String> {
        // Disconnect existing connection first
        self.disconnect().await;

        // Gateway WebSocket is on a separate port (API port + 1, default 3001)
        // e.g. http://localhost:3000 -> ws://localhost:3001
        let gateway_url = std::env::var("GATEWAY_URL").unwrap_or_else(|_| {
            derive_gateway_url(&api_url)
        });

        let ws_url = gateway_url
            .replace("https://", "wss://")
            .replace("http://", "ws://");
        let ws_url = if ws_url.starts_with("ws") { ws_url } else { format!("ws://{}", ws_url) };
        let ws_url = format!("{}/ws", ws_url.trim_end_matches('/'));

        *self.ws_url.lock().await = ws_url.clone();
        *self.api_key.lock().await = api_key.clone();

        // Add API key as query param for WS auth
        let connect_url = if api_key.is_empty() {
            ws_url.clone()
        } else {
            format!("{}?token={}", ws_url, api_key)
        };

        let connected = self.connected.clone();
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        *self.shutdown.lock().await = Some(shutdown_tx);

        let ws_url_clone = ws_url.clone();

        tokio::spawn(async move {
            loop {
                match connect_async(&connect_url).await {
                    Ok((ws_stream, _)) => {
                        connected.store(true, Ordering::Relaxed);
                        let _ = app.emit("ws:connected", WsStatus {
                            connected: true,
                            url: ws_url_clone.clone(),
                        });

                        let (_, mut read) = ws_stream.split();

                        loop {
                            tokio::select! {
                                msg = read.next() => {
                                    match msg {
                                        Some(Ok(message)) => {
                                            if let tokio_tungstenite::tungstenite::Message::Text(text) = message {
                                                if let Ok(event) = serde_json::from_str::<WsEvent>(&text) {
                                                    let event_name = format!("ws:{}", event.event);
                                                    let _ = app.emit(&event_name, &event.data);
                                                    // Also emit generic event for the notification system
                                                    let _ = app.emit("ws:event", &event);
                                                }
                                            }
                                        }
                                        Some(Err(_)) | None => {
                                            // Connection lost
                                            break;
                                        }
                                    }
                                }
                                _ = &mut shutdown_rx => {
                                    connected.store(false, Ordering::Relaxed);
                                    let _ = app.emit("ws:disconnected", ());
                                    return; // Exit task completely
                                }
                            }
                        }

                        // Connection lost, will reconnect
                        connected.store(false, Ordering::Relaxed);
                        let _ = app.emit("ws:disconnected", ());
                    }
                    Err(e) => {
                        eprintln!("WebSocket connection failed: {}", e);
                    }
                }

                // Check if we should stop before reconnecting
                if shutdown_rx.try_recv().is_ok() {
                    connected.store(false, Ordering::Relaxed);
                    return;
                }

                // Reconnect after 5 seconds
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        });

        Ok(())
    }

    pub async fn disconnect(&self) {
        if let Some(tx) = self.shutdown.lock().await.take() {
            let _ = tx.send(());
        }
        self.connected.store(false, Ordering::Relaxed);
    }
}

/// Derive gateway URL from API URL (port + 1).
/// e.g. "http://localhost:3000" -> "http://localhost:3001"
fn derive_gateway_url(api_url: &str) -> String {
    // Try to parse and increment port
    if let Some(colon_pos) = api_url.rfind(':') {
        let (base, port_str) = api_url.split_at(colon_pos + 1);
        // Remove trailing slash if any
        let port_str = port_str.trim_end_matches('/');
        if let Ok(port) = port_str.parse::<u16>() {
            return format!("{}{}", base, port + 1);
        }
    }
    // Fallback: append :3001
    format!("{}:3001", api_url.trim_end_matches('/'))
}
