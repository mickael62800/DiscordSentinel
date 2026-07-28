//! Port inbound : use case Roue du Destin.

use async_trait::async_trait;

use crate::domain::entities::wheel::WheelSpin;
use crate::domain::errors::DomainError;

#[derive(Debug, Clone)]
pub struct PlayWheelCommand {
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
}

#[derive(Debug, Clone)]
pub struct PlayWheelResult {
    pub spin: WheelSpin,
    pub balance_after: i64,
    /// True si la case est "memorable" (jackpot/licorne/bombe).
    pub is_memorable: bool,
}

#[async_trait]
pub trait PlayWheelUseCase: Send + Sync {
    /// 1 spin par joueur par jour (claim quotidien).
    /// Erreur `Validation` si le joueur a deja tire aujourd'hui.
    async fn spin(&self, cmd: PlayWheelCommand) -> Result<PlayWheelResult, DomainError>;
}
