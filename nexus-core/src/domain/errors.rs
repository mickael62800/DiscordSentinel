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
}
