//! Use case du duel amical (cf. COUPE_AMELIORATIONS 4.5).
//!
//! Mode "training" : combat sans mise, sans consequence economique.
//! - Aucun coin transfere.
//! - Pas d'assurance, pas de prime, pas de filet de securite.
//! - +20 XP au gagnant, +5 au perdant (XP boost vs combat normal).
//! - Stats stockees dans `friendly_wins` / `friendly_losses` (separe).

use async_trait::async_trait;

use crate::domain::entities::system::discord_ids::GuildId;
use crate::domain::errors::DomainError;

#[derive(Debug, Clone)]
pub struct FriendlyDuelInput {
    pub guild_id: GuildId,
    pub attacker_id: String,
    pub attacker_name: String,
    pub defender_id: String,
    pub defender_name: String,
}

#[derive(Debug, Clone)]
pub struct FriendlyDuelOutput {
    pub winner_id: Option<String>,
    pub loser_id: Option<String>,
    pub draw: bool,
    pub total_rounds: i32,
    pub attacker_hp_final: i32,
    pub attacker_hp_max: i32,
    pub defender_hp_final: i32,
    pub defender_hp_max: i32,
    pub winner_xp: i64,
    pub loser_xp: i64,
}

#[async_trait]
pub trait ResolveFriendlyDuelUseCase: Send + Sync {
    async fn resolve(&self, input: FriendlyDuelInput) -> Result<FriendlyDuelOutput, DomainError>;
}
