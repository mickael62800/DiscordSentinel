use crate::{domain::errors::DomainError, ports::outbound::coude_insurance_repository::CoudeInsurance};
use async_trait::async_trait;
#[async_trait] pub trait CoudeInsuranceUseCase: Send + Sync { async fn buy(&self, guild_id: &str, user_id: &str) -> Result<CoudeInsurance, DomainError>; async fn active(&self, guild_id: &str, user_id: &str) -> Result<Option<CoudeInsurance>, DomainError>; }
