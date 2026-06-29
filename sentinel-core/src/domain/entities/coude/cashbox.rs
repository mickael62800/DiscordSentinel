//! Entites pour la caisse communautaire Coup de Coude.
//!
//! Phase 9 : elimine la contraction de l'economie en collectant tous les
//! coins "perdus" (shop, assurances, penalites) dans une caisse par guild,
//! puis en les redistribuant aleatoirement chaque semaine aux joueurs
//! actifs.

use crate::domain::entities::system::discord_ids::GuildId;
use crate::domain::entities::system::discord_ids::UserId;
use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;

/// Etat de la caisse d'une guild.
#[derive(Debug, Clone)]
pub struct Cashbox {
    pub guild_id: GuildId,
    pub balance: i64,
    pub total_collected: i64,
    pub total_redistributed: i64,
    pub last_redistribution_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Une redistribution hebdomadaire complete (historique).
#[derive(Debug, Clone)]
pub struct CashboxRedistribution {
    pub id: Uuid,
    pub guild_id: GuildId,
    pub total_amount: i64,
    pub winners_count: i32,
    pub created_at: DateTime<Utc>,
}

/// Un gain individuel dans une redistribution.
#[derive(Debug, Clone)]
pub struct CashboxRedistributionEntry {
    pub id: Uuid,
    pub redistribution_id: Uuid,
    pub user_id: UserId,
    pub username: String,
    pub amount_won: i64,
    pub created_at: DateTime<Utc>,
}

/// Source du deposit dans la cashbox (pour tracing / audit).
/// Pas persiste directement — utilise par le service comme label de log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CashboxSource {
    ShopPurchase,
    InsurancePurchase,
    ProtectionPurchase,
    BoostPurchase,
    ClassChangeCost,
    ResetStatsCost,
    DonationTax,
    CowardicePenalty,
    BetCommission,
}

impl CashboxSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ShopPurchase => "shop_purchase",
            Self::InsurancePurchase => "insurance_purchase",
            Self::ProtectionPurchase => "protection_purchase",
            Self::BoostPurchase => "boost_purchase",
            Self::ClassChangeCost => "class_change",
            Self::ResetStatsCost => "reset_stats",
            Self::DonationTax => "donation_tax",
            Self::CowardicePenalty => "cowardice_penalty",
            Self::BetCommission => "bet_commission",
        }
    }
}

#[cfg(test)]
#[path = "tests/cashbox.rs"]
mod tests;
