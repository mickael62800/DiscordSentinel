//! Port outbound pour les maledictions (cf. COUPE_AMELIORATIONS section 5.1).

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::coude::curse::ActiveCurse;
use crate::domain::entities::coude::curse::CurseKind;
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

    /// Variante de `cast` avec compteur d utilisations initial (cf.
    /// Empoisonner). Default impl delegue a `cast` (ignore uses) pour
    /// preserver les mocks existants.
    async fn cast_with_uses(
        &self,
        guild_id: &str,
        target_id: &str,
        source_id: &str,
        kind: CurseKind,
        duration_hours: i64,
        _uses_remaining: Option<i32>,
    ) -> Result<Uuid, DomainError> {
        self.cast(guild_id, target_id, source_id, kind, duration_hours)
            .await
    }

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

    /// Decremente `uses_remaining` d une curse. Si le compteur tombe a 0,
    /// la curse est automatiquement marquee comme levee. No-op si la curse
    /// est deja inactive ou n a pas de compteur. Retourne le nouveau
    /// uses_remaining (None = curse pas a compteur ou consumed/lifted).
    /// Default impl Ok(None) pour ne pas casser les mocks.
    async fn consume_one_use(&self, _id: Uuid) -> Result<Option<i32>, DomainError> {
        Ok(None)
    }

    /// Liste les maledictions actives lancees par un joueur (utile pour
    /// stats / antifrais sur le profil).
    async fn list_active_by_source(
        &self,
        guild_id: &str,
        source_id: &str,
    ) -> Result<Vec<ActiveCurse>, DomainError>;
}
