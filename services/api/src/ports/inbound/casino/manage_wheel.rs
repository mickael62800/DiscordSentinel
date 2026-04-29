//! Use case Roue du Destin.

use async_trait::async_trait;

use crate::domain::entities::coude::taunt::TauntEvent;
use crate::domain::entities::casino::wheel::WheelCase;
use crate::domain::entities::casino::wheel::WheelSpin;
use crate::domain::entities::casino::wheel::WheelTopWinner;
use crate::domain::errors::DomainError;
use crate::domain::entities::system::discord_ids::UserId;

#[derive(Debug, Clone)]
pub struct WheelSpinCommand {
    pub guild_id: String,
    pub user_id: UserId,
    pub username: String,
}

#[derive(Debug, Clone)]
pub struct WheelSpinResult {
    pub spin: WheelSpin,
    pub case: WheelCase,
    pub balance_after: i64,
    /// True si la case decroche un effet "memorable" (jackpot/licorne/bombe).
    pub is_memorable: bool,
    pub triggered_taunts: Vec<TauntEvent>,
}

#[async_trait]
pub trait ManageWheelUseCase: Send + Sync {
    /// 1 spin par jour. Erreurs : ValidationError("deja reclame aujourd hui")
    async fn spin(&self, cmd: WheelSpinCommand) -> Result<WheelSpinResult, DomainError>;

    async fn recent_spins(&self, guild_id: &str, limit: i64) -> Result<Vec<WheelSpin>, DomainError>;

    async fn top_winners(
        &self,
        guild_id: &str,
        days: i64,
        limit: i64,
    ) -> Result<Vec<WheelTopWinner>, DomainError>;
}
