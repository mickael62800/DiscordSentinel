use serde::Serialize;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize)]
pub struct WsEvent {
    pub event: String,
    pub data: serde_json::Value,
}

pub struct EventBroadcaster {
    tx: broadcast::Sender<WsEvent>,
}

impl EventBroadcaster {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<WsEvent> {
        self.tx.subscribe()
    }

    pub fn broadcast(&self, event: &str, data: serde_json::Value) {
        let ws_event = WsEvent {
            event: event.to_string(),
            data,
        };
        // Ignore error (no subscribers)
        let _ = self.tx.send(ws_event);
    }
}
