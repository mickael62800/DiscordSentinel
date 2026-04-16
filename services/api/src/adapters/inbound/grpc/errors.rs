//! Mapping `DomainError` -> `tonic::Status`. Aligne sur les codes HTTP
//! deja utilises cote Axum (cf. `adapters/inbound/http/errors.rs`).

use tonic::{Code, Status};

use crate::domain::errors::DomainError;

pub fn domain_to_status(err: DomainError) -> Status {
    let (code, msg) = match &err {
        DomainError::RuleNotFound(_)
        | DomainError::InfractionNotFound(_)
        | DomainError::TicketNotFound(_)
        | DomainError::NotFound(_) => (Code::NotFound, err.to_string()),

        DomainError::InvalidRule(_) => (Code::InvalidArgument, err.to_string()),
        DomainError::ValidationError(_) => (Code::InvalidArgument, err.to_string()),
        DomainError::Conflict(_) => (Code::AlreadyExists, err.to_string()),
        DomainError::Forbidden(_) => (Code::PermissionDenied, err.to_string()),
        DomainError::RateLimited(_) => (Code::ResourceExhausted, err.to_string()),
        DomainError::Timeout(_) => (Code::DeadlineExceeded, err.to_string()),
        DomainError::Internal(_) => (Code::Internal, err.to_string()),
    };
    Status::new(code, msg)
}

/// Convertit une erreur sqlx en `Status::Internal`. Utilise par les handlers
/// gRPC qui font du SQL direct (community, etc.) pour eviter les
/// `.map_err(|e| Status::internal(format!("...: {e}")))` inline repetes.
pub fn sqlx_to_status(context: &str) -> impl Fn(sqlx::Error) -> Status + '_ {
    move |e| Status::internal(format!("{context}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn rule_not_found_maps_to_not_found() {
        let s = domain_to_status(DomainError::RuleNotFound(Uuid::nil()));
        assert_eq!(s.code(), Code::NotFound);
    }

    #[test]
    fn infraction_not_found_maps_to_not_found() {
        let s = domain_to_status(DomainError::InfractionNotFound(Uuid::nil()));
        assert_eq!(s.code(), Code::NotFound);
    }

    #[test]
    fn ticket_not_found_maps_to_not_found() {
        let s = domain_to_status(DomainError::TicketNotFound("t1".into()));
        assert_eq!(s.code(), Code::NotFound);
    }

    #[test]
    fn generic_not_found_maps_to_not_found() {
        let s = domain_to_status(DomainError::NotFound("absent".into()));
        assert_eq!(s.code(), Code::NotFound);
        assert!(s.message().contains("absent"));
    }

    #[test]
    fn invalid_rule_maps_to_invalid_argument() {
        let s = domain_to_status(DomainError::InvalidRule("bad regex".into()));
        assert_eq!(s.code(), Code::InvalidArgument);
    }

    #[test]
    fn validation_error_maps_to_invalid_argument() {
        let s = domain_to_status(DomainError::ValidationError("champ invalide".into()));
        assert_eq!(s.code(), Code::InvalidArgument);
        assert!(s.message().contains("invalide"));
    }

    #[test]
    fn conflict_maps_to_already_exists() {
        let s = domain_to_status(DomainError::Conflict("deja la".into()));
        assert_eq!(s.code(), Code::AlreadyExists);
    }

    #[test]
    fn forbidden_maps_to_permission_denied() {
        let s = domain_to_status(DomainError::Forbidden("interdit".into()));
        assert_eq!(s.code(), Code::PermissionDenied);
    }

    #[test]
    fn rate_limited_maps_to_resource_exhausted() {
        let s = domain_to_status(DomainError::RateLimited("trop vite".into()));
        assert_eq!(s.code(), Code::ResourceExhausted);
    }

    #[test]
    fn timeout_maps_to_deadline_exceeded() {
        let s = domain_to_status(DomainError::Timeout("trop long".into()));
        assert_eq!(s.code(), Code::DeadlineExceeded);
    }

    #[test]
    fn internal_maps_to_internal() {
        let s = domain_to_status(DomainError::Internal("oops".into()));
        assert_eq!(s.code(), Code::Internal);
    }
}
