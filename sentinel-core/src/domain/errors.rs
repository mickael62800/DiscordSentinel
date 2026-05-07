use thiserror::Error;

#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum DomainError {
    /// 404 — ressource introuvable. Le message est libre, ex:
    /// `format!("Regle {uuid}")`, `format!("Ticket {id}")`, etc.
    #[error("Ressource introuvable : {0}")]
    NotFound(String),

    /// 400 / 422 — donnees invalides (validation cote domain ou serde).
    #[error("Donnees invalides : {0}")]
    ValidationError(String),

    /// 409 — conflit (unique constraint, version stale, etc.).
    #[error("Conflit : {0}")]
    Conflict(String),

    /// 403 — acces refuse (RBAC, ownership, guild membership).
    #[error("Acces refuse : {0}")]
    Forbidden(String),

    /// 429 — rate limit depasse.
    #[error("Rate limited : {0}")]
    RateLimited(String),

    /// 504 — timeout sur appel externe (Discord API, ONNX inference, etc.).
    #[error("Timeout : {0}")]
    Timeout(String),

    /// 500 — erreur interne (sqlx, redis, runtime). Message technique pour debug.
    #[error("Erreur interne : {0}")]
    Internal(String),

    /// 501 — methode de port non implementee par cet adapter (mock partiel,
    /// implementation a venir). Remplace les anciens `unimplemented!()`.
    #[error("Non implemente : {0}")]
    NotImplemented(String),
}
