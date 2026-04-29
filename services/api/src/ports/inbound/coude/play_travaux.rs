//! Use case `/travaux` cote API (Phase 2 #2 audit).
//!
//! Pipeline :
//!  1. Verifier que le joueur est en prison (sinon Forbidden).
//!  2. Verifier le cooldown 2h (`travaux_prison`).
//!  3. Tirer une tache + outcome (succes 50%, montant 50-100c).
//!  4. Si succes : credit wallet + add_xp.
//!  5. Toujours : poser le cooldown 2h, retourner le verdict.

use async_trait::async_trait;

use crate::domain::errors::DomainError;
use crate::domain::entities::system::discord_ids::UserId;
use crate::domain::entities::system::discord_ids::GuildId;

#[derive(Debug, Clone)]
pub struct PlayTravauxCommand {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub username: String,
}

#[derive(Debug, Clone)]
pub struct TravauxResolution {
    pub task_key: &'static str,
    pub task_label: &'static str,
    pub task_description: &'static str,
    pub success: bool,
    pub flavor: &'static str,
    /// Coins credites si succes, sinon 0.
    pub coins_gain: i64,
    /// XP credite (constant 5 par tache, succes ou echec — voir `TRAVAUX_XP_PER_TASK`).
    pub xp_gain: i64,
}

#[async_trait]
pub trait PlayTravauxUseCase: Send + Sync {
    /// Errors :
    /// - `Forbidden` si le joueur n'est pas en prison.
    /// - `RateLimited` si le cooldown 2h est encore actif.
    /// - `Internal` sur erreur DB / wallet.
    async fn play(&self, cmd: PlayTravauxCommand) -> Result<TravauxResolution, DomainError>;
}
