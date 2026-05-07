use async_trait::async_trait;
use sentinel_core::ports::uow::DbTx;
use sentinel_core::domain::entities::casino::wheel::WheelSpin;
use sentinel_core::domain::entities::casino::wheel::WheelTopWinner;
use sentinel_core::domain::errors::DomainError;

#[async_trait]
pub trait WheelRepository: Send + Sync {
    async fn has_claimed_today(&self, guild_id: &str, user_id: &str) -> Result<bool, DomainError>;

    async fn log_spin_in_tx(
        &self,
        tx: &mut dyn DbTx,
        spin: &WheelSpin,
    ) -> Result<(), DomainError>;

    async fn mark_claimed_in_tx(
        &self,
        tx: &mut dyn DbTx,
        guild_id: &str,
        user_id: &str,
    ) -> Result<(), DomainError>;

    async fn recent_spins(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<WheelSpin>, DomainError>;

    async fn top_winners(
        &self,
        guild_id: &str,
        days: i64,
        limit: i64,
    ) -> Result<Vec<WheelTopWinner>, DomainError>;
}
