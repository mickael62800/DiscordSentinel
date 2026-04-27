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

    // 403
    #[error("Accès refusé : {0}")]
    Forbidden(String),

    // 429
    #[error("Rate limited : {0}")]
    RateLimited(String),

    // 504
    #[error("Timeout : {0}")]
    Timeout(String),

    // 500
    #[error("Erreur interne : {0}")]
    Internal(String),

    // 501 — methode de port non implementee par cet adapter (mock partiel,
    // implementation a venir). Remplace les anciens `unimplemented!()` qui
    // paniquaient en runtime.
    #[error("Non implemente : {0}")]
    NotImplemented(String),
}
