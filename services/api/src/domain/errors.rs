use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Règle introuvable : {0}")]
    RuleNotFound(Uuid),

    #[error("Règle invalide : {0}")]
    InvalidRule(String),

    #[error("Infraction introuvable : {0}")]
    InfractionNotFound(Uuid),

    #[error("Erreur interne : {0}")]
    Internal(String),
}
