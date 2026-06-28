//! Helpers de validation partages par les services applicatifs.
//!
//! Evite de repeter le garde `if x.trim().is_empty() { return Err(...) }`
//! dans chaque methode de service.

use crate::domain::errors::DomainError;

/// Valide qu'un `guild_id` n'est pas vide. Message coherent partout.
pub fn validate_guild_id(guild_id: &str) -> Result<(), DomainError> {
    validate_non_empty(guild_id, "guild_id")
}

/// Valide qu'un champ texte n'est pas vide (apres trim).
pub fn validate_non_empty(value: &str, field: &str) -> Result<(), DomainError> {
    if value.trim().is_empty() {
        Err(DomainError::ValidationError(format!("{field} requis")))
    } else {
        Ok(())
    }
}
