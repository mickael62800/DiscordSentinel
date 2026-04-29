//! Use case dedie a l'expiration batch des combats pending Coup de Coude.
//!
//! Phase 4 refacto : appele par coude-worker via gRPC. Gere :
//!   - UPDATE status='expired' sur les combats depasses
//!   - Penalite defenseur (20% mise debitee, total_lost +=)
//!   - Increment cowardice_count defenseur
//!   - Refund des paris sur le combat

use async_trait::async_trait;

use crate::domain::errors::DomainError;
use crate::domain::entities::system::discord_ids::ChannelId;
use crate::domain::entities::system::discord_ids::GuildId;

/// Info minimale pour chaque combat expire (utilisee pour logging + eventuel
/// post Discord a decider cote worker).
#[derive(Debug, Clone)]
pub struct ExpiredCombatOutput {
    pub combat_id: String,
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub defender_id: String,
    pub defender_name: String,
    pub penalty: i64,
}

#[async_trait]
pub trait ExpireCombatsBatchUseCase: Send + Sync {
    async fn expire_batch(&self) -> Result<Vec<ExpiredCombatOutput>, DomainError>;
}
