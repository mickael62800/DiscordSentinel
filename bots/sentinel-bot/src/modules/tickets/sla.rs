use std::time::{Duration, Instant};

use dashmap::DashMap;

/// Tracker SLA pour les tickets.
pub struct SlaTracker {
    /// ticket_id -> timestamp de creation
    created: DashMap<String, Instant>,
    /// ticket_id -> timestamp de premiere reponse staff
    first_response: DashMap<String, Instant>,
    /// ticket_id -> flag "deja escalade"
    escalated: DashMap<String, bool>,
}

#[allow(dead_code)]
impl SlaTracker {
    pub fn new() -> Self {
        Self {
            created: DashMap::new(),
            first_response: DashMap::new(),
            escalated: DashMap::new(),
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

    pub fn record_close(&self, ticket_id: &str) -> Option<(Duration, Option<Duration>)> {
        let created_at = self.created.remove(ticket_id)?;
        let now = Instant::now();
        let resolution = now.duration_since(created_at.1);

        let first_response = self
            .first_response
            .remove(ticket_id)
            .map(|(_, fr)| fr.duration_since(created_at.1));

        self.escalated.remove(ticket_id);

        Some((resolution, first_response))
    }

    pub fn breached_tickets(&self, max_minutes: u64) -> Vec<String> {
        let threshold = Duration::from_secs(max_minutes * 60);
        let now = Instant::now();

        self.created
            .iter()
            .filter(|entry| {
                let ticket_id = entry.key();
                let created = entry.value();
                !self.first_response.contains_key(ticket_id)
                    && now.duration_since(*created) > threshold
            })
            .map(|entry| entry.key().clone())
            .collect()
    }

    pub fn mark_escalated(&self, ticket_id: &str) {
        self.escalated.insert(ticket_id.to_string(), true);
    }

    pub fn is_escalated(&self, ticket_id: &str) -> bool {
        self.escalated.contains_key(ticket_id)
    }

    pub fn cleanup_stale(&self) {
        let now = Instant::now();
        let max_age = Duration::from_secs(48 * 3600);
        self.created.retain(|_, ts| now.duration_since(*ts) < max_age);
        self.first_response.retain(|id, _| self.created.contains_key(id));
        self.escalated.retain(|id, _| self.created.contains_key(id));
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
