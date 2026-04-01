use std::time::{Duration, Instant};

use dashmap::DashMap;

/// Tracker SLA pour les tickets.
/// Mesure le temps de premiere reponse staff et le temps de resolution.
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

    /// Enregistre la creation d'un ticket.
    pub fn record_creation(&self, ticket_id: &str) {
        self.created.insert(ticket_id.to_string(), Instant::now());
    }

    /// Enregistre une reponse staff. Retourne le delai de premiere reponse si c'est la premiere.
    pub fn record_staff_response(&self, ticket_id: &str) -> Option<Duration> {
        if self.first_response.contains_key(ticket_id) {
            return None; // Deja enregistre
        }

        let created_at = self.created.get(ticket_id)?;
        let now = Instant::now();
        let duration = now.duration_since(*created_at);

        self.first_response.insert(ticket_id.to_string(), now);
        Some(duration)
    }

    /// Enregistre la fermeture d'un ticket.
    /// Retourne (resolution_time, first_response_time) si disponible.
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

    /// Retourne les ticket_ids sans reponse staff depassant le delai en minutes.
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

    /// Marque un ticket comme escalade (pour eviter les escalades repetees).
    pub fn mark_escalated(&self, ticket_id: &str) {
        self.escalated.insert(ticket_id.to_string(), true);
    }

    /// Verifie si un ticket a deja ete escalade.
    pub fn is_escalated(&self, ticket_id: &str) -> bool {
        self.escalated.contains_key(ticket_id)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_creation_and_response() {
        let tracker = SlaTracker::new();
        tracker.record_creation("t1");

        // Premiere reponse retourne Some
        let duration = tracker.record_staff_response("t1");
        assert!(duration.is_some());

        // Deuxieme reponse retourne None
        assert!(tracker.record_staff_response("t1").is_none());
    }

    #[test]
    fn response_without_creation_returns_none() {
        let tracker = SlaTracker::new();
        assert!(tracker.record_staff_response("unknown").is_none());
    }

    #[test]
    fn record_close_returns_times() {
        let tracker = SlaTracker::new();
        tracker.record_creation("t1");
        tracker.record_staff_response("t1");

        let result = tracker.record_close("t1");
        assert!(result.is_some());

        let (resolution, first_response) = result.unwrap();
        assert!(resolution.as_millis() >= 0);
        assert!(first_response.is_some());
    }

    #[test]
    fn close_without_response() {
        let tracker = SlaTracker::new();
        tracker.record_creation("t1");

        let result = tracker.record_close("t1");
        assert!(result.is_some());

        let (_, first_response) = result.unwrap();
        assert!(first_response.is_none());
    }

    #[test]
    fn close_unknown_ticket_returns_none() {
        let tracker = SlaTracker::new();
        assert!(tracker.record_close("unknown").is_none());
    }

    #[test]
    fn breached_tickets_empty_when_no_tickets() {
        let tracker = SlaTracker::new();
        assert!(tracker.breached_tickets(30).is_empty());
    }

    #[test]
    fn breached_tickets_excludes_responded() {
        let tracker = SlaTracker::new();
        tracker.record_creation("t1");
        tracker.record_staff_response("t1");

        // Meme avec un long delai, t1 a deja une reponse
        assert!(tracker.breached_tickets(0).is_empty());
    }

    #[test]
    fn escalation_tracking() {
        let tracker = SlaTracker::new();
        assert!(!tracker.is_escalated("t1"));
        tracker.mark_escalated("t1");
        assert!(tracker.is_escalated("t1"));
    }

    #[test]
    fn close_cleans_escalation() {
        let tracker = SlaTracker::new();
        tracker.record_creation("t1");
        tracker.mark_escalated("t1");
        tracker.record_close("t1");
        assert!(!tracker.is_escalated("t1"));
    }

    #[test]
    fn format_sla_seconds() {
        assert_eq!(format_sla_duration(Duration::from_secs(30)), "30s");
    }

    #[test]
    fn format_sla_minutes() {
        assert_eq!(format_sla_duration(Duration::from_secs(300)), "5min");
    }

    #[test]
    fn format_sla_hours() {
        assert_eq!(format_sla_duration(Duration::from_secs(3600)), "1h");
    }

    #[test]
    fn format_sla_mixed() {
        assert_eq!(format_sla_duration(Duration::from_secs(5400)), "1h30min");
    }
}
