//! Port outbound pour les maledictions (cf. COUPE_AMELIORATIONS section 5.1).

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::{ActiveCurse, CurseKind};
use crate::domain::errors::DomainError;

#[async_trait]
pub trait CoudeCursesRepository: Send + Sync {
    /// Cree une nouvelle malediction. Retourne l UUID genere.
    /// Le contrainte unique partial assure qu une seule curse non levee
    /// peut exister par (guild, target) — l appelant doit verifier
    /// qu il n y en a pas deja une active avant d insert.
    async fn cast(
        &self,
        guild_id: &str,
        target_id: &str,
        source_id: &str,
        kind: CurseKind,
        duration_hours: i64,
    ) -> Result<Uuid, DomainError>;

    /// Retourne la malediction active sur cette cible (lifted_at NULL,
    /// expires_at > NOW). None si aucune.
    async fn get_active_for_target(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<Option<ActiveCurse>, DomainError>;

    /// Marque une malediction comme levee. Renvoie Conflict si deja levee
    /// ou inexistante.
    async fn lift(&self, id: Uuid, lifted_by: &str) -> Result<(), DomainError>;

    /// Liste les maledictions actives lancees par un joueur (utile pour
    /// stats / antifrais sur le profil).
    async fn list_active_by_source(
        &self,
        guild_id: &str,
        source_id: &str,
    ) -> Result<Vec<ActiveCurse>, DomainError>;
}
