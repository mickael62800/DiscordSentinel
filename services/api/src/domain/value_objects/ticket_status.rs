use serde::{Deserialize, Serialize};
use std::fmt;

/// Statuts possibles d'un ticket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TicketStatus {
    Open,
    Pending,
    Closed,
}

impl TicketStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Pending => "pending",
            Self::Closed => "closed",
        }
    }

    /// Parse un statut depuis une string. Retourne `None` si invalide.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "open" => Some(Self::Open),
            "pending" => Some(Self::Pending),
            "closed" => Some(Self::Closed),
            _ => None,
        }
    }

    /// Liste des valeurs valides (pour les messages d'erreur).
    pub const VALID_VALUES: &'static [&'static str] = &["open", "pending", "closed"];
}

impl fmt::Display for TicketStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_all_variants() {
        for s in TicketStatus::VALID_VALUES {
            let status = TicketStatus::from_str(s).unwrap();
            assert_eq!(status.as_str(), *s);
        }
    }

    #[test]
    fn from_str_invalid() {
        assert!(TicketStatus::from_str("invalid").is_none());
        assert!(TicketStatus::from_str("").is_none());
        assert!(TicketStatus::from_str("OPEN").is_none());
    }

    #[test]
    fn serde_roundtrip() {
        let json = serde_json::to_string(&TicketStatus::Open).unwrap();
        assert_eq!(json, "\"open\"");
        let back: TicketStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, TicketStatus::Open);
    }

    #[test]
    fn display_trait() {
        assert_eq!(format!("{}", TicketStatus::Closed), "closed");
    }

    #[test]
    fn valid_values_count() {
        assert_eq!(TicketStatus::VALID_VALUES.len(), 3);
    }
}
