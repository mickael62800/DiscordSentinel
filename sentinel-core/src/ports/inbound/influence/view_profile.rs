//! Use case : consulter le profil d'un citoyen.
//!
//! Applique la regle « stocke chiffre / expose narratif » : le proprietaire
//! voit ses chiffres exacts, les tiers ne voient que des paliers.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::errors::DomainError;

/// Vue d'un capital : palier toujours present, valeur exacte seulement pour soi.
#[derive(Debug, Clone)]
pub struct CapitalView {
    pub tier: &'static str,
    pub stars: &'static str,
    pub exact: Option<i64>,
}

/// Profil rendu, pret a etre formatte par l'adaptateur (bot / web).
#[derive(Debug, Clone)]
pub struct ProfileView {
    pub username: String,
    /// `true` si le viewer consulte son propre profil.
    pub is_self: bool,
    pub influence: CapitalView,
    pub money: CapitalView,
    /// Reputation : echelle qualitative dediee (Excellente..Desastreuse).
    pub reputation_tier: &'static str,
    pub reputation_exact: Option<i64>,
    pub information: CapitalView,
    pub network: CapitalView,
    pub joined_at: DateTime<Utc>,
    /// Reputation multi-dimensionnelle (chiffres exacts, seulement pour soi).
    pub reputation_dims:
        Option<crate::domain::entities::influence::reputation_dims::ReputationDims>,
}

#[async_trait]
pub trait ViewProfileUseCase: Send + Sync {
    /// Consulte (et enregistre a la volee) le citoyen `target_user_id`, du point
    /// de vue de `viewer_user_id`.
    async fn view(
        &self,
        guild_id: &str,
        viewer_user_id: &str,
        target_user_id: &str,
        target_username: &str,
    ) -> Result<ProfileView, DomainError>;
}
