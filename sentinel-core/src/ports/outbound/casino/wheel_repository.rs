use crate::domain::entities::casino::wheel::WheelSpin;
use crate::domain::entities::casino::wheel::WheelTopWinner;
use crate::domain::errors::DomainError;
use crate::ports::uow::DbTx;
use async_trait::async_trait;

#[async_trait]
pub trait WheelRepository: Send + Sync {
    async fn has_claimed_today(&self, guild_id: &str, user_id: &str) -> Result<bool, DomainError>;

    async fn log_spin_in_tx(&self, tx: &mut dyn DbTx, spin: &WheelSpin) -> Result<(), DomainError>;

    /// Marque le tirage du jour comme claim (insere dans wheel_daily_claims).
    /// ON CONFLICT DO NOTHING. Retourne `true` si la row a ete inseree (premier
    /// tirage du jour), `false` si elle existait deja. Sert de claim atomique :
    /// seule la PREMIERE tx concurrente obtient `true` et peut payer le spin.
    async fn mark_claimed_in_tx(
        &self,
        tx: &mut dyn DbTx,
        guild_id: &str,
        user_id: &str,
    ) -> Result<bool, DomainError>;

    async fn recent_spins(&self, guild_id: &str, limit: i64)
        -> Result<Vec<WheelSpin>, DomainError>;

    async fn top_winners(
        &self,
        guild_id: &str,
        days: i64,
        limit: i64,
    ) -> Result<Vec<WheelTopWinner>, DomainError>;
}
