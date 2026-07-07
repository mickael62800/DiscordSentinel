use std::time::{Duration, Instant};

use dashmap::DashMap;

/// Tracker SLA pour les tickets.
///
/// Le bot ne fait que mesurer le delai de premiere reponse staff pour le
/// remonter a l'API (`api.update_ticket_sla`). La decision d'escalade/breach
/// vit dans les workers API (events Redis `ticket_sla_*`).
pub struct SlaTracker {
    /// ticket_id -> timestamp de creation
    created: DashMap<String, Instant>,
    /// ticket_id -> timestamp de premiere reponse staff
    first_response: DashMap<String, Instant>,
}

impl SlaTracker {
    pub fn new() -> Self {
        Self {
            created: DashMap::new(),
            first_response: DashMap::new(),
        }
    }

    pub fn record_creation(&self, ticket_id: &str) {
        self.created.insert(ticket_id.to_string(), Instant::now());
    }

    pub fn record_staff_response(&self, ticket_id: &str) -> Option<Duration> {
        if self.first_response.contains_key(ticket_id) {
            return None;
        }

        let created_at = self.created.get(ticket_id)?;
        let now = Instant::now();
        let duration = now.duration_since(*created_at);

        self.first_response.insert(ticket_id.to_string(), now);
        Some(duration)
    }
}

/// Formate une duree en texte lisible.
pub fn format_sla_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let minutes = total_secs / 60;
    let hours = minutes / 60;

    if hours > 0 {
        let remaining_min = minutes % 60;
        if remaining_min > 0 {
            format!("{}h{}min", hours, remaining_min)
        } else {
            format!("{}h", hours)
        }
    } else if minutes > 0 {
        format!("{}min", minutes)
    } else {
        format!("{}s", total_secs)
    }
}
