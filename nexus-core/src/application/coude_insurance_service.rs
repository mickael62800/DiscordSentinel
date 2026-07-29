use std::sync::Arc; use async_trait::async_trait; use rand::Rng;
use crate::{domain::errors::DomainError, ports::{inbound::coude_insurance::CoudeInsuranceUseCase, outbound::coude_insurance_repository::{CoudeInsurance, CoudeInsuranceRepository}}};
pub struct CoudeInsuranceService { repo: Arc<dyn CoudeInsuranceRepository> }
impl CoudeInsuranceService { pub fn new(repo: Arc<dyn CoudeInsuranceRepository>) -> Self { Self { repo } } }
#[async_trait] impl CoudeInsuranceUseCase for CoudeInsuranceService { async fn buy(&self, guild_id: &str, user_id: &str) -> Result<CoudeInsurance, DomainError> { let is_scam = rand::thread_rng().gen_range(0..100) < 5; self.repo.buy(guild_id, user_id, is_scam).await } async fn active(&self, guild_id: &str, user_id: &str) -> Result<Option<CoudeInsurance>, DomainError> { self.repo.active(guild_id, user_id).await } }
