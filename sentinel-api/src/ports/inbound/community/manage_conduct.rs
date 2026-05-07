use async_trait::async_trait;

use sentinel_core::domain::entities::community::conduct::ConductConfig;
use sentinel_core::domain::entities::community::conduct::ConductPointsLog;
use sentinel_core::domain::entities::community::conduct::UserConductPoints;
use sentinel_core::domain::errors::DomainError;
use sentinel_core::domain::entities::system::discord_ids::UserId;
use sentinel_core::domain::entities::system::discord_ids::GuildId;

pub struct SaveConductConfigCommand {
    pub guild_id: GuildId,
    pub max_points: i32,
    pub regen_amount: i32,
    pub regen_interval: String,
    pub penalty_warn: i32,
    pub penalty_delete: i32,
    pub penalty_mute: i32,
    pub penalty_ban: i32,
}

pub struct DeductPointsCommand {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub username: String,
    pub action: String,
}

pub struct AddPointsCommand {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub amount: i32,
    pub reason: String,
}

#[async_trait]
pub trait ManageConductUseCase: Send + Sync {
    async fn get_config(&self, guild_id: &str) -> Result<ConductConfig, DomainError>;
    async fn save_config(&self, cmd: SaveConductConfigCommand) -> Result<ConductConfig, DomainError>;
    async fn get_points(&self, guild_id: &str, user_id: &str) -> Result<UserConductPoints, DomainError>;
    async fn deduct_points(&self, cmd: DeductPointsCommand) -> Result<UserConductPoints, DomainError>;
    async fn add_points(&self, cmd: AddPointsCommand) -> Result<UserConductPoints, DomainError>;
    async fn get_leaderboard(&self, guild_id: &str, limit: i64) -> Result<Vec<UserConductPoints>, DomainError>;
    async fn get_points_log(&self, guild_id: &str, user_id: &str, limit: i64) -> Result<Vec<ConductPointsLog>, DomainError>;
    #[allow(dead_code)]
    async fn run_regen(&self) -> Result<u64, DomainError>;

    /// Cree des propositions de ban (`infractions` action='ban') pour les
    /// users dont les points de conduite sont a 0 et qui n'ont pas encore
    /// de proposition de ban liee a la conduite. Idempotent (skip ceux
    /// deja proposes). Retourne le nombre de propositions creees.
    /// Default impl `Ok(0)` pour preserver les mocks existants.
    async fn sync_ban_proposals(&self) -> Result<u64, DomainError> {
        Ok(0)
    }

    /// Restitue les points de conduite associes a une action moderee
    /// (warn/mute/ban/delete) suite a son annulation. Lit la penalite
    /// depuis la config guild et l'ajoute via `add_points`. Clamp a
    /// max_points. Default impl no-op pour preserver les mocks.
    async fn restore_for_action(
        &self,
        _guild_id: &str,
        _user_id: &str,
        _action: &str,
    ) -> Result<(), DomainError> {
        Ok(())
    }

    /// Reset tous les points de conduite d'une guild a `max_points`.
    /// Appele lors d'un "Vider le journal" massif. Retourne le nombre de
    /// users impactes. Default impl `Ok(0)` pour preserver les mocks.
    async fn reset_all_points(&self, _guild_id: &str) -> Result<u64, DomainError> {
        Ok(0)
    }
}
