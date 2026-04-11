use serde::{Deserialize, Serialize};
use std::fmt;

/// Priorites possibles d'un ticket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TicketPriority {
    Low,
    Medium,
    High,
    Urgent,
}

impl TicketPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Urgent => "urgent",
        }
    }

    /// Parse une priorite depuis une string. Retourne `None` si invalide.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            "urgent" => Some(Self::Urgent),
            _ => None,
        }
    }

    /// Liste des valeurs valides.
    pub const VALID_VALUES: &'static [&'static str] = &["low", "medium", "high", "urgent"];
}

impl fmt::Display for TicketPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_all_variants() {
        for s in TicketPriority::VALID_VALUES {
            let prio = TicketPriority::from_str(s).unwrap();
            assert_eq!(prio.as_str(), *s);
        }
    }

    #[test]
    fn from_str_invalid() {
        assert!(TicketPriority::from_str("critical").is_none());
        assert!(TicketPriority::from_str("").is_none());
    }

    #[test]
    fn ordering() {
        assert!(TicketPriority::Urgent > TicketPriority::High);
        assert!(TicketPriority::High > TicketPriority::Medium);
        assert!(TicketPriority::Medium > TicketPriority::Low);
    }

    #[test]
    fn serde_roundtrip() {
        let json = serde_json::to_string(&TicketPriority::Urgent).unwrap();
        assert_eq!(json, "\"urgent\"");
        let back: TicketPriority = serde_json::from_str(&json).unwrap();
        assert_eq!(back, TicketPriority::Urgent);
    }

    #[test]
    fn valid_values_count() {
        assert_eq!(TicketPriority::VALID_VALUES.len(), 4);
    }
}
