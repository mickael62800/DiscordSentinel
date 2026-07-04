//! Use case : consultation detaillee des capitaux + conversions (Phase 2).

use async_trait::async_trait;

use crate::domain::entities::influence::capital::Capital;
use crate::domain::entities::influence::conversion::ConversionKind;
use crate::domain::entities::influence::movement::CapitalMovement;
use crate::domain::errors::DomainError;

/// Un capital et sa valeur exacte.
#[derive(Debug, Clone)]
pub struct CapitalLine {
    pub capital: Capital,
    pub value: i64,
}

/// Vue detaillee des capitaux d'un citoyen (chiffres exacts + historique).
#[derive(Debug, Clone)]
pub struct CapitalOverview {
    pub lines: Vec<CapitalLine>,
    pub movements: Vec<CapitalMovement>,
}

/// Resultat d'une conversion effectuee.
#[derive(Debug, Clone)]
pub struct ConversionOutcome {
    pub kind: ConversionKind,
    pub spent: i64,
    pub gained: i64,
    pub new_source: i64,
    pub new_target: i64,
}

#[async_trait]
pub trait ManageCapitalUseCase: Send + Sync {
    /// Capitaux exacts + derniers mouvements (reserve au proprietaire).
    async fn view(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
    ) -> Result<CapitalOverview, DomainError>;

    /// Convertit `budget` unites de capital source vers le capital cible.
    async fn convert(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        kind: ConversionKind,
        budget: i64,
    ) -> Result<ConversionOutcome, DomainError>;
}
