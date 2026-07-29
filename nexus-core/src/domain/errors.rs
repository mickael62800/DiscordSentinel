//! Erreurs du domaine Nexus — volontairement minimales pour la v1.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    /// Regle metier violee (ex: daily deja claim). Message affichable au joueur.
    #[error("{0}")]
    Validation(String),

    /// Ressource introuvable.
    #[error("introuvable: {0}")]
    NotFound(String),

    /// Erreur d'infrastructure (DB, reseau) remontee par un adapter.
    #[error("erreur infrastructure: {0}")]
    Infrastructure(String),

    // ── Variants alignes sur sentinel-core (portage game-portal) ──
    /// Donnees invalides (alias sentinel de `Validation`).
    #[error("Donnees invalides : {0}")]
    ValidationError(String),

    /// Conflit d'etat (ex: nom de serveur deja pris).
    #[error("Conflit : {0}")]
    Conflict(String),

    /// Acces refuse.
    #[error("Acces refuse : {0}")]
    Forbidden(String),

    /// Rate limited.
    #[error("Rate limited : {0}")]
    RateLimited(String),

    /// Timeout d'une operation.
    #[error("Timeout : {0}")]
    Timeout(String),

    /// Erreur interne (infra, docker, redis…).
    #[error("Erreur interne : {0}")]
    Internal(String),

    /// Fonctionnalite non implementee.
    #[error("Non implemente : {0}")]
    NotImplemented(String),
}
