use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum DomainError {
    // 404
    #[error("Règle introuvable : {0}")]
    RuleNotFound(Uuid),

    #[error("Infraction introuvable : {0}")]
    InfractionNotFound(Uuid),

    #[error("Ticket introuvable : {0}")]
    TicketNotFound(String),

    #[error("Ressource introuvable : {0}")]
    NotFound(String),

    // 400
    #[error("Règle invalide : {0}")]
    InvalidRule(String),

    // 422
    #[error("Données invalides : {0}")]
    ValidationError(String),

    // 409
    #[error("Conflit : {0}")]
    Conflict(String),

    // 504
    #[error("Timeout : {0}")]
    Timeout(String),

    // 500
    #[error("Erreur interne : {0}")]
    Internal(String),
}
