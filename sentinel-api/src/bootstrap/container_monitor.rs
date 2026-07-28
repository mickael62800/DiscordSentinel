//! Poll Docker en arriere-plan, detecte added/removed/restarted/image_changed
//! et logue dans server_events. Garde un snapshot + 24h d'historique.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use bollard::container::ListContainersOptions;
use bollard::Docker;
use sqlx::PgPool;
use tokio::sync::RwLock;

use crate::adapters::inbound::http::handlers::system::security::{
    ContainerChangeEntry, ContainerSnapshot,
};

#[derive(Default, Debug, Clone)]
pub struct ContainerMonitorState {
    pub last_check: String,
    pub current: Vec<ContainerSnapshot>,
    pub recent_changes: Vec<ContainerChangeEntry>,
}

pub fn spawn(pg_pool: PgPool) -> Arc<RwLock<ContainerMonitorState>> {
    let state = Arc::new(RwLock::new(ContainerMonitorState::default()));
    let st = state.clone();
    tokio::spawn(async move {
        let docker = match Docker::connect_with_local_defaults() {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("container_monitor : Docker indisponible : {e}");
                return;
            }
        };
        let mut prev: HashMap<String, ContainerSnapshot> = HashMap::new();
        let mut first_run = true;
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            let opts = ListContainersOptions::<String> {
                all: true,
                ..Default::default()
            };
            let conts = match docker.list_containers(Some(opts)).await {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!("container_monitor list : {e}");
                    continue;
                }
            };
            let now = chrono::Utc::now().to_rfc3339();
            let mut current_map: HashMap<String, ContainerSnapshot> = HashMap::new();
            let mut current_vec: Vec<ContainerSnapshot> = Vec::new();
            for c in conts {
                let id = c.id.clone().unwrap_or_default();
                if id.is_empty() {
                    continue;
                }
                let name = c
                    .names
                    .as_ref()
                    .and_then(|v| v.first())
                    .map(|s| s.trim_start_matches('/').to_string())
                    .unwrap_or_default();
                let snap = ContainerSnapshot {
                    id: id.clone(),
                    name,
                    image: c.image.clone().unwrap_or_default(),
                    state: c.state.map(|s| format!("{:?}", s)).unwrap_or_default(),
                    started_at: c.created.map(|t| t.to_string()),
                };
                current_map.insert(id.clone(), snap.clone());
                current_vec.push(snap);
            }

            // Diff (skip first run pour pas tout marquer comme nouveau)
            let mut changes: Vec<ContainerChangeEntry> = Vec::new();
            if !first_run {
                for (id, snap) in &current_map {
                    match prev.get(id) {
                        None => changes.push(ContainerChangeEntry {
                            timestamp: now.clone(),
                            kind: "added".into(),
                            container: snap.clone(),
                            previous: None,
                        }),
                        Some(p) if p.image != snap.image => changes.push(ContainerChangeEntry {
                            timestamp: now.clone(),
                            kind: "image_changed".into(),
                            container: snap.clone(),
                            previous: Some(p.clone()),
                        }),
                        Some(p) if p.state != snap.state => changes.push(ContainerChangeEntry {
                            timestamp: now.clone(),
                            kind: "state_changed".into(),
                            container: snap.clone(),
                            previous: Some(p.clone()),
                        }),
                        _ => {}
                    }
                }
                for (id, snap) in &prev {
                    if !current_map.contains_key(id) {
                        changes.push(ContainerChangeEntry {
                            timestamp: now.clone(),
                            kind: "removed".into(),
                            container: snap.clone(),
                            previous: None,
                        });
                    }
                }
            }
            first_run = false;
            prev = current_map;

            // Logue chaque change dans server_events
            for ch in &changes {
                let action = format!("docker.{}", ch.kind);
                let target = format!(
                    "{} ({})",
                    ch.container.name,
                    &ch.container.id[..12.min(ch.container.id.len())]
                );
                let details = serde_json::to_value(ch).unwrap_or(serde_json::Value::Null);
                let severity = if ch.kind == "removed" || ch.kind == "added" {
                    "warn"
                } else {
                    "info"
                };
                let _ = sqlx::query(
                    "INSERT INTO server_events (timestamp, actor, actor_name, action, target, severity, details)
                     VALUES (NOW(), $1, NULL, $2, $3, $4, $5)"
                )
                .bind("system:container_monitor")
                .bind(&action)
                .bind(&target)
                .bind(severity)
                .bind(&details)
                .execute(&pg_pool)
                .await;
            }

            // Update state public + garde 24h max
            let mut w = st.write().await;
            w.last_check = now.clone();
            w.current = current_vec;
            for ch in changes {
                w.recent_changes.push(ch);
            }
            // Trim : garde 200 derniers changes
            if w.recent_changes.len() > 200 {
                let drop_n = w.recent_changes.len() - 200;
                w.recent_changes.drain(0..drop_n);
            }
        }
    });
    state
}
