use crate::domain::errors::DomainError;
use async_trait::async_trait;
#[derive(Debug, Clone)] pub struct CoudeInsurance { pub is_scam: bool, pub expires_at: chrono::DateTime<chrono::Utc> }
#[async_trait]
pub trait CoudeInsuranceRepository: Send + Sync {
    async fn buy(&self, guild_id: &str, user_id: &str, is_scam: bool) -> Result<CoudeInsurance, DomainError>;
    async fn active(&self, guild_id: &str, user_id: &str) -> Result<Option<CoudeInsurance>, DomainError>;
}
