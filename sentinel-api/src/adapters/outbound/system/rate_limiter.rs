//! Rate limiter dynamique en memoire : compte les requetes par IP sur une
//! fenetre glissante, declenche un ban auto via le shim ban-apply quand
//! le seuil est depasse. Configure via env :
//!   RATE_LIMIT_THRESHOLD (defaut 200 req/min)
//!   RATE_LIMIT_WINDOW_SECS (defaut 60)
//!   RATE_LIMIT_BAN_DURATION_HOURS (defaut 1, indicatif pour fail2ban)

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use tokio::sync::Mutex;

pub struct RateLimiter {
    pub threshold: usize,
    pub window: Duration,
    counts: DashMap<String, Mutex<VecDeque<Instant>>>,
    /// IPs deja bannies recemment pour eviter de spammer le fichier de ban
    recent_bans: DashMap<String, Instant>,
}

impl RateLimiter {
    pub fn from_env() -> Self {
        let threshold = std::env::var("RATE_LIMIT_THRESHOLD")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(200);
        let window_secs = std::env::var("RATE_LIMIT_WINDOW_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(60);
        Self {
            threshold,
            window: Duration::from_secs(window_secs),
            counts: DashMap::new(),
            recent_bans: DashMap::new(),
        }
    }

    /// A appeler dans le middleware pour chaque requete. Retourne true si
    /// l'IP doit etre bannie maintenant (pour declenchement async).
    pub async fn observe(&self, ip: &str) -> bool {
        if ip.is_empty() || ip == "unknown" {
            return false;
        }
        // Skip si deja banni dans les 5 dernieres minutes
        if let Some(t) = self.recent_bans.get(ip) {
            if t.elapsed() < Duration::from_secs(300) {
                return false;
            }
        }
        let now = Instant::now();
        let entry = self
            .counts
            .entry(ip.to_string())
            .or_insert_with(|| Mutex::new(VecDeque::new()));
        let mut q = entry.value().lock().await;
        // Purge les entrees hors fenetre
        while let Some(front) = q.front() {
            if now.duration_since(*front) > self.window {
                q.pop_front();
            } else {
                break;
            }
        }
        q.push_back(now);
        if q.len() >= self.threshold {
            self.recent_bans.insert(ip.to_string(), now);
            q.clear();
            return true;
        }
        false
    }

    /// Ecrit l'IP dans le fichier de ban consume par le shim ban-apply.
    pub async fn trigger_ban(self: &Arc<Self>, ip: String) {
        let path = "/var/lib/sentinel/ban-requests.json";
        let entry = serde_json::json!({
            "ip": ip,
            "reason": format!("rate-limit auto: > {} req/{:?}", self.threshold, self.window),
            "requested_at": chrono::Utc::now().to_rfc3339(),
        });
        // Append a la liste si fichier existe, sinon cree
        let existing: Vec<serde_json::Value> = std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let mut all = existing;
        all.push(entry);
        if let Ok(s) = serde_json::to_string_pretty(&all) {
            let _ = std::fs::write(path, s);
        }
        tracing::warn!(ip = %ip, "rate-limit auto-ban declenche");
    }
}
