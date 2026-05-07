//! Worker periodique : poll les indicateurs critiques (auth failures,
//! patterns suspects, fail2ban, container changes, TLS expiration) et
//! envoie un webhook Discord quand un seuil est franchi. Configure via :
//!   SECURITY_ALERTS_WEBHOOK (Discord webhook URL)
//!   SECURITY_ALERTS_INTERVAL_SECS (defaut 300 = 5 min)

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use sqlx::PgPool;
use tokio::sync::Mutex;

use super::container_monitor::ContainerMonitorState;
use tokio::sync::RwLock;

pub fn spawn(pg_pool: PgPool, container_state: Option<Arc<RwLock<ContainerMonitorState>>>) {
    let webhook = match std::env::var("SECURITY_ALERTS_WEBHOOK") {
        Ok(s) if !s.is_empty() => s,
        _ => {
            tracing::info!("SECURITY_ALERTS_WEBHOOK non defini, alertes desactivees");
            return;
        }
    };
    let interval_secs: u64 = std::env::var("SECURITY_ALERTS_INTERVAL_SECS")
        .ok().and_then(|s| s.parse().ok()).unwrap_or(300);
    tokio::spawn(async move {
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build() {
            Ok(c) => c,
            Err(e) => { tracing::warn!("alerts client: {e}"); return; }
        };
        let already_sent: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
        loop {
            tokio::time::sleep(Duration::from_secs(interval_secs)).await;
            let mut alerts: Vec<(String, String, u32)> = Vec::new(); // (key, content, color)

            // 1) Auth failures > 50 sur la derniere heure
            if let Ok(row) = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM logs WHERE created_at > NOW() - INTERVAL '1 hour' AND status_code IN (401, 403)"
            ).fetch_one(&pg_pool).await {
                if row > 50 {
                    let key = format!("auth-failures-{}", chrono::Utc::now().format("%Y%m%d%H"));
                    alerts.push((key, format!("🚨 **{} echecs d'auth (1h)** - possible brute-force", row), 0xE74C3C));
                }
            }

            // 2) Container changes recents (non-info)
            if let Some(ref cs) = container_state {
                let s = cs.read().await;
                let critical: Vec<_> = s.recent_changes.iter()
                    .filter(|c| c.kind == "removed" || c.kind == "image_changed")
                    .collect();
                if !critical.is_empty() {
                    let key = format!("container-{}", critical[0].timestamp);
                    let names: Vec<String> = critical.iter().take(5)
                        .map(|c| format!("`{}` ({})", c.container.name, c.kind))
                        .collect();
                    alerts.push((key, format!("🐳 **Conteneurs modifies** : {}", names.join(", ")), 0xF39C12));
                }
            }

            // 3) TLS expiration < 14j (lit le shim si dispo)
            if let Ok(s) = std::fs::read_to_string("/var/lib/sentinel/tls-cert.json") {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                    if let Some(days) = v.get("days_until_expiry").and_then(|x| x.as_i64()) {
                        if days < 14 {
                            let key = format!("tls-expiry-{}", days);
                            alerts.push((key, format!("🔐 **Cert TLS expire dans {} jours**", days), 0xF39C12));
                        }
                    }
                }
            }

            // Envoie Discord pour chaque alerte non encore envoyee
            let mut sent = already_sent.lock().await;
            for (key, content, color) in alerts {
                if sent.contains(&key) { continue; }
                let body = serde_json::json!({
                    "username": "DiscordSentinel · Securite",
                    "embeds": [{
                        "title": "Alerte securite serveur",
                        "description": content,
                        "color": color,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    }]
                });
                match client.post(&webhook).json(&body).send().await {
                    Ok(r) if r.status().is_success() => {
                        sent.insert(key);
                        if sent.len() > 500 { sent.clear(); }
                    }
                    Ok(r) => tracing::warn!(status = %r.status(), "alerte webhook : status non-2xx"),
                    Err(e) => tracing::warn!(?e, "alerte webhook : erreur envoi"),
                }
            }
        }
    });
}
