use std::sync::atomic::{AtomicUsize, Ordering};

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Evenement WebSocket transmis aux clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsEvent {
    pub event: String,
    pub data: serde_json::Value,
}

/// Broadcaster local — distribue les events recus de Redis vers les clients WebSocket.
pub struct EventBroadcaster {
    tx: broadcast::Sender<WsEvent>,
    connected_clients: AtomicUsize,
    max_connections: usize,
}

impl EventBroadcaster {
    pub fn new(capacity: usize, max_connections: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self {
            tx,
            connected_clients: AtomicUsize::new(0),
            max_connections,
        }
    }

    pub fn subscribe(&self) -> Option<broadcast::Receiver<WsEvent>> {
        let current = self.connected_clients.load(Ordering::Relaxed);
        if current >= self.max_connections {
            return None;
        }
        self.connected_clients.fetch_add(1, Ordering::Relaxed);
        Some(self.tx.subscribe())
    }

    pub fn unsubscribe(&self) {
        self.connected_clients.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn broadcast(&self, event: WsEvent) {
        let _ = self.tx.send(event);
    }

    pub fn connected_count(&self) -> usize {
        self.connected_clients.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscribe_increments_count() {
        let broadcaster = EventBroadcaster::new(16, 100);
        assert_eq!(broadcaster.connected_count(), 0);

        let _rx = broadcaster.subscribe();
        assert_eq!(broadcaster.connected_count(), 1);

        let _rx2 = broadcaster.subscribe();
        assert_eq!(broadcaster.connected_count(), 2);
    }

    #[test]
    fn test_unsubscribe_decrements_count() {
        let broadcaster = EventBroadcaster::new(16, 100);
        let _rx = broadcaster.subscribe();
        assert_eq!(broadcaster.connected_count(), 1);

        broadcaster.unsubscribe();
        assert_eq!(broadcaster.connected_count(), 0);
    }

    #[test]
    fn test_max_connections_enforced() {
        let broadcaster = EventBroadcaster::new(16, 2);

        let _rx1 = broadcaster.subscribe();
        let _rx2 = broadcaster.subscribe();
        assert_eq!(broadcaster.connected_count(), 2);

        // 3eme connexion refusee
        let rx3 = broadcaster.subscribe();
        assert!(rx3.is_none());
        assert_eq!(broadcaster.connected_count(), 2);
    }

    #[test]
    fn test_broadcast_received_by_subscriber() {
        let broadcaster = EventBroadcaster::new(16, 100);
        let mut rx = broadcaster.subscribe().unwrap();

        let event = WsEvent {
            event: "test".to_string(),
            data: serde_json::json!({"key": "value"}),
        };
        broadcaster.broadcast(event.clone());

        let received = rx.try_recv().unwrap();
        assert_eq!(received.event, "test");
        assert_eq!(received.data["key"], "value");
    }

    #[test]
    fn test_broadcast_no_subscriber_no_panic() {
        let broadcaster = EventBroadcaster::new(16, 100);
        // Pas de subscriber, ne doit pas paniquer
        broadcaster.broadcast(WsEvent {
            event: "orphan".to_string(),
            data: serde_json::json!(null),
        });
    }

    #[test]
    fn test_ws_event_serde_roundtrip() {
        let event = WsEvent {
            event: "infraction_new".to_string(),
            data: serde_json::json!({"username": "test", "action": "ban"}),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: WsEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.event, "infraction_new");
        assert_eq!(back.data["action"], "ban");
    }
}
