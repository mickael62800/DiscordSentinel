use crate::domain::errors::DomainError;
use async_trait::async_trait;
#[derive(Debug, Clone)] pub struct CoussinInsurance { pub is_scam: bool, pub expires_at: chrono::DateTime<chrono::Utc> }
#[async_trait]
pub trait CoussinInsuranceRepository: Send + Sync {
    async fn buy(&self, guild_id: &str, user_id: &str, is_scam: bool) -> Result<CoussinInsurance, DomainError>;
    async fn active(&self, guild_id: &str, user_id: &str) -> Result<Option<CoussinInsurance>, DomainError>;
}
