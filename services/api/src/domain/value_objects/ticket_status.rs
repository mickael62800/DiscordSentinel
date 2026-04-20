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
    #[allow(clippy::should_implement_trait)]
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
#[path = "tests/ticket_status.rs"]
mod tests;
