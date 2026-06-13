use sentinel_core::domain::errors::DomainError;

// Wrappers Pg* pour les enums du domaine (sqlx::Type vit ici, pas dans core).
pub mod types;
pub mod uow;

// Bounded contexts (mirror de ports/outbound/).
pub mod audit;
pub mod casino;
pub mod community;
pub mod coude;
pub mod game;
pub mod moderation;
pub mod system;
pub mod tamagotchi;

/// Helper centralise : convertit une erreur sqlx en DomainError::Internal.
/// Utilise par tous les repositories Postgres.
pub(crate) fn pg_err(e: sqlx::Error) -> DomainError {
    DomainError::Internal(e.to_string())
}

/// Variante avec contexte (nom de table / repo). Le contexte apparait
/// dans le message d'erreur pour aider au debug : `"coude_safety_nets pg: ..."`.
/// Remplace les ~14 fonctions `pg_err` locales redefinies dans chaque repo.
pub(crate) fn pg_err_ctx(ctx: &'static str, e: sqlx::Error) -> DomainError {
    DomainError::Internal(format!("{ctx} pg: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pg_err_wraps_sqlx_row_not_found_into_internal() {
        let err = pg_err(sqlx::Error::RowNotFound);
        match err {
            DomainError::Internal(msg) => {
                assert!(!msg.is_empty());
            }
            other => panic!("expected Internal, got {other:?}"),
        }
    }

    #[test]
    fn pg_err_wraps_protocol_error() {
        let err = pg_err(sqlx::Error::Protocol("connexion fermee".into()));
        match err {
            DomainError::Internal(msg) => assert!(msg.contains("connexion fermee")),
            other => panic!("expected Internal, got {other:?}"),
        }
    }

    #[test]
    fn pg_err_message_matches_display() {
        let source_err = sqlx::Error::PoolClosed;
        let display = source_err.to_string();
        let wrapped = pg_err(source_err);
        match wrapped {
            DomainError::Internal(msg) => assert_eq!(msg, display),
            other => panic!("expected Internal, got {other:?}"),
        }
    }
}
