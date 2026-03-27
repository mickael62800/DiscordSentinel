use async_trait::async_trait;

use crate::domain::entities::{ConductConfig, ConductPointsLog, UserConductPoints};
use crate::domain::errors::DomainError;

pub struct SaveConductConfigCommand {
    pub guild_id: String,
    pub max_points: i32,
    pub regen_amount: i32,
    pub regen_interval: String,
    pub penalty_warn: i32,
    pub penalty_delete: i32,
    pub penalty_mute: i32,
    pub penalty_ban: i32,
}

pub struct DeductPointsCommand {
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub action: String,
}

pub struct AddPointsCommand {
    pub guild_id: String,
    pub user_id: String,
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
    async fn run_regen(&self) -> Result<u64, DomainError>;
}
