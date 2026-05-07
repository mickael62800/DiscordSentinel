use async_trait::async_trait;

use sentinel_core::domain::entities::community::guild_member::GuildMember;
use sentinel_core::domain::errors::DomainError;

#[async_trait]
pub trait MemberRepository: Send + Sync {
    async fn find_by_guild(&self, guild_id: &str) -> Result<Vec<GuildMember>, DomainError>;
    async fn find_one(&self, guild_id: &str, user_id: &str) -> Result<Option<GuildMember>, DomainError>;
    async fn upsert(&self, member: &GuildMember) -> Result<(), DomainError>;
    async fn upsert_many(&self, members: &[GuildMember]) -> Result<u64, DomainError>;
    async fn delete(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError>;
    async fn update_last_seen(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError>;
}
