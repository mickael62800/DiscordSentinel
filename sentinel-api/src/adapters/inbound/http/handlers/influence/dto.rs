//! DTOs HTTP du jeu Influence.

use chrono::{DateTime, Utc};
use serde::Serialize;

use sentinel_core::ports::inbound::influence::view_profile::{CapitalView, ProfileView};

#[derive(Debug, Serialize)]
pub struct CapitalViewDto {
    pub tier: String,
    pub stars: String,
    /// Valeur exacte, presente uniquement quand on consulte son propre profil.
    pub exact: Option<i64>,
}

impl From<CapitalView> for CapitalViewDto {
    fn from(v: CapitalView) -> Self {
        Self {
            tier: v.tier.to_string(),
            stars: v.stars.to_string(),
            exact: v.exact,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ProfileViewDto {
    pub username: String,
    pub is_self: bool,
    pub influence: CapitalViewDto,
    pub money: CapitalViewDto,
    pub reputation_tier: String,
    pub reputation_exact: Option<i64>,
    pub information: CapitalViewDto,
    pub network: CapitalViewDto,
    pub joined_at: DateTime<Utc>,
}

impl From<ProfileView> for ProfileViewDto {
    fn from(p: ProfileView) -> Self {
        Self {
            username: p.username,
            is_self: p.is_self,
            influence: p.influence.into(),
            money: p.money.into(),
            reputation_tier: p.reputation_tier.to_string(),
            reputation_exact: p.reputation_exact,
            information: p.information.into(),
            network: p.network.into(),
            joined_at: p.joined_at,
        }
    }
}
