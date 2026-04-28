use async_trait::async_trait;

use crate::domain::entities::community::conduct::ConductConfig;
use crate::domain::entities::community::conduct::ConductPointsLog;
use crate::domain::entities::community::conduct::UserConductPoints;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait ConductRepository: Send + Sync {
    // Config
    async fn get_config(&self, guild_id: &str) -> Result<Option<ConductConfig>, DomainError>;
    async fn save_config(&self, config: &ConductConfig) -> Result<(), DomainError>;

    // Points
    async fn get_points(&self, guild_id: &str, user_id: &str) -> Result<Option<UserConductPoints>, DomainError>;
    async fn save_points(&self, points: &UserConductPoints) -> Result<(), DomainError>;
    async fn update_points(&self, guild_id: &str, user_id: &str, new_points: i32) -> Result<(), DomainError>;
    async fn get_leaderboard(&self, guild_id: &str, limit: i64) -> Result<Vec<UserConductPoints>, DomainError>;
    #[allow(dead_code)]
    async fn find_users_needing_regen(&self, interval: &str) -> Result<Vec<UserConductPoints>, DomainError>;
    #[allow(dead_code)]
    async fn update_regen_timestamp(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError>;
    #[allow(dead_code)]
    async fn delete_points(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError>;

    // Log
    async fn save_log(&self, log: &ConductPointsLog) -> Result<(), DomainError>;
    async fn get_log(&self, guild_id: &str, user_id: &str, limit: i64) -> Result<Vec<ConductPointsLog>, DomainError>;

    /// Liste les users dont `points <= 0` qui n'ont pas deja une infraction
    /// `action='ban'` avec une `reason` LIKE `reason_prefix%`. Utilise par
    /// `ManageConductUseCase::sync_ban_proposals` pour creer une proposition
    /// de ban quand les points tombent a zero. Default impl `Ok(vec![])`
    /// pour preserver les mocks existants.
    async fn find_zero_points_users_without_ban_proposal(
        &self,
        _reason_prefix: &str,
    ) -> Result<Vec<UserConductPoints>, DomainError> {
        Ok(Vec::new())
    }
}
