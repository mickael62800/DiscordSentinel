use crate::{
    domain::errors::DomainError,
    ports::outbound::coude_repository::{CoudeCombat, CoudeProfile},
};
use async_trait::async_trait;
#[async_trait]
pub trait CoudeProfileUseCase: Send + Sync {
    async fn profile(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
    ) -> Result<CoudeProfile, DomainError>;
    async fn choose_class(&self, guild_id: &str, user_id: &str, username: &str, class: &str) -> Result<CoudeProfile, DomainError>;
    async fn train(&self, guild_id: &str, user_id: &str, username: &str, stat: &str) -> Result<CoudeProfile, DomainError>;
}

#[async_trait]
pub trait CoudeCombatUseCase: Send + Sync {
    async fn challenge(
        &self,
        guild_id: &str,
        channel_id: &str,
        attacker_id: &str,
        attacker_name: &str,
        defender_id: &str,
        defender_name: &str,
        mise: i64,
    ) -> Result<CoudeCombat, DomainError>;
    async fn accept(&self, id: uuid::Uuid, defender_id: &str) -> Result<bool, DomainError>;
    async fn refuse(&self, id: uuid::Uuid, defender_id: &str) -> Result<bool, DomainError>;
    async fn resolve(&self, id: uuid::Uuid) -> Result<bool, DomainError>;
}
