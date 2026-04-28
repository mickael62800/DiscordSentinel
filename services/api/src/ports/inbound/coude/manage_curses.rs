//! Use case des maledictions (cf. COUPE_AMELIORATIONS section 5.1).

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::coude::curse::ActiveCurse;
use crate::domain::entities::coude::curse::CurseKind;
use crate::domain::errors::DomainError;

/// Resultat d un cast reussi : ce qu il faut afficher cote bot.
#[derive(Debug, Clone)]
pub struct CastedCurse {
    pub id: Uuid,
    pub kind: CurseKind,
    pub cost_paid: i64,
}

#[async_trait]
pub trait ManageCoudeCursesUseCase: Send + Sync {
    /// Pose une malediction sur `target_id` au nom de `source_id`. Si
    /// `kind` est `None`, une malediction est tiree au sort.
    ///
    /// Effectue : valide source != target, verifie qu il n y a pas deja
    /// une malediction active sur la cible, debit `CURSE_COST_COINS` du
    /// wallet de l auteur, insere la curse.
    async fn cast(
        &self,
        guild_id: &str,
        source_id: &str,
        source_username: &str,
        target_id: &str,
        kind: Option<CurseKind>,
    ) -> Result<CastedCurse, DomainError>;

    /// Retourne la malediction active sur cette cible, ou None.
    async fn get_active(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<Option<ActiveCurse>, DomainError>;

    /// La cible leve sa propre malediction. Cout : `lift_cost`, transfere
    /// integralement a l auteur initial. Retourne la curse mise a jour.
    async fn lift_own(
        &self,
        guild_id: &str,
        target_id: &str,
        target_username: &str,
    ) -> Result<ActiveCurse, DomainError>;
}
